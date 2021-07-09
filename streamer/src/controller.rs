use anyhow::Result;
use parking_lot::{Mutex, RwLock};
use std::collections::HashSet;
use std::sync::Arc;
use tagtools::{CHAN16, Tag};
use tagtools::pat;
use timetag::ffi::{new_time_tagger, FfiTag};

use crate::data::PubData;
use crate::tags_capnp::tags;

pub fn main(
    data: Arc<Mutex<PubData>>,
    cur_tagmask: Arc<RwLock<u16>>,
    cur_patmasks: Arc<RwLock<HashSet<u16>>>,
    rx: flume::Receiver<()>,
    tx: flume::Sender<()>,
) -> Result<()> {
    let t = new_time_tagger();
    t.open();
    for ch in CHAN16 {
        t.set_input_threshold(ch, 2.0);
    }
    t.set_fg(200_000, 100_000);
    t.start_timetags();
    t.freeze_single_counter();
    while let Ok(()) = rx.recv() {
        // Acquire new data
        let tags = Arc::new(t.read_tags()
            .iter()
            .map(|t: &FfiTag| Tag { time: t.time, channel: t.channel })
            .collect::<Vec<_>>()
        );
        let dur = t.freeze_single_counter();
        
        // Check in on what to process
        let t = cur_tagmask.read();
        let _tagmask = *t;
        let ps = cur_patmasks.read();
        let patmasks = (*ps).clone();

        let mut data = data.lock();
        { // copypasta from tagtools::ser
            let message_builder = data.tags.init_root::<tags::Builder>();

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
        data.patcounts = pat::patterns(
            &tags.clone(),
            &patmasks.into_iter().collect::<Vec<_>>(),
            1,
            [0; 16]
        );
        data.duration = dur;
        
        // Signal to publisher
        tx.send(()).unwrap();
    }
    t.stop_timetags();
    t.close();
    Ok(())
}