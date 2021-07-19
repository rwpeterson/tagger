use crossterm::event::{self, Event as CEvent, KeyEvent};
use std::time::{Duration, Instant};
use crate::app::Event;

struct Timer<I> {
    tick_rate: Duration,
    last_tick: Instant,
    sender: flume::Sender<Event<I>>,
}

pub struct TimerHandle<I> {
    pub receiver: flume::Receiver<Event<I>>,
}

impl TimerHandle<KeyEvent> {
    pub fn new(tick_rate: Duration) -> Self {
        let (sender, receiver) = flume::unbounded();
        std::thread::spawn(move || {
            let mut timer = Timer { tick_rate, last_tick: Instant::now(), sender };
            loop {
                // poll for tick rate duration, if no events, send tick event
                let timeout = timer.tick_rate
                    .checked_sub(timer.last_tick.elapsed())
                    .unwrap_or_else(|| Duration::from_secs(0));
                if event::poll(timeout).unwrap() {
                    if let CEvent::Key(key) = event::read().unwrap() {
                        let _ = timer.sender.send(Event::Input(key));
                    }
                }
                if timer.last_tick.elapsed() >= timer.tick_rate {
                    let _ = timer.sender.send(Event::Tick);
                    timer.last_tick = Instant::now();
                }
            }
        });
        TimerHandle { receiver }
    }
}