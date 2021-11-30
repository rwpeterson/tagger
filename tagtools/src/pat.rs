//! Tools for analyzing patterns in time tag datasets

use crate::Tag;
use itertools::Itertools;
use std::cmp;
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
    win: i64,
    ch_a: u8,
    ch_b: u8,
    min_delay: i64,
    max_delay: i64,
) -> Vec<u64> {
    let mut tag_iter = tags.iter().peekable();

    // Histogram stores bins of a given window size, not the time resolution
    let mut histogram: Vec<u64> = vec![0; ((max_delay - min_delay) / win) as usize + 1];

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
    let mut histogram: Vec<Vec<Tag>> =
        vec![Vec::new(); ((max_delay - min_delay) / win) as usize + 1];

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
pub fn g2(tags: &[Tag], win: i64, ch_a: u8, ch_b: u8, min_delay: i64, max_delay: i64) -> Vec<f64> {
    let total_time = (tags.last().unwrap().time - tags.first().unwrap().time) as f64;
    let singles_a = singles(tags, ch_a) as f64;
    let singles_b = singles(tags, ch_b) as f64;
    let g2_histogram = coincidence_histogram(tags, win, ch_a, ch_b, min_delay, max_delay)
        .iter()
        .map(|&b| (b as f64) * total_time / win as f64 / singles_a / singles_b)
        .collect::<Vec<f64>>();

    return g2_histogram;
}
