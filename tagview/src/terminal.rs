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
