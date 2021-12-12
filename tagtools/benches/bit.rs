#[allow(unused_imports)]
use criterion::{black_box, criterion_group, criterion_main, Criterion};

use tagtools::bit::BitOps;

fn trait_u8(c: &mut Criterion) {
    c.bench_function("trait_u8", |b| {
        b.iter(|| {
            let i = 0u8;
            let b = 0;
            let mut x = i;

            x.set(b);
            x.clear(b);
            x.toggle(b);
            x.toggle(b);
            let _ = black_box(x);
        })
    });
}

fn prim_u8(c: &mut Criterion) {
    c.bench_function("prim_u8", |b| {
        b.iter(|| {
            let i = 0u8;
            let b = 0;
            let mut x = i;
            x |= 1 << b;
            x &= !(1 << b);
            x ^= 1 << b;
            x ^= 1 << b;
            let _ = black_box(x);
        })
    });
}

fn trait_u64(c: &mut Criterion) {
    c.bench_function("trait_u64", |b| {
        b.iter(|| {
            let i = 0u64;
            let b = 0;
            let mut x = i;

            x.set(b);
            x.clear(b);
            x.toggle(b);
            x.toggle(b);
            let _ = black_box(x);
        })
    });
}

fn prim_u64(c: &mut Criterion) {
    c.bench_function("prim_u64", |b| {
        b.iter(|| {
            let i = 0u64;
            let b = 0;
            let mut x = i;
            x |= 1 << b;
            x &= !(1 << b);
            x ^= 1 << b;
            x ^= 1 << b;
            let _ = black_box(x);
        })
    });
}

criterion_group!(benches, trait_u8, prim_u8, trait_u64, prim_u64);
criterion_main!(benches);
