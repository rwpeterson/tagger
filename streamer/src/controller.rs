use anyhow::{bail, Result};
use tagtools::{CHAN16, Tag};
use timetag::error_text;
use timetag::ffi::{new_time_tagger, FfiTag};

#[allow(unused_imports)]
use tracing::{debug, error, info, span, warn, Instrument, Level};

use crate::{CliArgs, Event, InputSetting};

/// Create and manage time tagger, providing data to the server thread
pub fn main(
    args: CliArgs,
    receiver_timer: flume::Receiver<Event>,
    receiver_event: flume::Receiver<Event>,
    sender: flume::Sender<(Vec<Tag>, u64)>,
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
    for ch in CHAN16 {
        tt.set_input_threshold(ch, 2.0);
    }
    if args.fgperiod != 0 && args.fghigh != 0 {
        tt.set_fg(args.fgperiod, args.fghigh);
        info!(
            "function generator enabled ({:e}, {:e}) sec",
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
                    let tags: Vec<Tag> = tt
                        .read_tags()
                        .iter()
                        // BUG: does this map cause the discrepancy in times?
                        .map(|t: &FfiTag| Tag {
                            time: t.time,
                            channel: t.channel,
                        })
                        .collect();

                    let flags = tt.read_error_flags();
                    if flags != 0 {
                        let span = span!(Level::WARN, "controller");
                        let _enter = span.enter();
                        warn!("tag {:?}: {:?}", &tags.get(0).and_then(|t| Some(t.time)).unwrap_or(0), error_text(flags));
                    }

                    sender.send((tags, dur))?;
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
    tt.close();
    info!("tagger connection closed");
    Ok(())
}