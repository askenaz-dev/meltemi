// SPDX-License-Identifier: Apache-2.0

//! RAII terminal guard (design D9): enters raw mode + alternate screen and
//! guarantees restoration on normal exit, on an error return, and on panic
//! (unwinding runs `Drop`). Rendering never leaves the terminal wedged.

use std::io;

use ratatui::DefaultTerminal;

/// Owns the terminal setup; restores it when dropped.
pub struct TerminalGuard {
    terminal: DefaultTerminal,
}

impl TerminalGuard {
    /// Enters raw mode and the alternate screen, returning the guard.
    pub fn enter() -> io::Result<Self> {
        let terminal = ratatui::try_init()?;
        Ok(Self { terminal })
    }

    /// The underlying terminal, for drawing.
    pub fn terminal(&mut self) -> &mut DefaultTerminal {
        &mut self.terminal
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Restore raw mode / alternate screen. Best-effort: nothing useful to do
        // if this fails while the process is tearing down.
        ratatui::restore();
    }
}
