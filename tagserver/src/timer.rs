use anyhow::Result;
use std::time::{Duration, Instant};

use crate::{Config, Event};

pub fn main(
    cfg: Config,    
    tx: flume::Sender<Event>,
) -> Result<()> {
    let tick_rate = Duration::from_millis(cfg.rate as u64);
    let _ = std::thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            std::thread::sleep(
                tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0))
            );
            match tx.send(Event::Tick) {
                Ok(()) => last_tick = Instant::now(),
                Err(_) => break,
            }
        }
    });
    Ok(())
}