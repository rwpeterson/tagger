use anyhow::{bail, Result};
use tagtools::{CHAN16, Tag};
use timetag::error_text;
use timetag::ffi::{new_time_tagger, FfiTag};

#[allow(unused_imports)]
use tracing::{debug, error, info, span, warn, Instrument, Level};

use crate::{Event, InputSetting};

/// Create and manage time tagger, providing data to the server thread
pub fn main(
    receiver_timer: flume::Receiver<Event>,
    receiver_event: flume::Receiver<Event>,
    sender: flume::Sender<(Vec<Tag>, u64)>,
) -> Result<()> {
    let span = span!(Level::WARN, "controller");
    let _enter = span.enter();

    let tt = new_time_tagger();
    tt.open();
    for ch in CHAN16 {
        tt.set_input_threshold(ch, 2.0);
    }
    // TODO: reduce set_fg to a command-line debug flag
    tt.set_fg(200_000, 100_000);
    tt.start_timetags();
    tt.freeze_single_counter();
    loop {
        let status: Result<()> = flume::Selector::new()
            .recv(&receiver_timer, |r| match r {
                Err(e) => bail!(e),
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
                        error!("tag {:?}: {:?}", &tags.get(0).and_then(|t| Some(t.time)).unwrap_or(0), error_text(flags));
                    }

                    sender.send((tags, dur))?;
                    Ok(())
                }
            })
            .recv(&receiver_event, |r| {
                match r {
                    Err(e) => bail!(e),
                    Ok(Event::Tick) => {}
                    Ok(Event::Set(s)) => match s {
                        InputSetting::InversionMask(m) => tt.set_inversion_mask(m),
                        InputSetting::Delay((ch, del)) => tt.set_delay(ch, del),
                        InputSetting::Threshold((ch, th)) => tt.set_input_threshold(ch, th),
                    },
                }
                Ok(())
            })
            .wait();
        if let Err(_) = status {
            break;
        }
    }
    tt.stop_timetags();
    tt.close();
    Ok(())
}
