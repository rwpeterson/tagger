use std::time::Duration;
use crate::Event;

pub fn main(sender: flume::Sender<Event>) -> anyhow::Result<()> {
    let dur = Duration::from_micros(10000);
    std::thread::spawn(move || {
        while let Ok(()) = sender.send(Event::Tick) {
            std::thread::sleep(dur);
        }
    });
    Ok(())
}