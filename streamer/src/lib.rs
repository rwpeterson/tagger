pub mod controller;
pub mod data;
pub mod processor;
pub mod rpc;
pub mod server;
pub mod timer;

use argh::FromArgs;
#[derive(Debug, FromArgs, Clone)]
/// Controller for time tagger that serves data over a Cap'n Proto RPC API
pub struct CliArgs {
    /// print version information
    #[argh(switch, short = 'v')]
    pub version: bool,
    /// function generator period
    #[argh(option, default = "0")]
    pub fgperiod: u32,
    /// function generator high duration
    #[argh(option, default = "0")]
    pub fghigh: u32,
    /// calibrate time tagger
    #[argh(option, default = "false")]
    pub calibrate: bool,
    /// logic mode (highly recommended: fix global window with --window)
    #[argh(switch, short = 'l')]
    pub logic: bool,
    /// fixed global window for logic mode
    #[argh(option, short = 'w')]
    pub window: Option<u32>,
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
    Window(u32),
}
