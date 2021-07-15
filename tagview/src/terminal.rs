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
use crate::{app::{App, Event}, client, save, ui};
use tui::{backend::CrosstermBackend, Terminal};

pub fn main() -> Result<()> {
    enable_raw_mode()?;

    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);

    let mut terminal = Terminal::new(backend)?;


    // Main loop - blocks on receiving events for input and ticks
    let mut app = App::new(
        "Tag View",
        cli.enhanced_graphics,
        tx_io.clone(),
        tx_client.clone(),
    );
    terminal.clear()?;
    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;
        match rx_event.recv()? {
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
