//! `tcat [INPUT]`
//!
//! Decode time tags in .tags compressed binary format and output
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
use std::io::{stdin, stdout, BufReader, Read, Write};

fn main() -> Result<()> {
    let args = env::args().collect::<Vec<_>>();
    let stdin = stdin();
    let stdout = stdout();
    let mut rdr: BufReader<Box<dyn Read>>;
    let mut wtr: csv::Writer<Box<dyn Write>>;
    match args.len() - 1 {
        0 => {
            let iptr = Box::new(stdin.lock());
            rdr = BufReader::new(iptr);

            let optr = Box::new(stdout.lock());
            wtr = csv::WriterBuilder::new()
                .has_headers(false)
                .delimiter(b'\t')
                .from_writer(optr);
        },
        1 => {
            let file = &args[1];
            rdr = BufReader::new(Box::new(File::open(file)?));

            let optr = Box::new(stdout.lock());
            wtr = csv::WriterBuilder::new()
                .has_headers(false)
                .delimiter(b'\t')
                .from_writer(optr);
        },
        _ => return Err(anyhow!("Wrong number of arguments")),
    }
    let tags = de::tags(&mut rdr)?;
    ser::tsv(&mut wtr, &tags)?;
    Ok(())
}
