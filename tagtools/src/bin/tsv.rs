//! `tcat [INPUT]`
//!
//! Decode time tags in .tags.zst compressed binary format and output
//! tab-separated tags. `tcat` is named in analogy to programs like `zcat`
//! that output (cf. `cat`) the decompressed content of a file.
//!
//! Most likely, you want the shell one-liner
//!
//!     tcat mydata.tags.zst > mydata.tsv
//!
//! to convert the compressed binary format to tab-separated values for
//! the widest data interopability.

use tagtools::{ser, de};

use anyhow::{anyhow, Result};
use std::boxed::Box;
use std::env;
use std::fs::File;
use std::io::{stdin, stdout, Read, Write};

fn main() -> Result<()> {
    let args = env::args().collect::<Vec<_>>();
    let stdin = stdin();
    let stdout = stdout();
    let mut rdr: csv::Reader<Box<dyn Read>>;
    let mut wtr: Box<dyn Write>;
    match args.len() - 1 {
        0 => {
            let iptr = Box::new(stdin.lock());
            rdr = csv::ReaderBuilder::new()
                .has_headers(false)
                .delimiter(b'\t')
                .from_reader(iptr);

            let optr = Box::new(stdout.lock());
            wtr = optr;
        },
        1 => {
            let file = &args[1];
            let iptr = Box::new(File::open(file)?);
            rdr = csv::ReaderBuilder::new()
                .has_headers(false)
                .delimiter(b'\t')
                .from_reader(iptr);

            let optr = Box::new(stdout.lock());
            wtr = optr;
        },
        _ => return Err(anyhow!("Wrong number of arguments")),
    }
    let tags = de::tsv(&mut rdr)?;
    ser::tags(&mut wtr, &tags)?;
    Ok(())
}
