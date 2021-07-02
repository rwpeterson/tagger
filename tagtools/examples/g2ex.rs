use tagtools::{de, pat, Bin};
use std::env;
use std::fs::File;
use std::vec::Vec;
use std::time::{Instant};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let tagfile = &args[1];
    let hstfile = &args[2];
    let g2file = &args[3];

    let mut tagrdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b'\t')
        .from_reader(
            File::open(tagfile)?
        );
    let mut hstrdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b'\t')
        .from_reader(
            File::open(hstfile)?
        );
    let mut g2rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b'\t')
        .from_reader(
            File::open(g2file)?
        );
    
    if let (Ok(tags), Ok(hstchk), Ok(g2chk)) = (
        de::tsv(&mut tagrdr),
        de::histogram_tsv::<File, f64, usize>(&mut hstrdr),
        de::histogram_tsv::<File, f64, f64>(&mut g2rdr),
    ) {
        let tstep: f64 = 156.25e-3; // in ns
        let win: i64 = 1;
        let ch_a: u8 = 3;
        let ch_b: u8 = 15;
        let min_delay: i64 = -64; // -10 ns
        let max_delay: i64 = 64; //  10 ns

        let mut t0 = Instant::now();
        let hst = pat::coincidence_histogram(&tags, win, ch_a, ch_b, min_delay, max_delay);
        println!("histogram time: {}", t0.elapsed().as_millis());
        t0 = Instant::now();
        let g2 = pat::g2(&tags, win, ch_a, ch_b, min_delay, max_delay);
        println!("g2 time: {}", t0.elapsed().as_millis());

        let mut hstbin: Vec<Bin<f64, usize>> = Vec::new();
        for (i, &coinc) in hst.iter().enumerate() {
            hstbin.push(Bin {
                x: (min_delay + i as i64 * win) as f64 * tstep,
                y: coinc,
            });
        }

        let mut g2bin: Vec<Bin<f64, f64>> = Vec::new();
        for (i, &g2val) in g2.iter().enumerate() {
            g2bin.push(Bin {
                x: (min_delay + i as i64 * win) as f64 * tstep,
                y: g2val,
            })
        }

        // Now to check
        let time_epsilon = 1e-4; // 0.1 ps
        let g2_epsilon = 1e-4;
        // Skip the first and last values since they are zero for the reference code
        let hstbin_ = &hstbin[1..(&hstbin.len() - 1)];
        let hstchk_ = &hstchk[1..(&hstchk.len() - 1)];
        let g2bin_ = &g2bin[1..(&g2bin.len() - 1)];
        let g2chk_ = &g2chk[1..(&g2chk.len() - 1)];
        for (&me, &lee) in hstbin_.iter().zip(hstchk_.iter()) {
            if me.x - lee.x > time_epsilon || me.y != lee.y {
                println!(
                    "ERROR: {0}, {1} does not match {2}, {3}",
                    me.x, me.y, lee.x, lee.y
                );
            }
        }
        for (&me, &lee) in g2bin_.iter().zip(g2chk_.iter()) {
            if me.x - lee.x > time_epsilon || me.y - lee.y > g2_epsilon {
                println!(
                    "ERROR: {0}, {1} does not match {2}, {3}",
                    me.x, me.y, lee.x, lee.y
                );
            }
        }
    }
    Ok(())
}
