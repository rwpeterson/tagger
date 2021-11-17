#[allow(unused_imports)]
use anyhow::{Context, Result};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::fs::File;
use std::io::{BufReader, stdout};
use std::net::ToSocketAddrs;
use std::path::PathBuf;
use std::time::Duration;
use tagtools::cfg;
use tagview::{
    Cli,
    app::{App, Event},
    client::ClientHandle,
    timer::TimerHandle,
    save::SaveHandle,
    ui,
};
use tui::{backend::CrosstermBackend, Terminal};

const GIT_VERSION: &str = git_version::git_version!();

/// Timetag visualization client
/// 
/// ## Structure
/// 
/// `tagview` needs to manage several threads:
/// - Main thread that responds to tick/input events and draws the UI
/// - Event loop in a dedicated thread. This sends ticks at regular intervals,
///   and polls for keyboard input which it passes along
/// - Saving tags to disk in a dedicated thread that holds the current file.
///   File IO is blocking, so this must be separate
/// - Cap'n Proto RPC using tokio. This has several tasks running concurrently,
///   including making the TCP connection with the server, sending/receiving
///   RPCs, and managing the data to be passed to the main thread
///
/// For an overview of when to use `std::thread`, `rayon`, or `tokio::spawn_blocking`
/// for blocking code, see [a tokio maintainer's blog](https://ryhl.io/blog/async-what-is-blocking/).
/// Basically, async code should always be `.await`ing, and blocking code needs 
/// to be dealt with between `std::thread`, `rayon`, and `tokio::spawn_blocking`
/// according to the degree it is persistent, CPU-bound, or meets other considerations.
///
/// For an overview of how to communicate between async and sync code, see the
/// [`tokio::sync::mpsc` docs](https://docs.rs/tokio/1.7.0/tokio/sync/mpsc/index.html) on the
/// correct choice of channel. tl;dr: channels should match the destination (async vs sync),
/// as (at least for unbounded channels), `.send()` is always non-blocking but `.recv()`
/// needs to block in sync code or return a future in async code. Because `std::sync::mpsc`
/// is relatively unpopular, it's worth noting that `flume` channels support sync and async
/// in both directions, making them very versitile.
///
///
fn main() -> Result<()> {
    let args: Cli = argh::from_env();

    if args.version {
        println!(
            concat!(
                env!("CARGO_BIN_NAME"),
                " ",
                "{}",
            ),
            GIT_VERSION,
        );
        return Ok(())
    }

    let addr = args.addr
        .to_socket_addrs()
        .unwrap()
        .next()
        .expect("could not parse address");

    // Process the config for subscription
    let path = PathBuf::from(&args.config);
    let f = File::open(path)?;
    let rdr = BufReader::new(f);
    let config: cfg::Run = serde_json::from_reader(rdr)?;
        

    // Client thread - runs async runtime for Cap'n Proto RPC
    let client_handle = ClientHandle::new(addr, config.clone());

    // Event thread - forwards input events and sends ticks
    let tick_rate = Duration::from_millis(args.tick_rate);
    let timer_handle = TimerHandle::new(tick_rate);

    // Disk IO thread
    let save_handle = SaveHandle::new();

    enable_raw_mode()?;

    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);

    let mut terminal = Terminal::new(backend)?;

    // Main loop - blocks on receiving events for input and ticks
    let mut app = App::new(
        "tagview",
        args.enhanced_graphics,
        client_handle,
        save_handle,
    );
    terminal.clear()?;
    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;
        match timer_handle.receiver.recv()? {
            Event::Input(event) => match event.code {
                KeyCode::Char('q') => {
                    disable_raw_mode()?;
                    execute!(
                        terminal.backend_mut(),
                        LeaveAlternateScreen,
                        DisableMouseCapture,
                    )?;
                    terminal.show_cursor()?;
                    break;
                }
                KeyCode::Char('r') => {
                    terminal.clear()?;
                }
                KeyCode::Char(c) => app.on_key(c),
                _ => {}
            },
            Event::Tick => {
                app.on_tick();
            }
        }
        if app.should_quit {
            break;
        }
    }
    Ok(())
}
