//! Serialization of time tag objects, supporting `.tags` and `.tsv`

use crate::Tag;
use anyhow::Result;
use capnp::{message, serialize};
use std::io::Write;
use tagger_capnp::tags_capnp::tags;
use zstd::stream;

/// Serialize to .tags format: zstd-compressed Cap'n Proto tags
///
/// Like many compressors, `zstd`'s API is linear under concatenation, in that
/// `zstd(m1 + m2) == zstd(m1) + zstd(m2)` (ignoring that the compressed bytes
/// will actually differ). So while we write repeated compressed messages when
/// saving data, it suffices to compress them individually.
pub fn tags(wtr: &mut impl Write, tags: &[Tag]) -> Result<()> {
    let mut zwtr = stream::write::Encoder::new(wtr, 0)?;
    tags_uncompressed(&mut zwtr, tags)?;
    zwtr.finish()?;
    Ok(())
}

/// Serialize to uncompressed, unpacked Cap'n Proto tags
///
/// As an implementation detail, tags are serialized as a `List(List(Tag))`,
/// as there is a 4 GiB limit per `struct_list` (like `List(Tag)`).
pub fn tags_uncompressed(wtr: &mut impl Write, tags: &[Tag]) -> Result<()> {
    let message = newmsg(&tags);
    serialize::write_message(wtr, &message)?;
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
pub fn newmsg(tags: &[Tag]) -> Box<message::Builder<message::HeapAllocator>> {
    let mut message = Box::new(capnp::message::Builder::new_default());
    fillmsg(&mut message, tags);
    return message;
}

/// Like msg::new, except you already have a Box<message> you want to use for serialization
pub fn fillmsg(message: &mut Box<message::Builder<message::HeapAllocator>>, tags: &[Tag]) {
    let message_builder = message.init_root::<tags::Builder>();

    // Cap'n Proto `struct_list`s are limited to a max of 2^29 - 1 words of data,
    // or a hair under 4 GiB. The first word in the encoding is a "tag word" pointer
    // describing the individual list elements. Since each Tag is two words, we can
    // store 2^27 Tags per List. But, by using a List(List(Tag)), we can overcome
    // this size limitation. `list_list` of Tag tops out at ~ 2 EiB, while the
    // maximum Cap'n Proto filesize overall is ~ 16 EiB.
    let exp: u32 = 27;
    let full_lists: u32 = (tags.len() / 2usize.pow(exp)) as u32;
    let remainder: u32 = (tags.len() % 2usize.pow(exp)) as u32;
    let lists: u32 = if remainder > 0 {
        full_lists + 1
    } else {
        full_lists
    };

    let mut tags_builder = message_builder.init_tags(lists);
    for (i, chunk) in tags.chunks(2usize.pow(exp)).enumerate() {
        let mut chunk_builder = tags_builder.reborrow().init(i as u32, chunk.len() as u32);
        for (j, tag) in chunk.iter().enumerate() {
            let mut tag_builder = chunk_builder.reborrow().get(j as u32);
            tag_builder.set_time(tag.time);
            tag_builder.set_channel(tag.channel)
        }
    }
}
