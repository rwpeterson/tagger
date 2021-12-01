#[allow(unused_imports)]
use criterion::{black_box, criterion_group, criterion_main, Criterion};

use tagtools::pat;

mod common;

fn singles(c: &mut Criterion) {
    let tags = common::load_test_data();
    
    c.bench_function("singles", |b| { b.iter( || {
        pat::singles(&tags, black_box(3));
    })});
}

fn coincidences(c: &mut Criterion) {
    let tags = common::load_test_data();
    
    c.bench_function("coincidences", |b| { b.iter( || {
        pat::coincidence(&tags, black_box(3), 15, 1, 26);
    })});
}

criterion_group!(benches, singles, coincidences);
criterion_main!(benches);