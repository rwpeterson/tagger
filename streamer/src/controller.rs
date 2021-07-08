use anyhow::Result;
use parking_lot::{Mutex, RwLock};
use std::collections::HashSet;
use std::sync::Arc;
use tagtools::{CHAN16, Tag};
use tagtools::pat;
use timetag::ffi::{new_time_tagger, FfiTag};

use crate::data::PubData;

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
        data.tags = (&tags.clone()).to_vec();
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