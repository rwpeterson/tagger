pub mod controller;
pub mod copier;
pub mod data;
pub mod processor;
pub mod server;
pub mod timer;

use argh::FromArgs;
#[derive(Debug, FromArgs, Clone)]
/// cli app args

pub struct CliArgs {
    /// print version information
    #[argh(switch, short = 'v')]
    pub version: bool,
    /// server address
    #[argh(option, default = "String::from(\"127.0.0.1:6969\")")]
    pub addr: String,
}

pub enum Event {
    Tick,
    Set(InputSetting),
}

pub enum InputSetting {
    InversionMask(u16),
    Delay((u8, u32)),
    Threshold((u8, f64)),
}