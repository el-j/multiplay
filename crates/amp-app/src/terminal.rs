//! Terminal lifecycle and raw mode management with RAII cleanup guard.

#![forbid(unsafe_code)]

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{stdout, Stdout};

/// RAII guard ensuring terminal raw mode and alternate screen are restored on drop.
pub struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalGuard {
    /// Initializes terminal into raw mode and alternate screen buffer.
    pub fn init() -> Result<Self, String> {
        enable_raw_mode().map_err(|e| format!("Failed to enable raw mode: {e}"))?;
        let mut stdout_handle = stdout();
        execute!(stdout_handle, EnterAlternateScreen)
            .map_err(|e| format!("Failed to enter alternate screen: {e}"))?;

        let backend = CrosstermBackend::new(stdout_handle);
        let mut terminal = Terminal::new(backend)
            .map_err(|e| format!("Failed to initialize terminal: {e}"))?;
        terminal
            .hide_cursor()
            .map_err(|e| format!("Failed to hide cursor: {e}"))?;

        Ok(Self { terminal })
    }

    /// Returns a mutable reference to the underlying Ratatui terminal.
    pub fn terminal_mut(&mut self) -> &mut Terminal<CrosstermBackend<Stdout>> {
        &mut self.terminal
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}
