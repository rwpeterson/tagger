use crate::Event;

pub fn main(period: std::time::Duration, tx: flume::Sender<Event>) -> anyhow::Result<()> {
    std::thread::spawn(move || {
        while let Ok(()) = tx.send(Event::Tick) {
            std::thread::sleep(period);
        }
    });
    Ok(())
}