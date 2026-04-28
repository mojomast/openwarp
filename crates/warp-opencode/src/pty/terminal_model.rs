use super::TerminalBuffer;

#[derive(Debug, Clone)]
pub struct TerminalModel {
    pub rows: u16,
    pub cols: u16,
    pub cursor: Option<u64>,
    pub buffer: TerminalBuffer,
}

impl Default for TerminalModel {
    fn default() -> Self {
        Self {
            rows: 24,
            cols: 80,
            cursor: None,
            buffer: TerminalBuffer::new(2 * 1024 * 1024),
        }
    }
}

impl TerminalModel {
    pub fn resize(&mut self, rows: u16, cols: u16) {
        self.rows = rows;
        self.cols = cols;
    }
}
