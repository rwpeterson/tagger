//! Tools for analyzing patterns in time tag datasets

use crate::Tag;
use itertools::Itertools;
use std::cmp;
use std::collections::BTreeMap;
use std::collections::VecDeque;

/// Count number of events in a given channel.
pub fn singles(tags: &[Tag], ch: u8) -> u64 {
    let n = tags.iter().filter(|&&t| t.channel == ch).count();
    return n as u64;
}

/// Count coincidences at a fixed delay
pub fn coincidence(
    tags: &[Tag],
    ch_a: u8,
    ch_b: u8,
    win: i64,
    delay: i64,
) -> u64 {
    let hist = coincidence_histogram(tags, ch_a, ch_b, win, delay, delay);
    return hist[0]
}

/// Calculate the raw coincidence histogram between ch_a, ch_b in a given
/// win, for delays inclusive of min_delay to max_delay.
pub fn coincidence_histogram(
    tags: &[Tag],
    ch_a: u8,
    ch_b: u8,
    win: i64,
    min_delay: i64,
    max_delay: i64,
) -> Vec<u64> {
    let mut tag_iter = tags.iter().peekable();

    // Distance to look ahead to accomodate extremes in positive and negative delay
    // Not binned by win since tags are also not binned yet when filling buffer
    let horizon = cmp::max(min_delay.saturating_abs(), max_delay);

    // Window-binned min and max delay
    let min_win = min_delay / win;
    let max_win = max_delay / win;

    // Histogram stores bins of a given window size, not the time resolution
    let mut histogram: Vec<u64> = vec![0; (max_win - min_win) as usize + 1];

    // Note below that tags are binned into windows when pushed onto the buffer
    let mut buffer: VecDeque<Tag> =
        VecDeque::with_capacity((horizon / win) as usize);

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
                    .peeking_take_while(|&&t| t.time - t0.time <= horizon)
                    .map(|&t| Tag {
                        time: t.time / win,
                        channel: t.channel,
                    }),
            );
            // Count coincidences at all nonnegative delays with t0 as ch_a
            if t0.channel == ch_a {
                for coinc in buffer
                    .iter()
                    .filter(|&&t| t.channel == ch_b)
                    // If min_win is positive, we skip from 0 to min_win
                    .skip_while(|&&t| {
                        if min_win.is_positive() {
                            t.time - t0.time < min_win
                        } else {
                            false
                        }
                    })
                    .take_while(|&&t| t.time - t0.time <= max_win)
                {
                    // Corresponds to positive delay in the autocorrelation
                    let delay = coinc.time - t0.time;
                    histogram[(delay - min_win) as usize] += 1;
                }
            // If min_win is nonpositive, consider all nonpositive delays with t0 as ch_b
            } else if !min_win.is_positive() && t0.channel == ch_b {
                for coinc in buffer
                    .iter()
                    .filter(|&&t| t.channel == ch_a)
                    // If max_win is negative, we skip from 0 to max_win
                    .skip_while(|&&t| {
                        if max_win.is_negative() {
                            t0.time - t.time > max_win
                        } else {
                            false
                        }
                    })
                    .take_while(|&&t| t0.time - t.time >= min_win)
                {
                    // Corresponds to nonpositive delay in the autocorrelation
                    // Fret not, this doesn't double count delay = 0 events
                    // since either ch_a or ch_b will be the first tag,
                    // so only one of the if or else if branches will execute.
                    // The hardware also enforces an invariant that there will
                    // be no more than one pulse per input per coincidence window
                    let delay = t0.time - coinc.time;
                    histogram[(delay - min_win) as usize] += 1;
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
    ch_a: u8,
    ch_b: u8,
    win: i64,
    min_delay: i64,
    max_delay: i64,
) -> BTreeMap<i64, f64> {
    let total_time = (tags.last().unwrap().time - tags.first().unwrap().time) as f64;
    let singles_a = singles(tags, ch_a) as f64;
    let singles_b = singles(tags, ch_b) as f64;
    let g2_histogram = coincidence_histogram(tags, ch_a, ch_b, win, min_delay, max_delay)
        .iter()
        .map(|(&d, &c)| {
            (
                d,
                (c as f64) * total_time / win as f64 / singles_a / singles_b,
            )
        })
        .collect();

    return g2_histogram;
}

/// Count coincidences using set intersection algorithm.
///
/// A linear complexity O(m + n) is possible if the two sets are sorted.
/// Because our (pre-delayed) tags are time-sorted, the two iterators of
/// each channel's tags are individually sorted, even with an arbitrary
/// delay added between them. Compare to C++'s `std::set_intersection`.
#[inline]
pub fn coincidence_intersection(tags: &[Tag], ch_a: u8, ch_b: u8, win: i64, delay: i64) -> u64 {
    let mut count = 0;
    let del_win = delay / win;
    if del_win >= 0 {
        let mut b_iter = tags.iter().filter(|&&t| t.channel == ch_b);
        if let Some(mut current_b) = b_iter.next() {
            for current_a in tags.iter().filter(|&&t| t.channel == ch_a) {
                while current_b.time / win < current_a.time / win - del_win {
                    current_b = match b_iter.next() {
                        Some(current_b) => current_b,
                        None => return count,
                    };
                }
                if current_a.time / win == (current_b.time - delay) / win {
                    count += 1;
                }
            }
        }
    // TODO: remove this branch by writing the arithmetic the right way
    } else {
        let mut a_iter = tags.iter().filter(|&&t| t.channel == ch_a);
        if let Some(mut current_a) = a_iter.next() {
            for current_b in tags.iter().filter(|&&t| t.channel == ch_b) {
                while current_a.time / win < current_b.time / win + del_win {
                    current_a = match a_iter.next() {
                        Some(current_b) => current_b,
                        None => return count,
                    };
                }
                if current_b.time / win == (current_a.time + delay) / win {
                    count += 1;
                }
            }
        }
    }
    count
}
