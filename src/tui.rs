use std::io::{self, stdout, Stdout};
use std::ops::{Deref, DerefMut};

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

pub struct Tui {
    pub terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl Tui {
    pub fn new() -> io::Result<Self> {
        execute!(stdout(), EnterAlternateScreen, EnableMouseCapture)?;
        enable_raw_mode()?;
        let terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
        Ok(Self { terminal })
    }
}

impl Deref for Tui {
    type Target = Terminal<CrosstermBackend<Stdout>>;

    fn deref(&self) -> &Self::Target {
        &self.terminal
    }
}

impl DerefMut for Tui {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.terminal
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        let _ = restore();
    }
}

pub fn init() -> io::Result<Tui> {
    Tui::new()
}

// Ensure this is safe to call multiple times (idempotent-ish)
pub fn restore() -> io::Result<()> {
    // We ignore errors here because this might be called during panic
    // or when the terminal is already in a bad state.
    let _ = execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture);
    let _ = disable_raw_mode();
    Ok(())
}
