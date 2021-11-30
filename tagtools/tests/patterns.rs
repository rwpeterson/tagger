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
fn coincidence_histogram() {
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
        assert_eq!(proof.y, histogram[i]);
    }
}

#[test]
fn g2_histogram() {
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
        assert!((proof.y - g2[i]).abs() < 3e-4); // all but x = 3.75, 3.9063, 4.0625 are < 1e-4
    }
}

#[test]
fn coincidence() {
    let tags = common::load_test_data();
    assert_eq!(76, pat::coincidence(&tags, 3, 15, 1, 26));
}
