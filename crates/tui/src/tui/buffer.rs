use std::cmp::min;

#[derive(Debug, Clone, Default)]
pub struct TextBuffer {
    text: String,
    cursor: usize,
}

impl TextBuffer {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            cursor: 0,
        }
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }

    pub fn set<T: Into<String>>(&mut self, value: T) {
        self.text = value.into();
        self.cursor = self.text.len();
    }

    pub fn insert_char(&mut self, ch: char) {
        if ch == '\r' {
            return;
        }
        let mut buf = [0u8; 4];
        let encoded = ch.encode_utf8(&mut buf);
        self.text.insert_str(self.cursor, encoded);
        self.cursor += encoded.len();
    }

    pub fn insert_newline(&mut self) {
        self.insert_char('\n');
    }

    pub fn insert_tab(&mut self) {
        self.insert_char('\t');
    }

    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let mut iter = self.text[..self.cursor].char_indices().rev();
        if let Some((idx, _ch)) = iter.next() {
            self.text.drain(idx..self.cursor);
            self.cursor = idx;
        }
    }

    pub fn delete_char(&mut self) {
        if self.cursor >= self.text.len() {
            return;
        }
        let mut iter = self.text[self.cursor..].char_indices();
        if let Some((idx, ch)) = iter.next() {
            let end = self.cursor + idx + ch.len_utf8();
            self.text.drain(self.cursor..end);
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let mut iter = self.text[..self.cursor].char_indices().rev();
        if let Some((idx, _)) = iter.next() {
            self.cursor = idx;
        } else {
            self.cursor = 0;
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor >= self.text.len() {
            return;
        }
        let mut iter = self.text[self.cursor..].char_indices();
        if let Some((idx, ch)) = iter.next() {
            self.cursor += idx + ch.len_utf8();
        } else {
            self.cursor = self.text.len();
        }
    }

    pub fn move_home(&mut self) {
        let offsets = self.line_offsets();
        let (line, _) = self.cursor_line_col();
        if line < offsets.len() {
            self.cursor = offsets[line];
        }
    }

    pub fn move_end(&mut self) {
        let offsets = self.line_offsets();
        let (line, _) = self.cursor_line_col();
        let end = self.line_end_offset(&offsets, line);
        self.cursor = end;
    }

    pub fn move_up(&mut self) {
        let offsets = self.line_offsets();
        let (line, col) = self.cursor_line_col();
        if line == 0 {
            return;
        }
        let target_line = line - 1;
        let start = offsets[target_line];
        let end = self.line_end_offset(&offsets, target_line);
        let slice = &self.text[start..end];
        let new_col = min(col, slice.chars().count());
        self.cursor = self.offset_for_column(start, slice, new_col);
    }

    pub fn move_down(&mut self) {
        let offsets = self.line_offsets();
        let (line, col) = self.cursor_line_col();
        if line + 1 >= self.line_count(&offsets) {
            return;
        }
        let target_line = line + 1;
        let start = offsets[target_line];
        let end = self.line_end_offset(&offsets, target_line);
        let slice = &self.text[start..end];
        let new_col = min(col, slice.chars().count());
        self.cursor = self.offset_for_column(start, slice, new_col);
    }

    pub fn cursor_line_col(&self) -> (usize, usize) {
        let offsets = self.line_offsets();
        for (line, start) in offsets.iter().enumerate() {
            let next_start = if line + 1 < offsets.len() {
                offsets[line + 1]
            } else {
                self.text.len()
            };
            if self.cursor >= *start && self.cursor <= next_start {
                let slice = &self.text[*start..self.cursor];
                return (line, slice.chars().count());
            }
        }
        (0, 0)
    }

    // Internal helpers
    fn line_offsets(&self) -> Vec<usize> {
        let mut offsets = vec![0];
        for (idx, ch) in self.text.char_indices() {
            if ch == '\n' {
                offsets.push(idx + ch.len_utf8());
            }
        }
        if *offsets.last().unwrap() != self.text.len() {
            offsets.push(self.text.len());
        }
        offsets
    }

    fn line_count(&self, offsets: &[usize]) -> usize {
        offsets.len().saturating_sub(1).max(1)
    }

    fn line_end_offset(&self, offsets: &[usize], line: usize) -> usize {
        if line + 1 < offsets.len() {
            let mut end = offsets[line + 1];
            if end > 0 && self.text.as_bytes()[end - 1] == b'\n' {
                end -= 1;
            }
            end
        } else {
            self.text.len()
        }
    }

    fn offset_for_column(&self, base: usize, slice: &str, column: usize) -> usize {
        let mut col = 0;
        for (idx, ch) in slice.char_indices() {
            if col == column {
                return base + idx;
            }
            col += 1;
            if col == column {
                return base + idx + ch.len_utf8();
            }
        }
        base + slice.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_places_cursor_at_end() {
        let mut buffer = TextBuffer::new();
        buffer.set("hello");

        assert_eq!(buffer.as_str(), "hello");
        assert_eq!(buffer.cursor_line_col(), (0, 5));
    }

    #[test]
    fn cursor_line_col_tracks_navigation() {
        let mut buffer = TextBuffer::new();
        buffer.set("alpha\nbeta");

        buffer.move_home();
        buffer.move_right();

        assert_eq!(buffer.cursor_line_col(), (1, 1));
    }
}
