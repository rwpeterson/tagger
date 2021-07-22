use anyhow::{bail, Result};
use chrono::{SecondsFormat, Utc};
use tagtools::Tag;
use tagtools::CHAN16;
use timetag::error_text;
use timetag::ffi::{new_time_tagger, FfiTag};

use crate::{Event, InputSetting};

/// Create and manage time tagger, providing data to the server thread
pub fn main(
    receiver_timer: flume::Receiver<Event>,
    receiver_event: flume::Receiver<Event>,
    sender: flume::Sender<(Vec<Tag>, u64)>,
) -> Result<()> {
    let tt = new_time_tagger();
    tt.open();
    for ch in CHAN16 {
        tt.set_input_threshold(ch, 2.0);
    }
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
                        .map(|t: &FfiTag| Tag {
                            time: t.time,
                            channel: t.channel,
                        })
                        .collect();

                    let flags = tt.read_error_flags();
                    if flags != 0 {
                        let ts = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
                        let t0 = match &tags.get(0) {
                            Some(t) => format!("{}", t.time),
                            None => String::from("[no tags]"),
                        };
                        print!("{}\t{}", ts, t0);
                        for error in error_text(flags) {
                            print!("\t{}", error);
                        }
                        println!("");
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
