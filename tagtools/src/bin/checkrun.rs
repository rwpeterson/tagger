//! `checkrun myrun.json`
//! 
//! Parse `myrun.json`. No output and an exit code of 0 indicates success.

use anyhow::Result;
use std::env;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use tagtools::cfg::Run;

fn main() -> Result<()> {
    let args = env::args().collect::<Vec<_>>();
    let path = PathBuf::from(&args[1]);
    let file = File::open(&path)?;
    let rdr = BufReader::new(file);
    let _run: Run = serde_json::from_reader(rdr)?;

    Ok(())
}