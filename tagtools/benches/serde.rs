#[allow(unused_imports)]
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

use tagtools::{de, ser};

mod common;

fn serialize_deserialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("Serialize/Deserialize");
    let tags = common::load_test_data_short();
    let mut buf: Vec<u8> = Vec::new();

    for level in [-5, -3, -1, 1, 3, 5,] {
        group.bench_with_input(
            BenchmarkId::new("packed", level),
            &level,
            |b, level| {
                b.iter(|| {
                    buf.clear();
                    ser::tags_bench(&mut buf, &tags, black_box(true), *level).unwrap();
                    let _ = black_box(de::tags_bench(&*buf, true));
                });
            }
        );
        group.bench_with_input(
            BenchmarkId::new("unpacked", level),
            &level,
            |b, level| {
                b.iter(|| {
                    buf.clear();
                    ser::tags_bench(&mut buf, &tags, black_box(false), *level).unwrap();
                    let _ = black_box(de::tags_bench(&*buf, false));
                });
            }
        );
    }
}

criterion_group!(benches, serialize_deserialize);

criterion_main!(benches);
