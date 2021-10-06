use anyhow::Result;
use parking_lot::RwLock;
use std::collections::HashSet;
use std::sync::Arc;
use tagtools::Tag;
use std::collections::HashMap;

use crate::data::count_patterns;

pub fn main(
    receiver: flume::Receiver<(Vec<Tag>, u64)>,
    sender: flume::Sender<(u64, Vec<Tag>, HashMap<(u16, Option<u32>),u64>)>,
    cur_tagmask: Arc<RwLock<u16>>,
    cur_patmasks: Arc<RwLock<HashSet<(u16, Option<u32>)>>>,
) -> Result<()> {
    std::thread::spawn(move || loop {
        match receiver.recv() {
            Ok((tags, dur)) => {
                // Check in on what to process
                let t = cur_tagmask.read();
                let _tagmask = *t;
                let p = cur_patmasks.read();
                let patmasks = (*p).clone();

                let patcounts = count_patterns(&tags.clone(), patmasks);


                // Signal to publisher
                sender.send((dur, tags, patcounts)).unwrap();
            },
            Err(_) => break,
        }
    });
    Ok(())
}