use either::Either;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tagtools::{bit, pat, Tag};

pub const WIN_DEFAULT: u32 = 1;

pub struct RawTags {
    pub dur: u64,
    pub tags: Arc<Vec<Tag>>,
}

pub struct TagData {
    pub dur: u64,
    pub tags: Arc<Vec<Tag>>,
    pub counts: HashMap<(u16, Option<u32>), u64>,
}

pub struct LogicData {
    pub dur: u64,
    pub counts: HashMap<(u16, Option<u32>), u64>,
}

pub type RawData = Either<RawTags, LogicData>;
pub type PubData = Either<TagData, LogicData>;

/// Calculate the counts in a set of pattern masks, doing the calculations in parallel
pub fn count_patterns(tags: &[Tag], patmasks: HashSet<(u16, Option<u32>)>) -> HashMap<(u16, Option<u32>), u64> {
    use rayon::prelude::*;

    let mut hm = HashMap::<(u16, Option<u32>), u64>::new();
    // Preallocate the hashmap so we can perform the calculations in parallel
    for (pat, win) in patmasks {
        match pat.count_ones() {
            1 => {
                hm.insert((pat, win), 0);
            }
            2 => {
                hm.insert((pat, win), 0);
            }
            // TODO: Implement higher-order patterns
            _ => {}
        }
    }
    hm.par_iter_mut().for_each(|((pat, win), count)| {
        match pat.count_ones() {
            1 => {
                *count += pat::singles(&tags.clone(), bit::mask_to_single(*pat).unwrap());
            }
            2 => {
                let (ch_a, ch_b) = bit::mask_to_pair(*pat).unwrap();
                *count += pat::coincidence(
                    &tags.clone(),
                    ch_a,
                    ch_b,
                    win.unwrap_or(WIN_DEFAULT).into(),
                    0,
                );
            }
            // TODO: Implement higher-order patterns
            _ => {}
        }
    });
    hm
}