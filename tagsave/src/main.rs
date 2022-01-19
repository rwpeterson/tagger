use anyhow::{bail, Result};
use chrono::Utc;
use indicatif::{ProgressBar, ProgressStyle};
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
        let stdout = std::io::stdout();
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

    // Timestamp to save under, reflect the beginning time rather than the end
    let ts = Utc::now();
    // This is the only cross-platform ISO 8601 compliant timestamp format
    let path_ts = ts.format("%Y%m%dT%H%M%SZ").to_string();

    // Start progress bar
    let total = 65536;
    let pb = ProgressBar::new(total);
    pb.set_style(ProgressStyle::default_bar()
        .template("{prefix:>12.cyan.bold} [{elapsed_precise}] [{bar:57}] ({eta}) {msg}")
        .progress_chars("=> ")
    );
    pb.set_prefix("Init");

    // Get tick rate
    let tick_rate = Duration::from_millis(args.tick_rate);

    // Tags file path
    let tags_desc = cfg_path
        .as_path()
        .file_stem()
        .unwrap_or_else(|| std::ffi::OsStr::new("data"))
        .to_string_lossy()
        .to_string();
    let mut tags_name = String::new();
    tags_name.push_str(&path_ts);
    tags_name.push('_');
    tags_name.push_str(&tags_desc);
    let mut tags_path = cfg_path.with_file_name(&tags_name);
    tags_path.set_extension("tags.zst");

    // Start save thread
    let save = SaveHandle::new(
        if config.save_tags == Some(cfg::SaveTags::Save(true)) {
            Some(tags_path.clone())
        } else {
            None
        }
    );

    // Data structures to hold data during the run
    let xtags= Arc::new(Mutex::new(Vec::<Tag>::new()));
    let xpats = Arc::new(Mutex::new(HashMap::<(u16, Option<u32>), u64>::new()));
    let filepath: Option<std::path::PathBuf> = Some(tags_path);

    let mut duration = 0u64;
    let timestamp = Utc::now();
    
    let first_tick = Instant::now();
    let mut last_tick = first_tick;

    // Start client thread, connect to server
    let client = ClientHandle::new(addr, config.clone());

    pb.set_prefix("Acquiring");

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
                pb.set_position(first_tick.elapsed().as_secs() * total / d.as_secs());
                if first_tick.elapsed() > d {
                    break
                }
            },
            Some(cfg::RunLimit::SinglesLimit(ch, limit)) => {
                match (*pats).get(&(bit::chans_to_mask(&[ch]), None)) {
                    Some(&cts) => {
                        pb.set_position(cts * total / limit);
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
                        pb.set_position(cts * total / limit);
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

    pb.set_prefix("Finishing");

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
        record.save_tags = Some(cfg::SaveTags::TagFile(filepath.clone().unwrap()));
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

    let rcd_desc = cfg_path
        .as_path()
        .file_stem()
        .unwrap_or_else(|| std::ffi::OsStr::new("data"))
        .to_string_lossy()
        .to_string();
    let mut rcd_name = String::new();
    rcd_name.push_str(&path_ts);
    rcd_name.push('_');
    rcd_name.push_str(&rcd_desc);
    let mut rcd_path = cfg_path.with_file_name(&rcd_name);
    rcd_path.set_extension("json");
    {
        let f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&rcd_path)
            .expect("Saving more than one file per second");
        let mut  wtr = BufWriter::new(f);
        wtr.write_all(json_record.as_bytes())?;

        save.sender.send(SaveMessage::Reset)?;
        
        pb.set_prefix("Saved");
        pb.println(    format!("Data saved to {}", rcd_path.file_name().unwrap().to_string_lossy()));
        if config.save_tags == Some(cfg::SaveTags::Save(true)) {
            pb.println(format!("Tags saved to {}", filepath.clone().unwrap().file_name().unwrap().to_string_lossy()));
        }
        pb.finish();
    }

    Ok(())
}
