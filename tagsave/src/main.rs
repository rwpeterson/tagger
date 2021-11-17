use anyhow::{bail, Result};
use chrono::Local;
use tagsave::CliArgs;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::io::{BufReader, BufWriter, Write};
use tagtools::{CHAN16, Tag, bit, cfg};
use tagsave::client::{ClientHandle, ClientMessage};
use tagsave::save::{SaveHandle, SaveMessage, SaveTags};

const GIT_VERSION: &str = git_version::git_version!();

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args: CliArgs = argh::from_env();

    if args.version {
        println!(
            concat!(
                env!("CARGO_BIN_NAME"),
                " ",
                "{}",
            ),
            GIT_VERSION,
        );
        return Ok(())
    }

    // Load address
    let addr = args
        .addr
        .to_socket_addrs()
        .unwrap()
        .next()
        .expect("could not parse address");

    // Load the run file
    if let None = args.config {
        panic!("no runfile provided!");
    }
    let cfg_path;
    let config;
    match args.config {
        Some(c) => {
            cfg_path = std::path::PathBuf::from(c.clone());
            let f = File::open(cfg_path.as_path())?;
            let rdr = BufReader::new(f);
            config = serde_json::from_reader(rdr)?;
        },
        None => {
            cfg_path = std::path::PathBuf::from("data");
            config = cfg::Run::default();
        }
    };

    // Get tick rate
    let tick_rate = Duration::from_millis(args.tick_rate);

    // Start save thread
    let save = SaveHandle::new();

    // Data structures to hold data during the run
    let xtags= Arc::new(Mutex::new(Vec::<Tag>::new()));
    let xpats = Arc::new(Mutex::new(HashMap::<(u16, Option<u32>), u64>::new()));
    let filepath: Option<std::path::PathBuf> = None;

    let mut duration = 0u64;
    let timestamp = Local::now();
    
    let first_tick = Instant::now();
    let mut last_tick = first_tick;

    // Start client thread, connect to server
    let client = ClientHandle::new(addr, config.clone());

    loop {
        let (respond_to, response) = flume::bounded(1);
        let _ = client.sender.send(ClientMessage::GetData { respond_to });

        let mut tags = xtags.lock();
        let mut pats = xpats.lock();
        let newdata = response.recv().unwrap();
        //duration = 0;
        (*tags).clear();
        //(*pats).clear();
        match newdata {
            Some(data) => {
                for mut chunk in data {
                    duration += chunk.tagpat.duration;
                    (*tags).append(&mut chunk.tagpat.tags);
                    for lpat in chunk.pats {
                        if let None = (*pats).get(&(lpat.patmask, lpat.window)) {
                            let _ = (*pats).insert((lpat.patmask, lpat.window), 0);
                        }
                        if let Some(v) = (*pats).get_mut(&(lpat.patmask, lpat.window)) {
                            *v += lpat.count;
                        }
                    }
                }
            }
            None => {},
        }

        // Save tags to disk
        if config.save_tags == Some(cfg::SaveTags::Save(true)) {
            match save.sender.send(
                SaveMessage::Save(
                    SaveTags { tags: xtags.clone(), path: filepath.clone() }
                )
            )
            {
                Ok(()) => {},
                Err(_) => {},
            }
        }

        // Check if limit condition met and break
        match config.limit {
            Some(cfg::RunLimit::Duration(d)) => {
                if first_tick.elapsed() > d {
                    break
                }
            },
            Some(cfg::RunLimit::SinglesLimit(ch, limit)) => {
                match (*pats).get(&(bit::chans_to_mask(&[ch]), None)) {
                    Some(&cts) => {
                        if cts > limit {
                            break
                        }
                    },
                    None => bail!("Limit singles pattern not found"),
                }
            },
            Some(cfg::RunLimit::CoincidenceLimit(ch_a, ch_b, win, limit)) => {
                match (*pats).get(&(bit::chans_to_mask(&[ch_a, ch_b]), Some(win))) {
                    Some(&cts) => {
                        if cts > limit {
                            break
                        }
                    },
                    None => bail!("Limit coincidence pattern not found"),
                }
            },
            None => {},
        }

        // Sleep for the rest of tick rate
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        std::thread::sleep(timeout);
        last_tick = Instant::now();
    }

    client.sender.send(ClientMessage::Shutdown)?;

    let raw_settings = client.join_handle.join().unwrap()?;

    // Now record the run record to disk
    let mut record = cfg::Run{
        // name:            from declaration
        timestamp:          Some(timestamp),
        // limit:           from declaration
        // save_counts:     from declaration
        save_tags:          None,
        duration:           Some(duration),
        singles:            Vec::new(),
        coincidences:       Vec::new(),
        channel_settings:   Vec::new(),
        ..config
    };
    let pats = xpats.lock();
    for ((pat, win), cts) in pats.clone() {
        if let Some(ch) = bit::mask_to_single(pat) {
            record.singles.push(
                cfg::Single::ChannelCounts((ch, cts))
            );
        }
        if let Some((ch_a, ch_b)) = bit::mask_to_pair(pat) {
            record.coincidences.push(
                cfg::Coincidence::ChannelsCounts((ch_a, ch_b, win.unwrap_or_default(), cts))
            );
        }
    }
    if config.save_tags == Some(cfg::SaveTags::Save(true)) {
        record.save_tags = Some(cfg::SaveTags::TagFile(filepath.unwrap()));
    }
    for channel in CHAN16 {
        record.channel_settings.push(
            cfg::ChannelSettings {
                channel,
                invert: Some(bit::mask_to_chans(raw_settings.invm).contains(&channel)),
                delay: Some(raw_settings.dels[channel as usize - 1]),
                threshold: Some(raw_settings.thrs[channel as usize - 1]),
            }
        );
    }

    let json_record = serde_json::to_string_pretty(&record)?;

    let ts = Local::now();
    let mut rcd_stem = cfg_path
        .as_path()
        .file_stem()
        .unwrap_or_else(|| std::ffi::OsStr::new("data"))
        .to_string_lossy()
        .to_string();
    rcd_stem.push('_');
    let mut rcd_name = String::from(&rcd_stem);
    rcd_name.push_str(&ts.format("%F_%H-%M-%S").to_string());
    let mut rcd_name2 = String::from(&rcd_stem);
    rcd_name2.push_str(&ts.format("%F_%H-%M-%S%.3f").to_string());
    let mut rcd_path = cfg_path.with_file_name(rcd_name);
    let mut rcd_path2 = cfg_path.with_file_name(rcd_name2);
    rcd_path.set_extension("json");
    rcd_path2.set_extension("json");
    {
        let f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(rcd_path).unwrap_or_else( |_|
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(rcd_path2)
                    .expect("Saving more than one file per millisecond")
            );
        let mut  wtr = BufWriter::new(f);
        wtr.write_all(json_record.as_bytes())?;
    }

    Ok(())
}
