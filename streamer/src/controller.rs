use anyhow::Result;
use chrono::{SecondsFormat, Utc};
use cxx::{CxxVector, UniquePtr};
use std::time::Duration;
use tagtools::CHAN16;
use timetag::ffi::{new_time_tagger, FfiTag};
use timetag::error_text;

use crate::{Event, InputSetting};

/// Create and manage time tagger, providing data to the server thread
pub fn main(
    rx: flume::Receiver<Event>,
    tx: flume::Sender<(UniquePtr<CxxVector<FfiTag>>, u64)>,
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
        let tags: UniquePtr<CxxVector<FfiTag>> = tt.read_tags();
        let dur = tt.freeze_single_counter();

        let flags = tt.read_error_flags();
        if flags != 0 {
            let ts = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
            let t0 = match &tags.get(0) {
                Some(t) => format!("{}", t.time),
                None => String::from("[no tags]")
            };
            print!("{}\t{}", ts, t0);
            for error in error_text(flags) {
                print!("\t{}", error);
            }
            println!("");
        }
 
        tx.send((tags, dur))?;

        if !rx.is_empty() {
            match rx.recv_timeout(Duration::from_millis(1)) {
                Ok(Event::Set(s)) => match s {
                    InputSetting::InversionMask(m) => tt.set_inversion_mask(m),
                    InputSetting::Delay((ch, del)) => tt.set_delay(ch, del),
                    InputSetting::Threshold((ch, th)) => tt.set_input_threshold(ch, th),
                },
                Ok(Event::Tick) => {}
                Err(_) => {}
            }
        }
    }
    tt.stop_timetags();
    tt.close();
    Ok(())
}