use std::fmt;

use vte::Parser;

use super::TerminalGrid;

/// Owns a VTE parser plus terminal grid. Feed websocket output here, then render
/// `grid()` from the UI. Kept independent from the live transport for easy root
/// integration in a later phase.
pub struct PtyState {
    parser: Parser,
    grid: TerminalGrid,
}

impl fmt::Debug for PtyState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PtyState")
            .field("grid", &self.grid)
            .finish_non_exhaustive()
    }
}

impl Clone for PtyState {
    fn clone(&self) -> Self {
        Self {
            // vte::Parser does not expose cloneable state. Starting a fresh parser is fine for
            // cloned render snapshots because callers clone PtyState to inspect the grid, not to
            // resume an in-flight escape sequence in the middle of parsing.
            parser: Parser::new(),
            grid: self.grid.clone(),
        }
    }
}

impl PtyState {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            parser: Parser::new(),
            grid: TerminalGrid::new(rows, cols),
        }
    }

    pub fn grid(&self) -> &TerminalGrid {
        &self.grid
    }
    pub fn grid_mut(&mut self) -> &mut TerminalGrid {
        &mut self.grid
    }

    pub fn feed(&mut self, bytes: impl AsRef<[u8]>) {
        for byte in bytes.as_ref() {
            self.parser.advance(&mut self.grid, *byte);
        }
    }

    pub fn resize(&mut self, rows: usize, cols: usize) {
        self.grid.resize(rows, cols);
    }
}

impl Default for PtyState {
    fn default() -> Self {
        Self::new(24, 80)
    }
}
