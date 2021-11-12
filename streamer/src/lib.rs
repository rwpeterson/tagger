pub mod controller;
pub mod copier;
pub mod data;
pub mod processor;
pub mod server;
pub mod timer;

pub enum Event {
    Tick,
    Set(InputSetting),
}

pub enum InputSetting {
    InversionMask(u16),
    Delay((u8, u32)),
    Threshold((u8, f64)),
}