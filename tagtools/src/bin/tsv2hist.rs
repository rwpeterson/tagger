use std::env;
use std::error::Error;
use std::vec::Vec;
use tagtools::{de, pat};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let help = "tsv2hist.rs win ch_a ch_b min_delay max_delay [tags.tsv]";
    match args.len() - 1 {
        5 | 6 => {
            match (
                args[1].parse::<i64>(),
                args[2].parse::<u8>(),
                args[3].parse::<u8>(),
                args[4].parse::<i64>(),
                args[5].parse::<i64>(),
            ) {
                (Ok(win), Ok(ch_a), Ok(ch_b), Ok(min_delay), Ok(max_delay)) => {
                    let tags;
                    match args.get(6) {
                        // 6th arg is filename
                        Some(path) => {
                            let mut rdr = csv::ReaderBuilder::new()
                                .has_headers(false)
                                .delimiter(b'\t')
                                .from_path(path)?;
                            tags = de::tsv(&mut rdr)?
                        },
                        // otherwise take from stdin
                        None => {
                            let input = std::io::stdin();
                            let input = input.lock();
                            let mut rdr = csv::ReaderBuilder::new()
                                .has_headers(false)
                                .delimiter(b'\t')
                                .from_reader(input);
                            tags = de::tsv(&mut rdr)?;
                        },
                    }

                    let hst =
                        pat::coincidence_histogram(&tags, win, ch_a, ch_b, min_delay, max_delay);
                    for bin in hst.iter() {
                        println!("{}", bin);
                    }
                }
                // args are not understood
                (_, _, _, _, _) => {
                    println!("{}", help)
                }
            }
        }
        // wrong number of args provided
        _ => println!("{}", help),
    }
    Ok(())
}
