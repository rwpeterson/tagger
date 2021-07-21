use anyhow::Result;
use cxx::{CxxVector, UniquePtr};
use tagtools::Tag;
use timetag::ffi::FfiTag;

pub fn main(
    receiver: flume::Receiver<(UniquePtr<CxxVector<FfiTag>>, u64)>,
    sender: flume::Sender<(Vec<Tag>, u64)>,
) -> Result<()> {
    std::thread::spawn(move || loop {
        match receiver.recv() {
            Ok((ftags, dur)) => {
                sender.send((
                    ftags
                        .iter()
                        .map(|ft: &FfiTag| Tag {
                            time: ft.time,
                            channel: ft.channel,
                        })
                        .collect(),
                    dur,
                )).unwrap();
            }
            Err(_) => break,
        }
    });
    Ok(())
}
