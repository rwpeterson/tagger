use argh::FromArgs;
use anyhow::{bail, Result};
use either::{Left, Right};
use std::fs::{self, File};
use std::io::{stdin, stdout, BufReader, Write, BufWriter};

use tagtools::{ser, de};

const GIT_VERSION: &str = git_version::git_version!();

#[derive(Debug, FromArgs, Clone)]
/// Encode time tags stored as tab-separated values to the
/// .tags.zst compressed binary format. Note: on Windows
/// -o must be specified as the encoded data is not valid
/// UTF-8 and thus cannot be written to stdout (a Rust stdlib
/// limitation)
pub struct CliArgs {
    /// print version information
    #[argh(switch, short = 'v')]
    pub version: bool,
    /// file to write output to (writes to standard output by default)
    #[argh(option, short = 'o')]
    pub out: Option<String>,
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
    let mut wtr: Box<dyn Write> = match args.out {
        None => {
            Box::new(stdout.lock())
        },
        Some(p) => {
            let f = File::create(p)?;
            Box::new(BufWriter::new(f))
        },
    };

    for i in inputs {
        match i {
            Left(()) => {
                let stdin = stdin();
                let stdin = stdin.lock();
                let brdr = BufReader::new(stdin);
                let mut rdr = csv::ReaderBuilder::new()
                    .has_headers(false)
                    .delimiter(b'\t')
                    .from_reader(brdr);
                let tags = de::tsv(&mut rdr).expect("Cannot deserialize tags from file");
                ser::tags(&mut wtr, &tags)?;
            },
            Right(path) => {
                let f = File::open(path)?;
                let brdr = BufReader::new(f);
                let mut rdr = csv::ReaderBuilder::new()
                    .has_headers(false)
                    .delimiter(b'\t')
                    .from_reader(brdr);
                let tags = de::tsv(&mut rdr).expect("Cannot deserialize tags from file");
                ser::tags(&mut wtr, &tags)?;
            },
        }
    }
    Ok(())
}