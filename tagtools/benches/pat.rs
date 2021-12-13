#[allow(unused_imports)]
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

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

fn coincidences(c: &mut Criterion) {
    let tags = common::load_test_data();
    let mut group = c.benchmark_group("Coincidences");
    for i in (-64..=64i64).step_by(8) {
        group.bench_with_input(
            BenchmarkId::new("histogram", i),
            &i,
            |b, i| {
                b.iter(|| {
                    pat::coincidence_histogram_1(&tags, black_box(3), 15, 1, *i);
                });
            }
        );
        group.bench_with_input(
            BenchmarkId::new("intersection", i),
            &i,
            |b, i| {
                b.iter(|| {
                    pat::coincidence_intersection(&tags, black_box(3), 15, 1, *i);
                });
            }
        );
    }
}

criterion_group!(
    benches,
    singles,
    coincidences,
);
criterion_main!(benches);