//! Rope-backed text buffer used by the input composer.
//!
//! The composer is still rendered with simple WarpUI text primitives, but all
//! edits go through this UTF-8 safe buffer so cursor movement and deletion work
//! in character offsets rather than byte offsets.

use ropey::Rope;

#[derive(Debug, Clone)]
pub struct DraftBuffer {
    rope: Rope,
    cursor: usize,
}

impl Default for DraftBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl DraftBuffer {
    pub fn new() -> Self {
        Self {
            rope: Rope::new(),
            cursor: 0,
        }
    }

    pub fn from_text(text: impl AsRef<str>) -> Self {
        let rope = Rope::from_str(text.as_ref());
        let cursor = rope.len_chars();
        Self { rope, cursor }
    }

    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    pub fn len_bytes(&self) -> usize {
        self.rope.len_bytes()
    }

    pub fn is_empty(&self) -> bool {
        self.len_chars() == 0
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn cursor_char_idx(&self) -> usize {
        self.cursor
    }

    pub fn set_cursor(&mut self, cursor: usize) {
        self.cursor = cursor.min(self.len_chars());
    }

    pub fn move_to_start(&mut self) {
        self.cursor = 0;
    }

    pub fn move_to_end(&mut self) {
        self.cursor = self.len_chars();
    }

    pub fn move_left(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        self.cursor -= 1;
        true
    }

    pub fn move_right(&mut self) -> bool {
        if self.cursor >= self.len_chars() {
            return false;
        }
        self.cursor += 1;
        true
    }

    pub fn insert(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.rope.insert(self.cursor, text);
        self.cursor += text.chars().count();
    }

    pub fn insert_str(&mut self, text: &str) {
        self.insert(text);
    }

    pub fn paste(&mut self, text: &str) {
        let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
        self.insert_str(&normalized);
    }

    pub fn insert_char(&mut self, ch: char) {
        self.rope.insert_char(self.cursor, ch);
        self.cursor += 1;
    }

    pub fn backspace(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        let start = self.cursor - 1;
        self.rope.remove(start..self.cursor);
        self.cursor = start;
        true
    }

    pub fn delete_backward(&mut self) -> bool {
        self.backspace()
    }

    pub fn delete(&mut self) -> bool {
        if self.cursor >= self.len_chars() {
            return false;
        }
        self.rope.remove(self.cursor..self.cursor + 1);
        true
    }

    pub fn delete_forward(&mut self) -> bool {
        self.delete()
    }

    pub fn delete_word_backward(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        let chars: Vec<char> = self.to_string().chars().collect();
        let mut start = self.cursor;
        while start > 0 && chars[start - 1].is_whitespace() {
            start -= 1;
        }
        while start > 0 && !chars[start - 1].is_whitespace() {
            start -= 1;
        }
        self.rope.remove(start..self.cursor);
        self.cursor = start;
        true
    }

    pub fn delete_word_forward(&mut self) -> bool {
        if self.cursor >= self.len_chars() {
            return false;
        }
        let chars: Vec<char> = self.to_string().chars().collect();
        let mut end = self.cursor;
        while end < chars.len() && chars[end].is_whitespace() {
            end += 1;
        }
        while end < chars.len() && !chars[end].is_whitespace() {
            end += 1;
        }
        self.rope.remove(self.cursor..end);
        true
    }

    pub fn delete_to_start_of_line(&mut self) -> bool {
        let (line, col) = self.cursor_line_and_col();
        if col == 0 {
            return false;
        }
        let start = self.line_start_char_idx(line);
        self.rope.remove(start..self.cursor);
        self.cursor = start;
        true
    }

    pub fn delete_to_end_of_line(&mut self) -> bool {
        let (line, col) = self.cursor_line_and_col();
        let line_len = self.line(line).chars().count();
        if col >= line_len {
            return false;
        }
        let end = self.line_start_char_idx(line) + line_len;
        self.rope.remove(self.cursor..end);
        true
    }

    pub fn move_cursor_left(&mut self) -> bool {
        self.move_left()
    }

    pub fn move_cursor_right(&mut self) -> bool {
        self.move_right()
    }

    pub fn move_cursor_start_of_line(&mut self) {
        let (line, _) = self.cursor_line_and_col();
        self.cursor = self.line_start_char_idx(line);
    }

    pub fn move_cursor_end_of_line(&mut self) {
        let (line, _) = self.cursor_line_and_col();
        self.cursor = self.line_start_char_idx(line) + self.line(line).chars().count();
    }

    pub fn move_cursor_up(&mut self) -> bool {
        let (line, col) = self.cursor_line_and_col();
        if line == 0 {
            return false;
        }
        let target_line = line - 1;
        self.cursor =
            self.line_start_char_idx(target_line) + col.min(self.line(target_line).chars().count());
        true
    }

    pub fn move_cursor_down(&mut self) -> bool {
        let (line, col) = self.cursor_line_and_col();
        if line + 1 >= self.line_count() {
            return false;
        }
        let target_line = line + 1;
        self.cursor =
            self.line_start_char_idx(target_line) + col.min(self.line(target_line).chars().count());
        true
    }

    pub fn insert_newline(&mut self) {
        self.insert_char('\n');
    }

    pub fn as_str(&self) -> String {
        self.to_string()
    }

    pub fn line_count(&self) -> usize {
        self.to_string().split('\n').count().max(1)
    }

    pub fn line(&self, idx: usize) -> String {
        self.to_string()
            .split('\n')
            .nth(idx)
            .unwrap_or_default()
            .to_string()
    }

    pub fn cursor_line_and_col(&self) -> (usize, usize) {
        let before = self.text_before_cursor();
        let line = before.chars().filter(|ch| *ch == '\n').count();
        let col = before.chars().rev().take_while(|ch| *ch != '\n').count();
        (line, col)
    }

    fn line_start_char_idx(&self, target_line: usize) -> usize {
        if target_line == 0 {
            return 0;
        }
        let mut line = 0;
        for (idx, ch) in self.to_string().chars().enumerate() {
            if ch == '\n' {
                line += 1;
                if line == target_line {
                    return idx + 1;
                }
            }
        }
        self.len_chars()
    }

    pub fn clear(&mut self) {
        self.rope = Rope::new();
        self.cursor = 0;
    }

    pub fn trim_text(&self) -> String {
        self.to_string().trim().to_owned()
    }

    pub fn text_before_cursor(&self) -> String {
        self.rope.slice(..self.cursor).to_string()
    }

    pub fn text_after_cursor(&self) -> String {
        self.rope.slice(self.cursor..).to_string()
    }

    /// Returns a single string with a visible caret marker inserted at the
    /// logical cursor. This is a fallback until the composer can render a real
    /// inline caret in WarpUI.
    pub fn display_with_caret(&self) -> String {
        let mut text = self.text_before_cursor();
        text.push('▍');
        text.push_str(&self.text_after_cursor());
        text
    }
}

impl From<&str> for DraftBuffer {
    fn from(value: &str) -> Self {
        Self::from_text(value)
    }
}

impl From<String> for DraftBuffer {
    fn from(value: String) -> Self {
        Self::from_text(value)
    }
}

impl std::fmt::Display for DraftBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.rope.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::DraftBuffer;

    #[test]
    fn inserts_at_cursor_and_preserves_unicode() {
        let mut draft = DraftBuffer::from("hi 🌊");
        draft.move_left();
        draft.insert("warp ");

        assert_eq!(draft.to_string(), "hi warp 🌊");
        assert_eq!(draft.cursor(), "hi warp ".chars().count());
    }

    #[test]
    fn backspace_and_delete_are_character_based() {
        let mut draft = DraftBuffer::from("a🤖b");
        draft.set_cursor(2);

        assert!(draft.backspace());
        assert_eq!(draft.to_string(), "ab");
        assert_eq!(draft.cursor(), 1);
        assert!(draft.delete());
        assert_eq!(draft.to_string(), "a");
    }

    #[test]
    fn cursor_is_clamped_and_can_move_to_edges() {
        let mut draft = DraftBuffer::from("abc");
        draft.set_cursor(99);
        assert_eq!(draft.cursor(), 3);
        assert!(!draft.move_right());
        draft.move_to_start();
        assert_eq!(draft.cursor(), 0);
        assert!(!draft.move_left());
    }

    #[test]
    fn display_with_caret_splits_at_logical_cursor() {
        let mut draft = DraftBuffer::from("ab\ncd");
        draft.set_cursor(3);
        assert_eq!(draft.display_with_caret(), "ab\n▍cd");
    }

    #[test]
    fn multiline_cursor_and_word_ops() {
        let mut draft = DraftBuffer::from("one two\nthree");
        assert_eq!(draft.line_count(), 2);
        draft.set_cursor(7);
        assert_eq!(draft.cursor_line_and_col(), (0, 7));
        assert!(draft.delete_word_backward());
        assert_eq!(draft.to_string(), "one \nthree");
        draft.move_cursor_down();
        assert_eq!(draft.cursor_line_and_col().0, 1);
        draft.move_cursor_start_of_line();
        draft.insert_str("new ");
        assert_eq!(draft.line(1), "new three");
    }

    #[test]
    fn clear_resets_text_and_cursor() {
        let mut draft = DraftBuffer::from("abc");
        draft.clear();
        assert!(draft.is_empty());
        assert_eq!(draft.cursor_char_idx(), 0);
    }

    #[test]
    fn paste_normalizes_windows_and_classic_line_endings() {
        let mut draft = DraftBuffer::new();

        draft.paste("one\r\ntwo\rthree");

        assert_eq!(draft.to_string(), "one\ntwo\nthree");
        assert_eq!(draft.line_count(), 3);
        assert_eq!(draft.cursor(), draft.len_chars());
    }

    #[test]
    fn paste_multiline_inserts_at_cursor() {
        let mut draft = DraftBuffer::from_text("abef");
        draft.set_cursor(2);

        draft.paste("c\nd");

        assert_eq!(draft.to_string(), "abc\ndef");
        assert_eq!(draft.cursor_line_and_col(), (1, 1));
    }
}
