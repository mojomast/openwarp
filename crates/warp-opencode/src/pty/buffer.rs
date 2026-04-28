use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct TerminalBuffer {
    max_bytes: usize,
    bytes: VecDeque<u8>,
}

impl TerminalBuffer {
    pub fn new(max_bytes: usize) -> Self {
        Self {
            max_bytes,
            bytes: VecDeque::new(),
        }
    }

    pub fn push_str(&mut self, text: &str) {
        self.bytes.extend(text.as_bytes());
        while self.bytes.len() > self.max_bytes {
            self.bytes.pop_front();
        }
    }

    pub fn as_lossy_string(&self) -> String {
        String::from_utf8_lossy(&self.bytes.iter().copied().collect::<Vec<_>>()).into_owned()
    }
}
