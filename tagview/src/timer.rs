use anyhow::Result;
use crossterm::event::{self, Event as CEvent, KeyEvent};
use std::time::{Duration, Instant};
use crate::app::Event;

pub fn main<I>(tx_event: flume::Sender<Event<I>>, tick_rate: Duration) -> Result<()> {
    let _ = std::thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            // poll for tick rate duration, if no events, send tick event
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));
            if event::poll(timeout).unwrap() {
                if let CEvent::Key(key) = event::read().unwrap() {
                    tx_event.send(Event::Input(key)).unwrap();
                }
            }
            if last_tick.elapsed() >= tick_rate {
                tx_event.send(Event::Tick).unwrap();
                last_tick = Instant::now();
            }
        }
    });
    Ok(())
}
