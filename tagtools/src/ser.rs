//! Serialization of time tag objects, supporting `.tags` and `.tsv`

use crate::Tag;
use anyhow::Result;
use capnp::serialize;
use std::io::Write;
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
    let message = msg::new(&tags);
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

/// Cap'n Proto message builders
pub mod msg {
    use crate::Tag;
    use crate::tags_capnp::tags;
    use capnp::message;
    use std::boxed::Box;

    /// Allocate and build a new message; return a pointer to it
    #[inline(always)]
    pub fn new(tags: &[Tag]) -> Box<message::Builder<message::HeapAllocator>> {
        let mut message = Box::new(capnp::message::Builder::new_default());
        {
            let message_builder = message.init_root::<tags::Builder>();

            // Cap'n Proto lists are limited to a max of 2^29 elements, and
            // additionally for struct lists, to a max of 2^29 words of data.
            // Since each Tag is two words, we can store 2^28 Tags per List.
            let full_lists: u32 = (tags.len() / 2usize.pow(28)) as u32;
            let remainder: u32 = (tags.len() % 2usize.pow(28)) as u32;

            let mut tags_builder = message_builder.init_tags(
                if remainder > 0 { full_lists + 1 } else { full_lists }
            );
            for (i, chunk) in tags.chunks(2usize.pow(29)).enumerate() {
                let mut chunk_builder = tags_builder.reborrow().init(i as u32, chunk.len() as u32);
                for (j, tag) in chunk.iter().enumerate() {
                    let mut tag_builder = chunk_builder
                        .reborrow()
                        .get(j as u32);
                    tag_builder.set_time(tag.time);
                    tag_builder.set_channel(tag.channel)
                }
            }
        }
        return message;
    }
}