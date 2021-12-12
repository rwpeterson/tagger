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
///
/// We have two implementations:
/// - `coincidence_intersection`, using set intersection
/// - `coincidence_histogram_1`, using the histogram algorithm at a single delay
/// The set intersection algorithm is ~50% faster and used by default.
/// Run `cargo bench` to benchmark this yourself.
///
/// Integration tests cross-check both implementations against each other
/// and known-correct results from other codes.
pub fn coincidence(tags: &[Tag], ch_a: u8, ch_b: u8, win: i64, delay: i64) -> u64 {
    coincidence_intersection(tags, ch_a, ch_b, win, delay)
}

/// Count coincidences using the histogram algorithm at a single fixed delay
#[inline]
pub fn coincidence_histogram_1(tags: &[Tag], ch_a: u8, ch_b: u8, win: i64, delay: i64) -> u64 {
    let hist = coincidence_histogram(tags, ch_a, ch_b, win, delay, delay);
    return *(hist.get(&(delay / win * win)).unwrap());
}

/// Calculate the raw coincidence histogram between `ch_a`, `ch_b` in a given
/// `win`, for delays inclusive of `min_delay` to `max_delay`.
///
/// Because the tags are already time-sorted, this algorithm processes them
/// in linear time. A deque (double-ended queue) holds an initial tag plus
/// all later tags within the delay interval. Each combination of the initial
/// tag and subsequent tags in the deque represents a coincidence if they are
/// from the two requested channels: the count of coincidences at each delay is
/// incremented. If the first tag is from `ch_a`, later tags from `ch_b` are
/// assigned positive delay; conversely, if the first tag is from `ch_b`, later
/// tags from `ch_a` are assigned negative delay. This processing of the deque
/// can therefore also be done in linear time. After this step, the first
/// tag is popped off the deque and discarded, and the process repeats.
///
/// The time performance of the algorithm should be `O(n * m)`, where there
/// are `n` tags and `m = max(min_delay, max_delay)` is the length of the deque
/// (up to a scaling factor).
pub fn coincidence_histogram(
    tags: &[Tag],
    ch_a: u8,
    ch_b: u8,
    win: i64,
    min_delay: i64,
    max_delay: i64,
) -> BTreeMap<i64, u64> {
    // Peekable iterator over the tags, binned by win via integer division
    // We don't multiply again by win to restore the original scale of the
    // time units; this is done later when required.
    let mut tag_iter = tags
        .iter()
        .map(|t| Tag { time: t.time / win, channel: t.channel })
        .peekable();

    // Scaled min and max delay for index calculations
    let min_win = min_delay / win;
    let max_win = max_delay / win;

    // Distance to look ahead to accommodate extremes in positive and negative delay
    let horizon = cmp::max(min_win.abs(), max_win);

    // Window-binned min and max delay in physical units
    let min = min_win * win;
    let max = max_win * win;

    // Histogram stores bins of a given window size, not the time resolution
    let mut histogram: BTreeMap<i64, u64> = BTreeMap::new();
    (min..=max).step_by(win as usize).for_each(|d| {
        histogram.insert(d, 0);
    });

    // Moving FIFO to hold one tag plus every subsequent tag in its horizon
    let mut buffer: VecDeque<Tag> = VecDeque::with_capacity(horizon as usize);

    // Seed the buffer with one tag
    if let Some(t) = tag_iter.next() {
        buffer.push_back(t);
    }

    // We scan all relevant delays for each tag, always looking at later tags
    // for positive (negative) delays when the first tag is from ch_a (ch_b).
    while !buffer.is_empty() {
        // This pops one tag per loop, the rest is recording the relative delays
        // to subsequent tags when they are between min_delay and max_delay
        if let Some(t0) = buffer.pop_front() {
            // Extend buffer to catch anything possibly within min/max delays
            buffer.extend(
                tag_iter
                    .peeking_take_while(|t| t.time - t0.time <= horizon)
            );
            // Count coincidences at all non-negative delays with t0 as ch_a
            if t0.channel == ch_a {
                buffer
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
                    .for_each(|coinc| {
                        // Corresponds to positive delay in the histogram,
                        // multiplied back up to the physical pre-windowing
                        // timescale
                        let delay = (coinc.time - t0.time) * win;
                        *(histogram.get_mut(&delay).unwrap()) += 1;
                    });
            // Count coincidences at all non-positive delays with t0 as ch_b
            } else if !min_win.is_positive() && t0.channel == ch_b {
                buffer
                    .iter()
                    .filter(|&&t| t.channel == ch_a)
                    // If max_win is negative, we skip from 0 to max_win
                    .skip_while(|&&t| {
                        if max_win.is_negative() {
                            t.time - t0.time < - max_win
                        } else {
                            false
                        }
                    })
                    .take_while(|&&t| t.time - t0.time <= - min_win)
                    .for_each(|coinc| {
                        // Corresponds to non-positive delay in the histogram
                        // Fret not, this doesn't double count delay = 0 events
                        // since either ch_a or ch_b will be the first tag,
                        // so only one of the if or else-if branches will execute.
                        // The first tag will be gone in the next loop so the
                        // opposite case is not present to double-count.
                        let delay = - (coinc.time - t0.time) * win;
                        *(histogram.get_mut(&delay).unwrap()) += 1;
                    });
            }
        }

        // Don't leave buffer empty for the next loop
        if buffer.is_empty() {
            if let Some(t) = tag_iter.next() {
                buffer.push_back(t)
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
