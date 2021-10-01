use anyhow::{bail, Result};
use chrono::Local;
use tagsave::CliArgs;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use std::io::Read;

use tagtools::{Tag, bit, cfg};

use tagsave::client::{ClientHandle, ClientMessage};
use tagsave::save::{SaveHandle, SaveMessage, SaveTags};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args: CliArgs = argh::from_env();

    // Load the run file
    let cfg_path = std::path::Path::new(&args.config);
    let mut f = std::fs::File::open(cfg_path)?;
    let mut s = String::new();
    f.read_to_string(&mut s)?;
    let config: cfg::Run = toml::de::from_str(&s)?;

    // Get tick rate
    let tick_rate = Duration::from_millis(args.tick_rate);

    // Start client thread, connect to server
    let client = ClientHandle::new(args);

    // Start save thread
    let save = SaveHandle::new();

    // Data structures to hold data during the run
    let xtags= Arc::new(Mutex::new(Vec::<Tag>::new()));
    let xpats = Arc::new(Mutex::new(HashMap::<u16,u64>::new()));
    let filepath: Option<std::path::PathBuf> = None;

    let mut duration = 0u64;
    let timestamp = Local::now();
    
    let first_tick = Instant::now();
    let mut last_tick = first_tick;
    loop {
        let (respond_to, response) = flume::bounded(1);
        let _ = client.sender.send(ClientMessage::GetData { respond_to });

        let mut tags = xtags.lock();
        let mut pats = xpats.lock();
        let newdata = response.recv().unwrap();
        //duration = 0;
        (*tags).clear();
        //(*pats).clear();
        if let Some(data) = newdata {
            for mut chunk in data {
                duration += chunk.tagpat.duration;
                (*tags).append(&mut chunk.tagpat.tags);
                for lpat in chunk.pats {
                    if let None = (*pats).get(&lpat.patmask) {
                        let _ = (*pats).insert(lpat.patmask, 0);
                    }
                    if let Some(v) = (*pats).get_mut(&lpat.patmask) {
                        *v += lpat.count;
                    }
                }
            }
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
                match (*pats).get(&bit::chans_to_mask(&[ch])) {
                    Some(&cts) => {
                        if cts > limit {
                            break
                        }
                    },
                    None => bail!("Limit singles pattern not found"),
                }
            },
            Some(cfg::RunLimit::CoincidenceLimit(ch_a, ch_b, limit)) => {
                match (*pats).get(&bit::chans_to_mask(&[ch_a, ch_b])) {
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
        channel_settings:   Vec::new(), // FIXME: actually deal with these
        ..config
    };
    let pats = xpats.lock();
    for (pat, cts) in pats.clone() {
        if let Some(ch) = bit::mask_to_single(pat) {
            record.singles.push(cfg::Single::ChannelCounts((ch, cts)));
        }
        if let Some((ch_a, ch_b)) = bit::mask_to_pair(pat) {
            record.coincidences.push(cfg::Coincidence::ChannelsCounts((ch_a, ch_b, cts)));
        }
    }
    if config.save_tags == Some(cfg::SaveTags::Save(true)) {
        record.save_tags = Some(cfg::SaveTags::TagFile(filepath.unwrap()));
    }


    Ok(())
}
