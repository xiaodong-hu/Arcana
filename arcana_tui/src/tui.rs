use crossterm::{
    event::{
        DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io::{self, stdout};

/// Terminal wrapper that manages raw mode and alternate screen.
pub struct Tui {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    mouse_capture: bool,
}

impl Tui {
    /// Initialize the terminal (enter raw mode, alternate screen, mouse capture).
    /// Mouse capture is enabled so scroll events are received directly (not translated to Up/Down).
    /// Use Shift+mouse for native text selection in most terminals.
    pub fn new() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        let mut out = stdout();
        execute!(
            out,
            EnterAlternateScreen,
            EnableBracketedPaste,
            EnableMouseCapture
        )?;
        let _ = execute!(
            out,
            PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                    | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
                    | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
            )
        );
        let backend = CrosstermBackend::new(out);
        let terminal = Terminal::new(backend)?;
        Ok(Self {
            terminal,
            mouse_capture: true,
        })
    }

    /// Draw a frame.
    pub fn draw<F>(&mut self, f: F) -> io::Result<()>
    where
        F: FnOnce(&mut Frame),
    {
        self.terminal.draw(f)?;
        Ok(())
    }

    pub fn set_mouse_capture(&mut self, enabled: bool) -> io::Result<()> {
        if self.mouse_capture == enabled {
            return Ok(());
        }
        if enabled {
            execute!(self.terminal.backend_mut(), EnableMouseCapture)?;
        } else {
            execute!(self.terminal.backend_mut(), DisableMouseCapture)?;
        }
        self.mouse_capture = enabled;
        Ok(())
    }

    pub fn mouse_capture(&self) -> bool {
        self.mouse_capture
    }

    /// Suspend the TUI for running an external program (editor).
    pub fn suspend(&mut self) -> io::Result<()> {
        let _ = execute!(self.terminal.backend_mut(), PopKeyboardEnhancementFlags);
        execute!(
            self.terminal.backend_mut(),
            DisableMouseCapture,
            DisableBracketedPaste,
            LeaveAlternateScreen
        )?;
        self.mouse_capture = false;
        terminal::disable_raw_mode()?;
        Ok(())
    }

    /// Resume the TUI after an external program exits.
    pub fn resume(&mut self) -> io::Result<()> {
        terminal::enable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            EnterAlternateScreen,
            EnableBracketedPaste,
            EnableMouseCapture
        )?;
        self.mouse_capture = true;
        let _ = execute!(
            self.terminal.backend_mut(),
            PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                    | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
                    | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
            )
        );
        self.terminal.clear()?;
        Ok(())
    }

    /// Restore the terminal to its original state (for exit).
    pub fn restore(&mut self) -> io::Result<()> {
        let _ = execute!(self.terminal.backend_mut(), PopKeyboardEnhancementFlags);
        terminal::disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            DisableMouseCapture,
            DisableBracketedPaste,
            LeaveAlternateScreen
        )?;
        self.mouse_capture = false;
        Ok(())
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}
