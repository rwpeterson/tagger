use anyhow::Result;
use parking_lot::{Mutex, RwLock};
use std::collections::HashSet;
use std::sync::Arc;
use tagtools::Tag;
use tagtools::ser::fillmsg;

use crate::data::{count_patterns, PubData};

pub fn main(
    receiver: flume::Receiver<(Vec<Tag>, u64)>,
    sender: flume::Sender<()>,
    data: Arc<Mutex<PubData>>,
    cur_tagmask: Arc<RwLock<u16>>,
    cur_patmasks: Arc<RwLock<HashSet<u16>>>,
) -> Result<()> {
    std::thread::spawn(move || loop {
        match receiver.recv() {
            Ok((tags, dur)) => {
                // Check in on what to process
                let t = cur_tagmask.read();
                let _tagmask = *t;
                let p = cur_patmasks.read();
                let patmasks = (*p).clone();

                let mut data = data.lock();

                fillmsg(&mut data.tags, &tags.clone());

                data.patcounts = count_patterns(&tags.clone(), patmasks);

                data.duration = dur;

                // Signal to publisher
                sender.send(()).unwrap();
            },
            Err(_) => break,
        }
    });
    Ok(())
}