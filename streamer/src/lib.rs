pub mod controller;
pub mod timer;
pub mod data;
pub mod server;
pub mod client;

pub enum Event {
    Tick,
    Set(InputSetting),
}

pub enum InputSetting {
    InversionMask(u16),
    Delay((u8, u32)),
    Threshold((u8, f64)),
}