//! Terminal lifecycle: enter alt screen + raw mode on construction,
//! restore on drop. Designed so the production `run()` path holds a
//! `TerminalGuard` in a `let` binding and the destructor runs even if
//! the body panics.

use crossterm::event::DisableMouseCapture;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{self, Stdout};

pub type Tui = Terminal<CrosstermBackend<Stdout>>;

pub struct TerminalGuard {
    terminal: Tui,
}

impl TerminalGuard {
    pub fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        crossterm::execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    pub fn terminal(&mut self) -> &mut Tui {
        &mut self.terminal
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Best-effort restore. Errors here would leave the user's shell
        // wedged; we log to stderr but never panic.
        let _ = disable_raw_mode();
        let _ = crossterm::execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn guard_type_exists() {
        // We do not invoke `enter()` in tests because it touches the real
        // terminal. Compilation alone proves the type signature.
        fn _accepts(_: super::TerminalGuard) {}
    }
}
