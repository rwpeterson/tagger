use tagtools::{pat, Tag};

mod common;

#[test]
fn singles() {
    let tags = common::load_test_data();
    assert_eq!(pat::singles(&tags, 3), 223_058);
    assert_eq!(pat::singles(&tags, 15), 251_662);
    assert_eq!(pat::singles(&tags, 16), 25_280);
}

#[test]
fn coincidence() {
    let tags = common::load_test_data();
    assert_eq!(76, pat::coincidence(&tags, 3, 15, 1, 26));
    assert_eq!(3, pat::coincidence(&tags, 3, 15, 1, -8));
}

#[test]
fn coincidence_histogram_1() {
    let tags = common::load_test_data();
    assert_eq!(76, pat::coincidence_histogram_1(&tags, 3, 15, 1, 26));
    assert_eq!(3, pat::coincidence_histogram_1(&tags, 3, 15, 1, -8));
}

#[test]
fn coincidence_intersection() {
    let tags = common::load_test_data();
    assert_eq!(76, pat::coincidence_intersection(&tags, 3, 15, 1, 26));
    assert_eq!(3, pat::coincidence_intersection(&tags, 3, 15, 1, -8));
}

/// Compare coincidence histogram against results from known-good code.
#[test]
fn coincidence_histogram_vs_other_code() {
    let tags = common::load_test_data();
    let proof_histogram = common::load_coincidence_histogram();
    let histogram = pat::coincidence_histogram(&tags, 3, 15, 1, -64, 64);

    assert_eq!(proof_histogram.len(), histogram.len());
    for (i, proof) in proof_histogram.into_iter().enumerate() {
        // skip the first and last bins since they are zero in the reference calculation
        if i == 0 {
            continue;
        } else if i == 128 {
            continue;
        }
        let histogram_x = (-64. + i as f64) * 0.15625; // in ns
        assert!((proof.x - histogram_x).abs() < 1e-4);
        assert_eq!(&proof.y, histogram.get(&(-64 + i as i64)).unwrap());
    }
}

/// Compare calculation of g2 against results from known-good code.
#[test]
fn g2_histogram_vs_other_code() {
    let tags = common::load_test_data();
    let proof_g2 = common::load_g2_histogram();
    let g2 = pat::g2(&tags, 3, 15, 1, -64, 64);
    assert_eq!(proof_g2.len(), g2.len());
    for (i, proof) in proof_g2.into_iter().enumerate() {
        // skip the first and last bins since they are zero in the reference calculation
        if i == 0 {
            continue;
        } else if i == 128 {
            continue;
        }
        let g2_x = (-64. + i as f64) * 0.15625; // in ns
        assert!((proof.x - g2_x).abs() < 1e-4);
        assert!((&proof.y - g2.get(&(-64 + i as i64)).unwrap()).abs() < 3e-4); // all but x = 3.75, 3.9063, 4.0625 are < 1e-4
    }
}

/// Compare coincidence histogram against values calculated at a single delay.
#[test]
fn coincidence_histogram_indexing() {
    let tags = common::load_test_data();
    let ch_a: u8 = 3;
    let ch_b: u8 = 15;
    let win: i64 = 1;
    let min_delay: i64 = -64;
    let max_delay: i64 = 64;

    let mut cts = std::collections::BTreeMap::new();

    let histogram = pat::coincidence_histogram(&tags, ch_a, ch_b, win, min_delay, max_delay);

    for delay in min_delay..=max_delay {
        cts.insert(
            delay,
            pat::coincidence_histogram_1(&tags, ch_a, ch_b, win, delay),
        );
    }

    assert_eq!(&cts, &histogram);
}

/// Delay one channel's tags to verify the histogram is shifted by the same amount.
#[test]
fn coincidence_histogram_delay() {
    let tags = common::load_test_data();

    let ch_a = 3;
    let ch_b = 15;
    let win = 1;
    let min_delay = -64;
    let max_delay = 64;

    let h_0 = pat::coincidence_histogram(&tags, ch_a, ch_b, win, min_delay, max_delay);

    let shift_min = -10;
    let shift_max = 10;

    for d in shift_min..=shift_max {
        let tags_d: Vec<Tag> = tags
            .clone()
            .into_iter()
            .map(|t| {
                if t.channel == ch_b {
                    Tag {
                        time: t.time + d,
                        channel: t.channel,
                    }
                } else {
                    Tag {
                        time: t.time,
                        channel: t.channel,
                    }
                }
            })
            .collect();
        let h_d = pat::coincidence_histogram(&tags_d, ch_a, ch_b, win, min_delay, max_delay);
        for i in (min_delay + d.abs())..=(max_delay - d.abs()) {
            assert_eq!(h_0.get(&i), h_d.get(&(i + d)));
        }
    }
}

/// Delay one channel's tags to verify the histogram is shifted by the same amount, for several windows
#[test]
fn coincidence_histogram_delay_win() {
    let tags = common::load_test_data();

    let ch_a = 3;
    let ch_b = 15;
    let min_delay = -64;
    let max_delay = 64;

    for win in [1, 2, 3, 4] {
        let min_delay_win = min_delay / win;
        let max_delay_win = max_delay / win;

        let h_0 = pat::coincidence_histogram(&tags, ch_a, ch_b, win, min_delay_win, max_delay_win);
        println!("{:?}", h_0);

        let shift_min = -10 / win * win;
        let shift_max = 10 / win * win;

        for d in (shift_min..=shift_max).step_by(win as usize) {
            let tags_d: Vec<Tag> = tags
                .clone()
                .into_iter()
                .map(|t| {
                    if t.channel == ch_b {
                        Tag {
                            time: t.time + d,
                            channel: t.channel,
                        }
                    } else {
                        Tag {
                            time: t.time,
                            channel: t.channel,
                        }
                    }
                })
                .collect();
            let h_d = pat::coincidence_histogram(&tags_d, ch_a, ch_b, win, min_delay, max_delay);
            for i in ((min_delay_win + d.abs() + win)..=(max_delay_win - d.abs() - win))
                .step_by(win as usize)
            {
                assert_eq!(h_0.get(&i), h_d.get(&(i + d)));
            }
        }
    }
}

/// Compare set intersection algorithm against histogram
#[test]
fn coincidence_intersection_vs_histogram() {
    let tags = common::load_test_data();
    let ch_a: u8 = 3;
    let ch_b: u8 = 15;
    let win: i64 = 1;
    let min_delay: i64 = -64;
    let max_delay: i64 = 64;

    let mut cts = std::collections::BTreeMap::new();

    let histogram = pat::coincidence_histogram(&tags, ch_a, ch_b, win, min_delay, max_delay);

    for delay in min_delay..=max_delay {
        cts.insert(
            delay,
            pat::coincidence_intersection(&tags, ch_a, ch_b, win, delay),
        );
    }
    println!("{:?}", cts);
    println!("{:?}", histogram);

    assert_eq!(&cts, &histogram);
}

/// Compare set intersection algorithm against histogram for various window sizes
#[test]
fn coincidence_intersection_vs_histogram_win() {
    let tags = common::load_test_data();
    let ch_a: u8 = 3;
    let ch_b: u8 = 15;
    for win in [2, 3, 4] {
        let min_delay: i64 = -64 / win * win;
        let max_delay: i64 = 64 / win * win;

        let mut cts = std::collections::BTreeMap::new();

        let histogram = pat::coincidence_histogram(&tags, ch_a, ch_b, win, min_delay, max_delay);

        for delay in (min_delay..=max_delay).step_by(win as usize) {
            cts.insert(
                delay,
                pat::coincidence_intersection(&tags, ch_a, ch_b, win, delay)
            );
        }

        assert_eq!(&cts, &histogram);
    }
}

/// Compare set intersection algorithm against histogram with one channel's tags delayed
#[test]
fn coincidence_intersection_vs_histogram_delay_win() {
    let tags = common::load_test_data();

    let ch_a = 3;
    let ch_b = 15;
    let min_delay = -64;
    let max_delay = 64;

    for win in [1, 2, 3, 4] {
        let min_delay_win = min_delay / win;
        let max_delay_win = max_delay / win;

        let h_0 = pat::coincidence_histogram(&tags, ch_a, ch_b, win, min_delay_win, max_delay_win);

        let shift_min = -10 / win * win;
        let shift_max = 10 / win * win;

        for d in (shift_min..=shift_max).step_by(win as usize) {
            let tags_d: Vec<Tag> = tags
                .clone()
                .into_iter()
                .map(|t| {
                    if t.channel == ch_b {
                        Tag {
                            time: t.time + d,
                            channel: t.channel,
                        }
                    } else {
                        Tag {
                            time: t.time,
                            channel: t.channel,
                        }
                    }
                })
                .collect();
            for i in ((min_delay_win + d.abs() + win)..=(max_delay_win - d.abs() - win))
                .step_by(win as usize)
            {
                assert_eq!(
                    h_0.get(&i).unwrap(),
                    &pat::coincidence_intersection(&tags_d, ch_a, ch_b, win, i + d)
                );
            }
        }
    }
}
