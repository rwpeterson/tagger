use anyhow::Result;
use bit_iter::BitIter;
use parking_lot::{Mutex, RwLock};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tagtools::pat;
use tagtools::{Tag, CHAN16};
use timetag::ffi::{new_time_tagger, FfiTag};

use rayon::prelude::*;

use crate::data::PubData;
use crate::tags_capnp::tags;
use crate::Event;

pub fn main(
    data: Arc<Mutex<PubData>>,
    cur_tagmask: Arc<RwLock<u16>>,
    cur_patmasks: Arc<RwLock<HashSet<u16>>>,
    rx: flume::Receiver<Event>,
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
    loop {
        match rx.recv() {
            Ok(Event::Tick) => {
                // Acquire new data
                let tags = Arc::new(
                    t.read_tags()
                        .iter()
                        .map(|t: &FfiTag| Tag {
                            time: t.time,
                            channel: t.channel,
                        })
                        .collect::<Vec<_>>(),
                );
                let dur = t.freeze_single_counter();

                // Check in on what to process
                let t = cur_tagmask.read();
                let _tagmask = *t;
                let ps = cur_patmasks.read();
                let patmasks = (*ps).clone();

                let mut data = data.lock();
                {
                    // copypasta from tagtools::ser
                    let message_builder = data.tags.init_root::<tags::Builder>();

                    // Cap'n Proto lists are limited to a max of 2^29 elements, and
                    // additionally for struct lists, to a max of 2^29 words of data.
                    // Since each Tag is two words, we can store 2^28 Tags per List.
                    let full_lists: u32 = (tags.len() / 2usize.pow(28)) as u32;
                    let remainder: u32 = (tags.len() % 2usize.pow(28)) as u32;

                    let mut tags_builder = message_builder.init_tags(if remainder > 0 {
                        full_lists + 1
                    } else {
                        full_lists
                    });
                    for (i, chunk) in tags.chunks(2usize.pow(29)).enumerate() {
                        let mut chunk_builder =
                            tags_builder.reborrow().init(i as u32, chunk.len() as u32);
                        for (j, tag) in chunk.iter().enumerate() {
                            let mut tag_builder = chunk_builder.reborrow().get(j as u32);
                            tag_builder.set_time(tag.time);
                            tag_builder.set_channel(tag.channel)
                        }
                    }
                }
                data.patcounts = {
                    let mut hm = HashMap::<u16, u64>::new();
                    // Preallocate the hashmap so we can perform the calculations in parallel
                    for pat in patmasks {
                        match pat.count_ones() {
                            1 => {
                                hm.insert(pat, 0);
                            }
                            2 => {
                                hm.insert(pat, 0);
                            }
                            // TODO: Implement higher-order patterns
                            _ => {}
                        }
                    }
                    hm.par_iter_mut().for_each(|(pat, count)| {
                        match pat.count_ones() {
                            1 => {
                                *count +=
                                    pat::singles(&tags.clone(), mask_to_single(*pat).unwrap());
                            }
                            2 => {
                                let (ch_a, ch_b) = mask_to_pair(*pat).unwrap();
                                *count += pat::coincidence(&tags.clone(), ch_a, ch_b, 1, 0);
                            }
                            // TODO: Implement higher-order patterns
                            _ => {}
                        }
                    });
                    hm
                };
                data.duration = dur;

                // Signal to publisher
                tx.send(()).unwrap();
            }
            Ok(Event::Set(_)) => {}
            Err(_) => break,
        }
    }
    t.stop_timetags();
    t.close();
    Ok(())
}

/// Returns a single channel if the mask is one channel
fn mask_to_single(m: u16) -> Option<u8> {
    match m.count_ones() {
        1 => Some(BitIter::from(m).next().unwrap() as u8),
        _ => None,
    }
}

/// Returns a tuple of channels if the mask is two channels
fn mask_to_pair(m: u16) -> Option<(u8, u8)> {
    match m.count_ones() {
        2 => {
            let mut i = BitIter::from(m);
            Some((i.next().unwrap() as u8, i.next().unwrap() as u8))
        }
        _ => None,
    }
}
