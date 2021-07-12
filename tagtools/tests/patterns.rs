use std::collections::HashMap;
use tagtools::pat;

mod common;

#[test]
fn singles() {
    let tags = common::load_test_data();
    assert_eq!(pat::singles(&tags, 3), 223_058);
    assert_eq!(pat::singles(&tags, 15), 251_662);
    assert_eq!(pat::singles(&tags, 16), 25_280);
}

#[test]
fn pattern_singles() {
    let tags = common::load_test_data();
    let mut proofs = HashMap::<u16, u64>::new();
    proofs.insert(1 << 2, 223_058);
    proofs.insert(1 << 14, 251_662);
    proofs.insert(1 << 15, 25_280);

    let delays = [0; 16];
    let pats = proofs.keys().cloned().collect::<Vec<_>>();
    let hashmap = pat::patterns(&tags, &pats, 1, delays);
    for (pat, cts) in proofs {
        assert_eq!(hashmap.get(&pat).unwrap(), &cts);
    }
}

#[test]
fn coincidence_histogram() {
    let tags = common::load_test_data();
    let proof_histogram = common::load_coincidence_histogram();
    let histogram = pat::coincidence_histogram(&tags, 1, 3, 15, -64, 64);

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
        assert_eq!(proof.y, histogram[i]);
    }
}

#[test]
fn pattern_coincidence_histogram() {
    let tags = common::load_test_data();
    let proof_histogram = common::load_coincidence_histogram();
    let mut histogram = Vec::new();
    let pat: u16 = (1 << 2) | (1 << 14);
    for delay in -64..=64 {
        let mut delays = [0; 16];
        let d = delays.get_mut(14).unwrap();
        *d = delay;
        let hm = pat::patterns(&tags, &vec![pat], 1, delays);
        let ct = hm.get(&pat).unwrap();
        histogram.push(*ct);
    }

    assert_eq!(proof_histogram.len(), histogram.len());
    for (i, proof) in proof_histogram.into_iter().enumerate() {
        // skip the first and last bins since they are zero in the reference calculation
        if i == 0 {
            continue;
        } else if i == 128 {
            continue;
        }
        println!("{0}\t{1}\t{2}", -64 + i as i64, proof.y, histogram[i]);
        let histogram_x = (-64. + i as f64) * 0.15625; // in ns
        assert!((proof.x - histogram_x).abs() < 1e-4);
        assert_eq!(proof.y, histogram[i]);
    }
}

#[test]
fn g2_histogram() {
    let tags = common::load_test_data();
    let proof_g2 = common::load_g2_histogram();
    let g2 = pat::g2(&tags, 1, 3, 15, -64, 64);
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
        assert!((proof.y - g2[i]).abs() < 3e-4); // all but x = 3.75, 3.9063, 4.0625 are < 1e-4
    }
}

#[test]
fn coincidence() {
    let tags = common::load_test_data();
    assert_eq!(76, pat::coincidence(&tags, 3, 15, 1, 26));
}
