use argh::FromArgs;

#[derive(Debug, FromArgs, Clone)]
/// cli app args
pub struct CliArgs {
    /// print version information
    #[argh(switch, short = 'v')]
    pub version: bool,
    /// tick period in ms
    #[argh(option, default = "250")]
    pub tick_rate: u64,
    /// server address
    #[argh(option, default = "String::from(\"127.0.0.1:6969\")")]
    pub addr: String,
    /// config file path
    #[argh(option)]
    pub config: Option<String>,
}

pub mod client;
pub mod save;