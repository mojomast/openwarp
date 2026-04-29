use std::cmp::{max, min};

use vte::{Params, Perform};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CellColor {
    #[default]
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub fg: CellColor,
    pub bg: CellColor,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub inverse: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: CellColor::Default,
            bg: CellColor::Default,
            bold: false,
            italic: false,
            underline: false,
            inverse: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedCell {
    pub row: usize,
    pub col: usize,
    pub cell: Cell,
}

#[derive(Debug, Clone)]
pub struct TerminalGrid {
    rows: usize,
    cols: usize,
    cells: Vec<Cell>,
    scrollback: Vec<Vec<Cell>>,
    cursor_row: usize,
    cursor_col: usize,
    current: Cell,
    max_scrollback: usize,
}

impl Default for TerminalGrid {
    fn default() -> Self {
        Self::new(24, 80)
    }
}

impl TerminalGrid {
    pub fn new(rows: usize, cols: usize) -> Self {
        let rows = max(rows, 1);
        let cols = max(cols, 1);
        Self {
            rows,
            cols,
            cells: vec![Cell::default(); rows * cols],
            scrollback: Vec::new(),
            cursor_row: 0,
            cursor_col: 0,
            current: Cell::default(),
            max_scrollback: 10_000,
        }
    }

    pub fn rows(&self) -> usize {
        self.rows
    }
    pub fn cols(&self) -> usize {
        self.cols
    }
    pub fn cursor(&self) -> (usize, usize) {
        (self.cursor_row, self.cursor_col)
    }
    pub fn scrollback_len(&self) -> usize {
        self.scrollback.len()
    }

    pub fn cell(&self, row: usize, col: usize) -> Option<&Cell> {
        (row < self.rows && col < self.cols).then(|| &self.cells[self.idx(row, col)])
    }

    pub fn render_region(&self, start_row: usize, row_count: usize) -> Vec<Vec<RenderedCell>> {
        let end = min(self.rows, start_row.saturating_add(row_count));
        (start_row..end)
            .map(|row| {
                (0..self.cols)
                    .map(|col| RenderedCell {
                        row,
                        col,
                        cell: self.cells[self.idx(row, col)].clone(),
                    })
                    .collect()
            })
            .collect()
    }

    pub fn resize(&mut self, rows: usize, cols: usize) {
        let rows = max(rows, 1);
        let cols = max(cols, 1);
        let mut resized = vec![Cell::default(); rows * cols];
        for row in 0..min(self.rows, rows) {
            for col in 0..min(self.cols, cols) {
                resized[row * cols + col] = self.cells[self.idx(row, col)].clone();
            }
        }
        self.rows = rows;
        self.cols = cols;
        self.cells = resized;
        self.cursor_row = min(self.cursor_row, rows - 1);
        self.cursor_col = min(self.cursor_col, cols - 1);
    }

    pub fn print_char(&mut self, ch: char) {
        if ch == '\n' {
            self.linefeed();
            return;
        }
        if ch == '\r' {
            self.carriage_return();
            return;
        }
        if self.cursor_col >= self.cols {
            self.carriage_return();
            self.linefeed();
        }
        let idx = self.idx(self.cursor_row, self.cursor_col);
        self.cells[idx] = Cell {
            ch,
            ..self.current.clone()
        };
        self.cursor_col += 1;
        if self.cursor_col >= self.cols {
            self.cursor_col = self.cols;
        }
    }

    pub fn carriage_return(&mut self) {
        self.cursor_col = 0;
    }
    pub fn linefeed(&mut self) {
        if self.cursor_row + 1 >= self.rows {
            self.scroll_up(1);
        } else {
            self.cursor_row += 1;
        }
    }

    pub fn scroll_up(&mut self, count: usize) {
        for _ in 0..max(count, 1) {
            let first = self.cells.drain(0..self.cols).collect::<Vec<_>>();
            self.scrollback.push(first);
            if self.scrollback.len() > self.max_scrollback {
                self.scrollback.remove(0);
            }
            self.cells.extend(vec![Cell::default(); self.cols]);
        }
    }

    pub fn scroll_down(&mut self, count: usize) {
        for _ in 0..max(count, 1) {
            self.cells.splice(0..0, vec![Cell::default(); self.cols]);
            self.cells.truncate(self.rows * self.cols);
        }
    }

    pub fn erase_display(&mut self, mode: u16) {
        match mode {
            0 => {
                for idx in self.idx(self.cursor_row, self.cursor_col)..self.cells.len() {
                    self.cells[idx] = Cell::default();
                }
            }
            1 => {
                for idx in 0..=self.idx(self.cursor_row, self.cursor_col) {
                    self.cells[idx] = Cell::default();
                }
            }
            2 | 3 => self.cells.fill(Cell::default()),
            _ => {}
        }
    }

    pub fn erase_line(&mut self, mode: u16) {
        let start = self.cursor_row * self.cols;
        match mode {
            0 => {
                for idx in self.idx(self.cursor_row, self.cursor_col)..start + self.cols {
                    self.cells[idx] = Cell::default();
                }
            }
            1 => {
                for idx in start..=self.idx(self.cursor_row, self.cursor_col) {
                    self.cells[idx] = Cell::default();
                }
            }
            2 => {
                for idx in start..start + self.cols {
                    self.cells[idx] = Cell::default();
                }
            }
            _ => {}
        }
    }

    fn idx(&self, row: usize, col: usize) -> usize {
        row * self.cols + col
    }
    fn move_cursor(&mut self, row: usize, col: usize) {
        self.cursor_row = min(row, self.rows - 1);
        self.cursor_col = min(col, self.cols - 1);
    }
    fn param(params: &[u16], idx: usize, default: u16) -> u16 {
        params
            .get(idx)
            .copied()
            .filter(|v| *v != 0)
            .unwrap_or(default)
    }
    fn flatten(params: &Params) -> Vec<u16> {
        params.iter().flat_map(|p| p.iter().copied()).collect()
    }

    fn sgr(&mut self, params: &[u16]) {
        let params = if params.is_empty() {
            vec![0]
        } else {
            params.to_vec()
        };
        let mut i = 0;
        while i < params.len() {
            match params[i] {
                0 => self.current = Cell::default(),
                1 => self.current.bold = true,
                3 => self.current.italic = true,
                4 => self.current.underline = true,
                7 => self.current.inverse = true,
                22 => self.current.bold = false,
                23 => self.current.italic = false,
                24 => self.current.underline = false,
                27 => self.current.inverse = false,
                30..=37 => self.current.fg = CellColor::Indexed((params[i] - 30) as u8),
                40..=47 => self.current.bg = CellColor::Indexed((params[i] - 40) as u8),
                90..=97 => self.current.fg = CellColor::Indexed((params[i] - 90 + 8) as u8),
                100..=107 => self.current.bg = CellColor::Indexed((params[i] - 100 + 8) as u8),
                39 => self.current.fg = CellColor::Default,
                49 => self.current.bg = CellColor::Default,
                38 | 48 => {
                    let is_fg = params[i] == 38;
                    if params.get(i + 1) == Some(&5) && i + 2 < params.len() {
                        let color = CellColor::Indexed(params[i + 2] as u8);
                        if is_fg {
                            self.current.fg = color;
                        } else {
                            self.current.bg = color;
                        }
                        i += 2;
                    } else if params.get(i + 1) == Some(&2) && i + 4 < params.len() {
                        let color = CellColor::Rgb(
                            params[i + 2] as u8,
                            params[i + 3] as u8,
                            params[i + 4] as u8,
                        );
                        if is_fg {
                            self.current.fg = color;
                        } else {
                            self.current.bg = color;
                        }
                        i += 4;
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }
}

impl Perform for TerminalGrid {
    fn print(&mut self, c: char) {
        self.print_char(c);
    }
    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' => self.linefeed(),
            b'\r' => self.carriage_return(),
            0x08 => self.cursor_col = self.cursor_col.saturating_sub(1),
            b'\t' => self.cursor_col = min(((self.cursor_col / 8) + 1) * 8, self.cols - 1),
            _ => {}
        }
    }
    fn hook(&mut self, _: &Params, _: &[u8], _: bool, _: char) {}
    fn put(&mut self, _: u8) {}
    fn unhook(&mut self) {}
    fn osc_dispatch(&mut self, _: &[&[u8]], _: bool) {}
    fn esc_dispatch(&mut self, _: &[u8], _: bool, byte: u8) {
        if byte == b'D' {
            self.linefeed();
        }
    }
    fn csi_dispatch(&mut self, params: &Params, _: &[u8], _: bool, action: char) {
        let p = Self::flatten(params);
        match action {
            'm' => self.sgr(&p),
            'H' | 'f' => self.move_cursor(
                Self::param(&p, 0, 1).saturating_sub(1) as usize,
                Self::param(&p, 1, 1).saturating_sub(1) as usize,
            ),
            'A' => {
                self.cursor_row = self
                    .cursor_row
                    .saturating_sub(Self::param(&p, 0, 1) as usize)
            }
            'B' => {
                self.cursor_row = min(
                    self.rows - 1,
                    self.cursor_row + Self::param(&p, 0, 1) as usize,
                )
            }
            'C' => {
                self.cursor_col = min(
                    self.cols - 1,
                    self.cursor_col + Self::param(&p, 0, 1) as usize,
                )
            }
            'D' => {
                self.cursor_col = self
                    .cursor_col
                    .saturating_sub(Self::param(&p, 0, 1) as usize)
            }
            'J' => self.erase_display(p.first().copied().unwrap_or(0)),
            'K' => self.erase_line(p.first().copied().unwrap_or(0)),
            'S' => self.scroll_up(Self::param(&p, 0, 1) as usize),
            'T' => self.scroll_down(Self::param(&p, 0, 1) as usize),
            _ => {}
        }
    }
}
