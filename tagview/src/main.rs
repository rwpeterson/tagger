#[allow(unused_imports)]
use anyhow::{Context, Result};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::fs::File;
use std::io::{stdout, BufReader, Write};
use std::net::ToSocketAddrs;
use std::path::PathBuf;
use std::time::Duration;
use tagtools::cfg;
use tagview::{
    app::{App, Event},
    client::ClientHandle,
    save::SaveHandle,
    settings_client::SettingsClientHandle,
    timer::TimerHandle,
    ui, Cli,
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
///   and polls for keyboard input which it passes to the main thread.
/// - Saving tags to disk in a dedicated thread that holds the current file.
///   File IO is blocking, so this must be separate.
/// - Cap'n Proto RPC using tokio. This has several tasks running concurrently,
///   including making the TCP connection with the server, sending/receiving
///   RPCs, and managing the data to be passed to the main thread
/// - Second tokio runtime to manage channel configuration get/set requests,
///   since I couldn't figure out how to incorporate this into the first
///   without creating deadlocks.
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
        let stdout = std::io::stdout();
        let mut stdout = stdout.lock();
        writeln!(
            stdout,
            concat!(env!("CARGO_BIN_NAME"), " ", "{}",),
            GIT_VERSION,
        )?;
        return Ok(());
    }

    let addr = args
        .addr
        .to_socket_addrs()
        .unwrap()
        .next()
        .expect("could not parse address");

    // Process the config for subscription
    let path = PathBuf::from(&args.config.unwrap_or("runfile_example.json".to_owned()));
    let f = File::open(path)?;
    let rdr = BufReader::new(f);
    let config: cfg::Run = serde_json::from_reader(rdr)?;

    // Async runtime for Cap'n Proto RPC to receive data
    let client_handle = ClientHandle::new(addr, config.clone());

    // Second async runtime for Cap'n Proto RPC to process channel settings
    let settings_handle = SettingsClientHandle::new(addr);

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
        settings_handle,
        save_handle,
        config,
    );
    terminal.clear()?;
    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;
        match timer_handle.receiver.recv()? {
            Event::Input(event) => match event {
                KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                } => {
                    disable_raw_mode()?;
                    execute!(
                        terminal.backend_mut(),
                        LeaveAlternateScreen,
                        DisableMouseCapture,
                    )?;
                    terminal.show_cursor()?;
                    break;
                }
                KeyEvent {
                    code: KeyCode::Char('r'),
                    modifiers: KeyModifiers::CONTROL,
                } => {
                    app.on_ctrlr();
                }
                KeyEvent {
                    code: KeyCode::Char(char),
                    modifiers: _,
                } => {
                    app.on_key(char);
                }
                KeyEvent {
                    code: KeyCode::Left,
                    modifiers: _,
                } => {
                    app.on_left();
                }
                KeyEvent {
                    code: KeyCode::Right,
                    modifiers: _,
                } => {
                    app.on_right();
                }
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
