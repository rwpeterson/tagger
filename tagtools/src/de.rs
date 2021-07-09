//! Deserialization of time tag objects, supporting `.tags` and `.tsv`

use crate::tags_capnp::tags;
use crate::{Bin, Tag};
use anyhow::Result;
use capnp::serialize;
use capnp::message::ReaderOptions;
use std::io::{BufReader, Read};
use std::vec::Vec;
use zstd::stream;

/// Deserialize from .tags format: zstd-compressed Cap'n Proto tags
///
/// Like many compressors, `zstd`'s API is linear under concatenation, in that
/// `zstd(m1 + m2) == zstd(m1) + zstd(m2)` (ignoring that the compressed bytes
/// will actually differ). So while we may write repeated compressed messages
/// when saving data, it suffices to decompress the entire stream at once.
pub fn tags(rdr: impl Read) -> Result<Vec<Tag>> {
    let mut zrdr = stream::read::Decoder::new(rdr)?;
    let tags = tags_uncompressed(&mut zrdr)?;
    Ok(tags)
}

/// Deserialize to uncompressed, unpacked Cap'n Proto tags.
///
/// As an implementation detail, tags are serialized as a `List(List(Tag))`,
/// as there is a 4 GiB limit per `struct_list`.  This deserializes to a
/// flattened `Vec<Tag>`, and furthermore concatenates all messages in the
/// buffer into one `Vec`.
pub fn tags_uncompressed(rdr: &mut impl Read) -> Result<Vec<Tag>> {
    let mut brdr = BufReader::new(rdr);
    let mut tags: Vec<Tag> = Vec::new();

    // Traversal limit is 64 MiB by default as a simple DoS mitigation.
    // To read in arbitrarily-large datasets, we need to disable this.
    let rdr_opts = ReaderOptions{
        traversal_limit_in_words: None,
        ..Default::default()
    };

    while let Some(message_reader) =
        serialize::try_read_message(&mut brdr, rdr_opts)?
    {
        let tags_reader = message_reader.get_root::<tags::Reader>()?;

        for chunk in tags_reader.get_tags()?.iter() {
            for tag in chunk?.iter() {
                tags.push(Tag { time: tag.get_time(), channel: tag.get_channel() })
            }
        }
    }

    Ok(tags)
}

/// Deserialize tags from tab-separated values (channel, time).
pub fn tsv(rdr: &mut csv::Reader<impl Read>) -> Result<Vec<Tag>> {
    let mut tags: Vec<Tag> = Vec::new();
    for result in rdr.records() {
        let record = result?;
        tags.push(Tag {
            time: record[1].parse::<i64>()?,
            channel: record[0].parse::<u8>()?,
        });
    }
    Ok(tags)
}

/// Deserialize a tab-separated histogram file of (x,y) records.
pub fn histogram_tsv<R, T, U>(rdr: &mut csv::Reader<R>,) -> anyhow::Result<Vec<Bin<T, U>>>
where
    R: std::io::Read,
    T: std::str::FromStr,
    U: std::str::FromStr,
{
    let mut bins: Vec<Bin<T, U>> = Vec::new();
    for result in rdr.records() {
        let record = result?;
        if let (Ok(x), Ok(y)) = (record[0].parse::<T>(), record[1].parse::<U>()) {
            bins.push(Bin { x, y });
        }
    }
    Ok(bins)
}