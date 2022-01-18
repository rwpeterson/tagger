use anyhow::{bail, Result};
use either::Either;
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tagtools::{CHAN16, Tag};
use timetag::error_text;
use timetag::ffi::{new_time_tagger, new_logic_counter, FfiTag};

#[allow(unused_imports)]
use tracing::{debug, error, info, span, warn, Instrument, Level};

use crate::{CliArgs, Event, InputSetting};
use crate::data::{LogicData, RawData, RawTags, WIN_DEFAULT};

/// Create and manage time tagger, providing data to the server thread
pub fn main(
    args: CliArgs,
    receiver_timer: flume::Receiver<Event>,
    receiver_event: flume::Receiver<Event>,
    sender: flume::Sender<RawData>,
    _cur_tagmask: Arc<RwLock<u16>>,
    cur_patmasks: Arc<RwLock<HashSet<(u16, Option<u32>)>>>,
    global_window: Arc<RwLock<Option<u32>>>,
) -> Result<()> {
    let span = span!(Level::INFO, "controller");
    let _enter = span.enter();

    let tt = new_time_tagger();
    if let Err(_) = tt.open() {
        let span = span!(Level::ERROR, "connection");
        let _enter = span.enter();
        error!("Could not connect to time tagger");
        bail!("tagger connection failed");
    }
    info!("tagger connected");
    if args.calibrate {
        tt.calibrate();
        info!("calibration complete")
    }
    let fpga_version = tt.get_fpga_version();
    info!("FPGA gateware version {}", fpga_version);
    let resolution = tt.get_resolution();
    let mut rbuf = ryu::Buffer::new();
    let res = rbuf.format(resolution);
    info!("timing resolution {} sec", res);
    for ch in CHAN16 {
        tt.set_input_threshold(ch, 2.0);
    }
    let fg_enabled = args.fgperiod != 0 && args.fghigh != 0;
    if !args.logic {
        // Time tag mode (default)
        info!("timetag mode");
        if fg_enabled {
            tt.set_fg(args.fgperiod, args.fghigh);
            info!(
                "Output 4 function gen enabled: ({:e}, {:e}) sec",
                5e-9 * f64::from(args.fgperiod),
                5e-9 * f64::from(args.fghigh),
            );
        }
        tt.start_timetags();
        info!("timetag acquisition start");
        tt.freeze_single_counter();
        loop {
            let should_break: Result<bool> = flume::Selector::new()
                .recv(&receiver_timer, |r| match r {
                    Err(_) => return Ok(true),
                    Ok(_) => {
                        let dur = tt.freeze_single_counter();
                        let tags: Arc<Vec<Tag>> = Arc::new(tt
                            .read_tags()
                            .iter()
                            // BUG: does this map cause the discrepancy in times?
                            .map(|t: &FfiTag| Tag {
                                time: t.time,
                                channel: t.channel,
                            })
                            .collect());

                        let flags = tt.read_error_flags();
                        if flags != 0 {
                            let span = span!(Level::WARN, "controller");
                            let _enter = span.enter();
                            warn!("tag {:?}: {:?}", &tags.get(0).and_then(|t| Some(t.time)).unwrap_or(0), error_text(flags));
                        }

                        sender.send(Either::Left(RawTags {dur, tags}))?;
                        Ok(false)
                    }
                })
                .recv(&receiver_event, |r| {
                    match r {
                        Err(_) => return Ok(true),
                        Ok(Event::Tick) => {}
                        Ok(Event::Set(s)) => match s {
                            InputSetting::InversionMask(m) => tt.set_inversion_mask(m),
                            InputSetting::Delay((ch, del)) => tt.set_delay(ch, del),
                            InputSetting::Threshold((ch, th)) => tt.set_input_threshold(ch, th),
                            InputSetting::Window(_) => {}, // Ignore window in tag mode
                        },
                    }
                    Ok(false)
                })
                .wait();
            match should_break {
                Ok(true) => break,
                Ok(false) => continue,
                Err(_) => break,
            }
        }
        tt.stop_timetags();
        info!("timetag acquisition stop");
    } else {
        // Logic mode
        info!("logic mode");
        let lc = new_logic_counter(tt.clone());
        lc.switch_logic_mode();
        let gw = global_window.read();
        if args.window == 0 {
            let span = span!(Level::WARN, "global window");
            let _enter = span.enter();
            warn!("It is recommended to set an explicit fixed window size in logic mode with --window");
            warn!("Dynamic management of global window size is possible via RPC but requires care");
        }
        match *gw {
            Some(w) => {
                let span = span!(Level::INFO, "global window");
                let _enter = span.enter();
                lc.set_window_width(w);
                info!("set global window: {}", w);
            },
            None => {
                let span = span!(Level::ERROR, "global window");
                let _enter = span.enter();
                lc.set_window_width(WIN_DEFAULT);
                error!("global window state corrupted, set to default: {}", WIN_DEFAULT);
            }
        }
        drop(gw);
        if fg_enabled {
            lc.set_fg(args.fgperiod, args.fghigh);
            info!(
                "Output 4 function gen enabled: ({:e}, {:e}) sec",
                5e-9 * f64::from(args.fgperiod),
                5e-9 * f64::from(args.fghigh),
            );
        }
        lc.read_logic();
        loop {
            let should_break: Result<bool> = flume::Selector::new()
                .recv(&receiver_timer, |r| match r {
                    Err(_) => return Ok(true),
                    Ok(_) => {
                        lc.read_logic();
                        let dur = lc.get_time_counter();

                        let flags = lc.read_error_flags();
                        if flags != 0 {
                            let span = span!(Level::WARN, "controller");
                            let _enter = span.enter();
                            warn!("{:?}", error_text(flags));
                        }

                        let gw = global_window.read();
                        let w = (*gw).unwrap_or_default();

                        // In logic mode, we need to query the tagger for rates now,
                        // instead of working with tags later in a thread pool
                        let patmasks = cur_patmasks.read();
                        let mut counts = HashMap::new();
                        for (pat, _) in patmasks.iter() {
                            let c = lc.calc_count_pos(*pat) as u64;
                            counts.insert((*pat, Some(w)), c);
                        }

                        sender.send(Either::Right(LogicData {dur, counts}))?;
                        Ok(false)
                    }
                })
                .recv(&receiver_event, |r| {
                    match r {
                        Err(_) => return Ok(true),
                        Ok(Event::Tick) => {}
                        Ok(Event::Set(s)) => match s {
                            InputSetting::InversionMask(m) => lc.set_inversion_mask(m),
                            InputSetting::Delay((ch, del)) => lc.set_delay(ch, del),
                            InputSetting::Threshold((ch, th)) => lc.set_input_threshold(ch, th),
                            InputSetting::Window(w) => match args.window {
                                // Allow window change if not locked
                                0 => lc.set_window_width(w),
                                // Otherwise ignore
                                _ => {},
                            }
                        },
                    }
                    Ok(false)
                })
                .wait();
            match should_break {
                Ok(true) => break,
                Ok(false) => continue,
                Err(_) => break,
            }
        }
        tt.close();
        info!("tagger connection closed");
    }
    Ok(())
}
