//! Serialization of time tag objects, supporting `.tags.zst` and `.tsv`

use crate::Tag;
use anyhow::Result;
use capnp::{message, serialize, serialize_packed};
use std::io::Write;
use tagger_capnp::tags_capnp::tags;
use zstd::stream;

/// Serialize to .tags.zst format: zstd-compressed Cap'n Proto tags
///
/// Like many compressors, `zstd` is "linear" under concatenation, in that
/// `zstd(m1 + m2) == zstd(m1) + zstd(m2)` (ignoring that the compressed bytes
/// will actually differ as they cannot be compressed across the boundary). So
/// while we repeatedly write compressed messages when saving data, they can be
/// uncompressed and read as if they were a single one.
pub fn tags(wtr: &mut impl Write, tags: &[Tag]) -> Result<()> {
    let mut zwtr = stream::write::Encoder::new(wtr, 0)?;
    tags_uncompressed(&mut zwtr, tags)?;
    zwtr.finish()?;
    Ok(())
}

/// Serialize to .tags: uncompressed, unpacked Cap'n Proto tags
pub fn tags_uncompressed(wtr: &mut impl Write, tags: &[Tag]) -> Result<()> {
    let message = newmsg(&tags);
    serialize::write_message(wtr, &message)?;
    Ok(())
}

/// Vary packing and compression level for benchmarks
pub fn tags_bench(wtr: &mut impl Write, tags: &[Tag], pack: bool, level: i32) -> Result<()> {
    let message = newmsg(&tags);
    let mut zwtr = stream::write::Encoder::new(wtr, level)?;

    if pack {
        serialize_packed::write_message(&mut zwtr, &message)?;
    } else {
        serialize::write_message(&mut zwtr, &message)?;
    }
    zwtr.finish()?;
    Ok(())
}

/// Serialize tags to tab-separated values (channel, time).
pub fn tsv(wtr: &mut csv::Writer<impl Write>, tags: &[Tag]) -> Result<()> {
    for tag in tags.iter() {
        wtr.write_record(&[tag.channel.to_string(), tag.time.to_string()])?;
    }
    Ok(())
}

/// Allocate and build a new message; return a pointer to it
#[inline(always)]
pub fn newmsg(tags: &[Tag]) -> message::Builder<message::HeapAllocator> {
    let mut message = message::Builder::new_default();
    fillmsg(&mut message, tags);
    return message;
}

/// Build tags message from an existing allocator
pub fn fillmsg<'a, A>(
    message: &'a mut message::Builder<A>,
    tags: &[Tag],
) where A: message::Allocator {
    let message_builder = message.init_root::<tags::Builder>();

    // Cap'n Proto `struct_list`s are limited to a max of 2^29 - 1 words of data,
    // or a hair under 4 GiB. The first word in the encoding is a "tag word" pointer
    // describing the individual list elements. Since each Tag is two words, we can
    // store 2^28 - 1 Tags per List. But, by using a List(List(Tag)), we can overcome
    // this size limitation. `list_list` of Tag tops out at ~ 2 EiB, while the
    // maximum Cap'n Proto filesize overall is ~ 16 EiB.
    let n = (1 << 28) - 1;
    let q: u32 = (tags.len() / n) as u32;
    let r: u32 = (tags.len() % n) as u32;
    let lists: u32 = if r > 0 { q + 1 } else { q };

    let mut tags_builder= message_builder.init_tags(lists);
    for (i, chunk) in tags.chunks(n).enumerate() {
        let mut chunk_builder = tags_builder.reborrow().init(i as u32, chunk.len() as u32);
        for (j, tag) in chunk.iter().enumerate() {
            let mut tag_builder = chunk_builder.reborrow().get(j as u32);
            tag_builder.set_time(tag.time);
            tag_builder.set_channel(tag.channel)
        }
    }
}
