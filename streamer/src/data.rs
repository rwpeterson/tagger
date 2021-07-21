use capnp::message;
use std::collections::{HashMap, HashSet};
use tagtools::{bit, pat, Tag};

#[allow(dead_code)]
pub struct TagPattern {
    tagmask: u16,
    duration: u64,
    tags: Vec<Tag>,
}

#[allow(dead_code)]
pub struct LogicPattern {
    patmask: u16,
    duration: u64,
    count: u64
}

/// Data from the tagger that needs to be passed between the controller and server.
/// Due to the size involved, we immediately create the capnp message for the tags
/// instead of passing them back and forth in memory several times first.
pub struct PubData {
    pub duration: u64,
    pub tags: Box<message::Builder<message::HeapAllocator>>,
    pub patcounts: HashMap<u16,u64>,
}


/// Calculate the counts in a set of pattern masks, doing the calculations in parallel
pub fn count_patterns(tags: &[Tag], patmasks: HashSet<u16>) -> HashMap<u16, u64> {
    use rayon::prelude::*;

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
                *count += pat::singles(&tags.clone(), bit::mask_to_single(*pat).unwrap());
            }
            2 => {
                let (ch_a, ch_b) = bit::mask_to_pair(*pat).unwrap();
                *count += pat::coincidence(&tags.clone(), ch_a, ch_b, 1, 0);
            }
            // TODO: Implement higher-order patterns
            _ => {}
        }
    });
    hm
}