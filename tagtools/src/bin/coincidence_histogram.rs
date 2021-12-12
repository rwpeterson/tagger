use tagtools::{de, pat};

use anyhow::{Result};
use std::fs::File;
use std::io::{BufReader, stdout};

#[derive(Debug, argh::FromArgs, Clone)]
/// cli app args
pub struct CliArgs {
    /// tags file path
    #[argh(positional)]
    pub tags: String,
    /// window size
    #[argh(option, default = "1")]
    pub win: i64,
    /// channel a
    #[argh(option, default = "1")]
    pub ch_a: u8,
    /// channel b
    #[argh(option, default = "2")]
    pub ch_b: u8,
    /// minimum delay
    #[argh(option, default = "-10")]
    pub min: i64,
    /// minimum delay
    #[argh(option, default = "10")]
    pub max: i64,
}

fn main() -> Result<()> {

    let config: CliArgs = argh::from_env();

    let file = config.tags;
    let mut rdr = BufReader::new(File::open(file)?);
    let tags = de::tags(&mut rdr)?;

    let histogram = pat::coincidence_histogram(
        &tags,
        config.ch_a,
        config.ch_b,
        config.win,
        config.min,
        config.max,
    );

    let stdout = stdout();
    let stdout = stdout.lock();
    let mut wtr = csv::WriterBuilder::new()
                .has_headers(false)
                .delimiter(b'\t')
                .from_writer(stdout);

    for (d, c) in histogram {
        wtr.write_record(&[d.to_string(), c.to_string()])?;
    }
    wtr.flush()?;
    Ok(())
}