#[macro_use]
extern crate bencher;

use bencher::Bencher;
use tagtools::pat;

mod common;

fn singles(bench: &mut Bencher) {
    let tags = common::load_test_data();
    bench.iter( || {
        pat::singles(&tags, 3);
    });
}

fn coincidences(bench: &mut Bencher) {
    let tags = common::load_test_data();
    bench.iter( || {
        pat::coincidence(&tags, 3, 15, 1, 26);
    });
}

benchmark_group!(benches, singles, coincidences);
benchmark_main!(benches);