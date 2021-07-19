#[allow(unused_imports)]
use anyhow::{Context, Result};
use argh::FromArgs;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::stdout;
use std::time::Duration;
use tagview::{app::{App, Event}, ui};
use tui::{backend::CrosstermBackend, Terminal};

use tagview::client::ClientHandle;
use tagview::timer::TimerHandle;
use tagview::save::SaveHandle;

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

fn main() -> Result<()> {
    let cli: Cli = argh::from_env();

    // Client thread - runs async runtime for Cap'n Proto RPC
    let client_handle = ClientHandle::new();

    // Event thread - forwards input events and sends ticks
    let tick_rate = Duration::from_millis(cli.tick_rate);
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
        cli.enhanced_graphics,
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
