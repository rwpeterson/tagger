#[allow(dead_code)]

use csv::ReaderBuilder;
use tagtools::de;
use std::env;
use std::fs;
use std::path;
use zstd::stream;

use tagtools::Tag;
use tagtools::Bin;

pub fn load_test_data() -> Vec<Tag> {
    let project_root = env::var("CARGO_MANIFEST_DIR").unwrap();
    let data_file = path::PathBuf::from(project_root)
        .join("tests/resources/testdata_500k.tsv.zst");
    let rdr = fs::File::open(data_file).unwrap();
    let zrdr = stream::read::Decoder::new(rdr).unwrap();
    let mut crdr = ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b'\t')
        .from_reader(zrdr);
    let tags = de::tsv(&mut crdr).unwrap();
    return tags;
}

pub fn load_coincidence_histogram() -> Vec<Bin<f64, u64>> {
    let project_root = env::var("CARGO_MANIFEST_DIR").unwrap();
    let data_file = path::PathBuf::from(project_root)
        .join("tests/resources/coincidence_histogram_500k.txt");
    let rdr = fs::File::open(data_file).unwrap();
    let mut crdr = ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b'\t')
        .from_reader(rdr);
    let hist: Vec<Bin<f64, u64>> = de::histogram_tsv(&mut crdr).unwrap();
    return hist;
}

pub fn load_g2_histogram() -> Vec<Bin<f64, f64>> {
    let project_root = env::var("CARGO_MANIFEST_DIR").unwrap();
    let data_file = path::PathBuf::from(project_root)
        .join("tests/resources/g2_histogram_500k.txt");
    let rdr = fs::File::open(data_file).unwrap();
    let mut crdr = ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b'\t')
        .from_reader(rdr);
    let hist: Vec<Bin<f64, f64>> = de::histogram_tsv(&mut crdr).unwrap();
    return hist;
}