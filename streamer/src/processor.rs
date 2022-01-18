use anyhow::Result;
use either::Either;
use parking_lot::RwLock;
use std::collections::HashSet;
use std::sync::Arc;

use crate::data::{count_patterns, RawData, RawTags, TagData, PubData};

/// Performs singles and coincidence rate calculations on tags in a thread pool,
/// or just passes through if in logic mode and no computation needs to be done.
pub fn main(
    receiver: flume::Receiver<RawData>,
    sender: flume::Sender<PubData>,
    cur_tagmask: Arc<RwLock<u16>>,
    cur_patmasks: Arc<RwLock<HashSet<(u16, Option<u32>)>>>,
) -> Result<()> {
    std::thread::spawn(move || loop {
        match receiver.recv() {
            Ok(Either::Left(RawTags {dur, tags})) => {
                // Check in on what to process
                let t = cur_tagmask.read();
                let _tagmask = *t;
                let p = cur_patmasks.read();
                let patmasks = (*p).clone();

                let counts = count_patterns(&tags.clone(), patmasks);

                sender.send(Either::Left(TagData { dur, tags: tags.clone(), counts })).unwrap();
            },
            Ok(Either::Right(ld)) => {
                // Just pass along
                sender.send(Either::Right(ld)).unwrap();
            }
            Err(_) => break,
        }
    });
    Ok(())
}