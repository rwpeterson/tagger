//! Tools for analyzing patterns in time tag datasets

use crate::Tag;
use bit_iter::BitIter;
use itertools::Itertools;
use std::cmp;
use std::collections::{HashMap, VecDeque};

/// Count number of events in a given channel.
pub fn singles(tags: &[Tag], ch: u8) -> usize {
    let n = tags.iter().filter(|&&t| t.channel == ch).count();
    return n;
}

/// Count coincidences at a fixed delay
pub fn coincidence(tags: &[Tag], ch_a: u8, ch_b: u8, win: i64, delay: i64) -> usize {
    // Ensure delay is commensurate with the windowing we do when pushing into
    // the deque later
    let delay_win = delay / win;

    let mut tag_iter = tags.iter().peekable();

    let mut count: usize = 0;

    // Note below that tags are binned into windows when pushed onto the buffer
    let mut buffer: VecDeque<Tag> = VecDeque::with_capacity(delay_win as usize);

    // Seed the buffer with one tag
    if let Some(&t) = tag_iter.next() {
        buffer.push_back(Tag {
            time: t.time / win,
            channel: t.channel,
        })
    }

    // We only look at a fixed delay between ch_a and ch_b
    while !buffer.is_empty() {
        if let Some(t0) = buffer.pop_front() {
            // Fill buffer
            buffer.extend(
                tag_iter
                    .peeking_take_while(|&&t| t.time - t0.time <= delay_win)
                    // Bin tag into win
                    .map(|&t| Tag {
                        time: t.time / win,
                        channel: t.channel,
                    }),
            );
            // Check if there is a tag at positive delay
            if !delay_win.is_negative() && t0.channel == ch_a {
                if let Some(_) = buffer
                    .iter()
                    .filter(|&&t| t.channel == ch_b)
                    .peekable()
                    .peeking_take_while(|&&t| t.time - t0.time <= delay_win)
                    .filter(|&&t| t.time - t0.time == delay_win)
                    // Iter should be empty if no matching tag
                    .peekable()
                    .peek()
                {
                    count += 1;
                }
            // Else check if there is a tag at negative delay
            } else if delay_win.is_negative() && t0.channel == ch_b {
                if let Some(_) = buffer
                    .iter()
                    .filter(|&&t| t.channel == ch_a)
                    .peekable()
                    .peeking_take_while(|&&t| t.time - t0.time <= delay_win)
                    .filter(|&&t| t.time - t0.time == delay_win)
                    // Iter should be empty if no matching tag
                    .peekable()
                    .peek()
                {
                    count += 1;
                }
            }
        }
        // Don't leave buffer empty for the next loop
        if buffer.is_empty() {
            if let Some(&t) = tag_iter.next() {
                buffer.push_back(Tag {
                    time: t.time / win,
                    channel: t.channel,
                })
            }
        }
    }
    return count;
}

/// Count arbitrary patterns at a fixed delay per channel
pub fn patterns(tags: &[Tag], pats: &[u16], win: i64, delays: [i64; 16]) -> HashMap<u16, u64> {
    // Time-related values are signed to avoid ubiquitous checking during casting,
    // but the following need to be nonnegative:
    assert!(win >= 0);
    assert!(delays.iter().map(|d| !d.is_negative()).fold(true, |a, d| a & d));

    // Delay, bin, and re-sort the tags
    // After this point, tags are coincident if their times are identical
    let t = tags.iter()
        .map(|&t| Tag {
            time: (t.time + delays[t.channel as usize - 1]) / win,
            channel: t.channel
        })
        .sorted()
        .collect::<Vec<_>>();

    let mut tag_iter = t.iter().peekable();

    let mut counts = HashMap::<u16, u64>::new();
    for &pat in pats {
        counts.insert(pat, 0);
    }

    // We expect to see ~N events per window for an N-fold pattern
    let x = pats.iter().map(|&y| BitIter::from(y).count()).max().unwrap_or(1);
    let mut buffer = VecDeque::<Tag>::with_capacity(2 * x);

    // Seed the buffer with one tag
    if let Some(&t) = tag_iter.next() {
        buffer.push_back(t);
    }

    while !buffer.is_empty() {
        if let Some(t0) = buffer.pop_front() {
            // Fill buffer with tags at same time
            buffer.extend(
                tag_iter.peeking_take_while(|&&t| t.time == t0.time)
            );
            // Drain buffer into pattern mask to check against
            let mask = buffer.drain(..).fold(0u16, |a, t| 1 << (t.channel - 1) | a);
            for pat in pats {
                if *pat & mask == *pat {
                    if let Some(c) = counts.get_mut(pat) {
                        *c += 1;
                    }
                }
            }
        }
        // Don't leave buffer empty for the next loop
        if buffer.is_empty() {
            if let Some(&t) = tag_iter.next() {
                buffer.push_back(t);
            }
        }
    }
    return counts;
}

/// Calculate the raw coincidence histogram between ch_a, ch_b in a given
/// win, for delays inclusive of min_delay to max_delay.
pub fn coincidence_histogram(
    tags: &[Tag],
    win: i64,
    ch_a: u8,
    ch_b: u8,
    min_delay: i64,
    max_delay: i64,
) -> Vec<usize> {
    let mut tag_iter = tags.iter().peekable();

    // Histogram stores bins of a given window size, not the time resolution
    let mut histogram: Vec<usize> = vec![0; ((max_delay - min_delay) / win) as usize + 1];

    // Note below that tags are binned into windows when pushed onto the buffer
    let mut buffer: VecDeque<Tag> =
        VecDeque::with_capacity(cmp::max(min_delay.abs(), max_delay) as usize);

    // Seed the buffer with one tag
    if let Some(&t) = tag_iter.next() {
        buffer.push_back(Tag {
            time: t.time / win,
            channel: t.channel,
        })
    }

    // We scan all relevant delays for each tag, always looking at later tags
    // for positive (negative) delays when the first tag is from ch_a (ch_b).
    while !buffer.is_empty() {
        if let Some(t0) = buffer.pop_front() {
            // Fill buffer
            buffer.extend(
                tag_iter
                    .peeking_take_while(|&&t| t.time - t0.time <= max_delay)
                    .map(|&t| Tag {
                        time: t.time / win,
                        channel: t.channel,
                    }),
            );
            // Count coincidences with tag t0 at all relevant delays
            if t0.channel == ch_a {
                for coinc in buffer
                    .iter()
                    .filter(|&&t| t.channel == ch_b)
                    // If min_delay is positive, we skip from 0 to min_delay
                    .skip_while(|&&t| {
                        if min_delay.is_positive() {
                            t.time - t0.time < min_delay
                        } else {
                            false
                        }
                    })
                    .take_while(|&&t| t.time - t0.time <= max_delay)
                {
                    // Corresponds to positive delay in the autocorrelation
                    let delay = coinc.time - t0.time;
                    histogram[(delay - min_delay) as usize] += 1;
                }
            // If min_delay is negative, we consider negative delays separately
            } else if min_delay.is_negative() && t0.channel == ch_b {
                for coinc in buffer
                    .iter()
                    .filter(|&&t| t.channel == ch_a)
                    .take_while(|&&t| t0.time - t.time >= min_delay)
                {
                    // Corresponds to negative delay in the autocorrelation
                    // Fret not, this doesn't double count tau = 0 events
                    // since we only look forward in time
                    let delay = t0.time - coinc.time;
                    histogram[(delay - min_delay) as usize] += 1;
                }
            }
        }
        // Don't leave buffer empty for the next loop
        if buffer.is_empty() {
            if let Some(&t) = tag_iter.next() {
                buffer.push_back(Tag {
                    time: t.time / win,
                    channel: t.channel,
                })
            }
        }
    }
    return histogram;
}

/// Find the tags with a coincidence between ch_a, ch_b in a given win,
/// returning 2D Vec of tags coincident at each delay, inclusive of min_delay
/// to max_delay. The retained tag is the earliest of the two in the
/// coincidence, from ch_a (ch_b) for pos (neg) delays.
pub fn tag_histogram(
    tags: &[Tag],
    win: i64,
    ch_a: u8,
    ch_b: u8,
    min_delay: i64,
    max_delay: i64,
) -> Vec<Vec<Tag>> {
    let mut tag_iter = tags.iter().peekable();

    // Histogram stores bins of a given window size, not the time resolution
    let mut histogram: Vec<Vec<Tag>> = vec![Vec::new(); ((max_delay - min_delay) / win) as usize + 1];

    // Note below that tags are binned into windows when pushed onto the buffer
    let mut buffer: VecDeque<Tag> =
        VecDeque::with_capacity(cmp::max(min_delay.abs(), max_delay) as usize);

    // Seed the buffer with one tag
    if let Some(&t) = tag_iter.next() {
        buffer.push_back(Tag {
            time: t.time / win,
            channel: t.channel,
        })
    }

    // We scan all relevant delays for each tag, always looking at later tags
    // for positive (negative) delays when the first tag is from ch_a (ch_b).
    while !buffer.is_empty() {
        if let Some(t0) = buffer.pop_front() {
            // Fill buffer
            buffer.extend(
                tag_iter
                    .peeking_take_while(|&&t| t.time - t0.time <= max_delay)
                    .map(|&t| Tag {
                        time: t.time / win,
                        channel: t.channel,
                    }),
            );
            // Count coincidences with tag t0 at all relevant delays
            if t0.channel == ch_a {
                for coinc in buffer
                    .iter()
                    .filter(|&&t| t.channel == ch_b)
                    // If min_delay is positive, we skip from 0 to min_delay
                    .skip_while(|&&t| {
                        if min_delay.is_positive() {
                            t.time - t0.time < min_delay
                        } else {
                            false
                        }
                    })
                    .take_while(|&&t| t.time - t0.time <= max_delay)
                {
                    // Corresponds to positive delay in the autocorrelation
                    let delay = coinc.time - t0.time;
                    histogram[(delay - min_delay) as usize].push(t0);
                }
            // If min_delay is negative, we consider negative delays separately
            } else if min_delay.is_negative() && t0.channel == ch_b {
                for coinc in buffer
                    .iter()
                    .filter(|&&t| t.channel == ch_a)
                    .take_while(|&&t| t0.time - t.time >= min_delay)
                {
                    // Corresponds to negative delay in the autocorrelation
                    // Fret not, this doesn't double count tau = 0 events
                    // since we only look forward in time
                    let delay = t0.time - coinc.time;
                    histogram[(delay - min_delay) as usize].push(t0);
                }
            }
        }
        // Don't leave buffer empty for the next loop
        if buffer.is_empty() {
            if let Some(&t) = tag_iter.next() {
                buffer.push_back(Tag {
                    time: t.time / win,
                    channel: t.channel,
                })
            }
        }
    }
    return histogram;
}

/// Calculate the second-order degree of coherence, or g^(2) function, of light
/// from photon correlations, as in an intensity interferometer or
/// Hanbury Brown-Twiss experiment. Window, channels, and delay range specified
/// as in coincidence_histogram, as this is essentially a normalization of that
/// histogram to the singles rates and window size.
pub fn g2(
    tags: &[Tag],
    win: i64,
    ch_a: u8,
    ch_b: u8,
    min_delay: i64,
    max_delay: i64,
) -> Vec<f64> {
    let total_time = (tags.last().unwrap().time - tags.first().unwrap().time) as f64;
    let singles_a = singles(tags, ch_a) as f64;
    let singles_b = singles(tags, ch_b) as f64;
    let g2_histogram = coincidence_histogram(tags, win, ch_a, ch_b, min_delay, max_delay)
        .iter()
        .map(|&b| (b as f64) * total_time / win as f64 / singles_a / singles_b)
        .collect::<Vec<f64>>();

    return g2_histogram;
}
