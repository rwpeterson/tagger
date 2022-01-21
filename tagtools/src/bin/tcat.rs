use argh::FromArgs;
use anyhow::{bail, Result};
use either::{Left, Right};
use std::fs::{self, File};
use std::io::{stdin, stdout, BufReader, Write};

use tagtools::{ser, de};

const GIT_VERSION: &str = git_version::git_version!();

#[derive(Debug, FromArgs, Clone)]
/// Decode time tags in .tags.zst compressed binary format and print
/// tab-separated tags to standard output. tcat is named in analogy
/// to programs like zcat(1) that output the decompressed content of
/// file(s).
pub struct CliArgs {
    /// print version information
    #[argh(switch, short = 'v')]
    pub version: bool,
    /// with no input or when input is '-', read from standard input
    #[argh(positional)]
    pub input: Vec<String>,
}

fn main() -> Result<()> {
    let args: CliArgs = argh::from_env();
    if args.version {
        let stdout = stdout();
        let mut stdout = stdout.lock();
        writeln!(
            stdout,
            concat!(
                env!("CARGO_BIN_NAME"),
                " ",
                "{}",
            ),
            GIT_VERSION,
        )?;
        return Ok(())
    }

    // Collect inputs
    let mut inputs = Vec::new();
    if args.input.len() == 0 {
        inputs.push(Left(()));
    } else {
        let mut contains_stdin = false;
        for i in args.input {
            if i == "-" {
                if contains_stdin {
                    panic!("cannot specify '-' for stdin twice");
                } else {
                    contains_stdin = true;
                    inputs.push(Left(()));
                }
            } else {
                match fs::metadata(&i) {
                    Ok(m) => {
                        if m.is_file() {
                            inputs.push(Right(i));
                        } else {
                            bail!("{} is not a file", &i);
                        }
                    },
                    Err(e) => bail!(e),
                }
            }
        }
    }

    let stdout = stdout();
    let stdout = stdout.lock();
    let mut wtr = csv::WriterBuilder::new()
        .has_headers(false)
        .delimiter(b'\t')
        .from_writer(stdout);

    for i in inputs {
        match i {
            Left(()) => {
                let stdin = stdin();
                let stdin = stdin.lock();
                let rdr = BufReader::new(stdin);
                let tags = de::tags(rdr).expect("Cannot deserialize tags from file");
                ser::tsv(&mut wtr, &tags)?;
            },
            Right(path) => {
                let f = File::open(path)?;
                let rdr = BufReader::new(f);
                let tags = de::tags(rdr).expect("Cannot deserialize tags from file");
                ser::tsv(&mut wtr, &tags)?;
            },
        }
    }
    Ok(())
}
