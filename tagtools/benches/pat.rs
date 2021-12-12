#[allow(unused_imports)]
use criterion::{black_box, criterion_group, criterion_main, Criterion};

use tagtools::pat;

mod common;

fn singles(c: &mut Criterion) {
    let tags = common::load_test_data();

    c.bench_function("singles", |b| {
        b.iter(|| {
            pat::singles(&tags, black_box(3));
        })
    });
}

fn coincidence_histogram_1(c: &mut Criterion) {
    let tags = common::load_test_data();

    c.bench_function("coincidence_histogram_1", |b| {
        b.iter(|| {
            pat::coincidence(&tags, black_box(3), 15, 1, 26);
        })
    });
}

fn coincidence_intersection(c: &mut Criterion) {
    let tags = common::load_test_data();

    c.bench_function("coincidence_intersection", |b| {
        b.iter(|| {
            pat::coincidence_intersection(&tags, black_box(3), 15, 1, 26);
        })
    });
}

criterion_group!(
    benches,
    singles,
    coincidence_histogram_1,
    coincidence_intersection
);
criterion_main!(benches);
