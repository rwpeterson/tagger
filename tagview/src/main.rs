#[allow(unused_imports)]
use anyhow::{Context, Result};
use argh::FromArgs;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::stdout;
use std::thread;
use std::time::{Duration, Instant};
use tagview::{app::{App, Event}, client, save, ui};
use tui::{backend::CrosstermBackend, Terminal};

#[derive(Debug, FromArgs)]
/// cli app args
struct Cli {
    /// tick period in ms
    #[argh(option, default = "1000")]
    tick_rate: u64,
    /// use unicode graphics
    #[argh(option, default = "true")]
    enhanced_graphics: bool,
}

/// Structure of `main`
/// 
/// ## Threads and async
/// 
/// We need to manage several tasks:
/// - UI event loop in a dedicated thread
/// - File IO in a dedicated thread that holds the current file
/// - Cap'n Proto RPC using tokio
///
/// For an overview of when to use dedicated threads, rayon, or tokio::spawn_blocking for blocking
/// code, see [a tokio maintainer's blog](https://ryhl.io/blog/async-what-is-blocking/). Basically,
/// async code should always be `.await`ing, and blocking code needs to be dealt with between tokio,
/// rayon, and std::thread according to the degree it's CPU-bound, or intended to
/// run forever.
///
/// For an overview of how to communicate between async and sync code, see the
/// [`tokio::sync::mpsc` docs](https://docs.rs/tokio/1.7.0/tokio/sync/mpsc/index.html) on the
/// correct choice of channel.
///
///
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let cli: Cli = argh::from_env();

    
    let (tx_event, rx_event) = flume::unbounded();
    let (tx_io, rx_io) = flume::unbounded();
    let (tx_client, rx_client) = flume::unbounded();

    // Event thread - forwards input events and sends ticks
    let tick_rate = Duration::from_millis(cli.tick_rate);
    let tx_event_c = tx_event.clone();


    // IO thread
    save::main(rx_io);

    // Client thread - runs async runtime for Cap'n Proto RPC
    client::main().await
}