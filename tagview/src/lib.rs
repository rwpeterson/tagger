pub mod app;
pub mod client;
pub mod save;
pub mod timer;
pub mod ui;

use argh::FromArgs;

#[derive(Debug, FromArgs, Clone)]
/// cli app args
pub struct Cli {
    /// tick period in ms
    #[argh(option, default = "250")]
    pub tick_rate: u64,
    /// use unicode graphics
    #[argh(option, default = "true")]
    pub enhanced_graphics: bool,
    /// print version information
    #[argh(switch, short = 'v')]
    pub version: bool,
    /// server address
    #[argh(option, default = "String::from(\"127.0.0.1:6969\")")]
    pub addr: String,
    /// config file path
    #[argh(option)]
    pub config: String,
}
