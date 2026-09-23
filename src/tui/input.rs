use crossterm::event::{ KeyCode, KeyEvent, KeyModifiers };

#[derive(Debug, Clone, Default)]
pub struct TextInput {
    value: String,
    cursor: usize,
}

impl TextInput {
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        let cursor = value.chars().count();
        Self { value, cursor }
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor = 0;
    }

    pub fn len(&self) -> usize {
        self.value.chars().count()
    }

    pub fn visible(&self, focused: bool) -> String {
        if !focused {
            return super::ascii::convert(&self.value).into_owned();
        }

        let at = self.byte_index(self.cursor);
        let (before, after) = self.value.split_at(at);
        if super::ascii::enabled() {
            format!("{}|{}", super::ascii::convert(before), super::ascii::convert(after))
        } else {
            format!("{before}▏{after}")
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        let control = key.modifiers.contains(KeyModifiers::CONTROL);

        match key.code {
            KeyCode::Char(c) if !control => {
                self.insert_char(c);
                true
            }
            KeyCode::Char('u') if control => {
                self.clear();
                true
            }
            KeyCode::Backspace => {
                self.backspace();
                true
            }
            KeyCode::Delete => {
                self.delete();
                true
            }
            KeyCode::Left => {
                self.cursor = self.cursor.saturating_sub(1);
                true
            }
            KeyCode::Right => {
                self.cursor = (self.cursor + 1).min(self.len());
                true
            }
            KeyCode::Home => {
                self.cursor = 0;
                true
            }
            KeyCode::End => {
                self.cursor = self.len();
                true
            }
            _ => false,
        }
    }

    fn insert_char(&mut self, c: char) {
        let at = self.byte_index(self.cursor);
        self.value.insert(at, c);
        self.cursor += 1;
    }

    pub fn insert_str(&mut self, text: &str) {
        for c in text.chars() {
            if c != '\r' && c != '\n' {
                self.insert_char(c);
            }
        }
    }

    fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }

        let at = self.byte_index(self.cursor - 1);
        self.value.remove(at);
        self.cursor -= 1;
    }

    fn delete(&mut self) {
        if self.cursor >= self.len() {
            return;
        }

        let at = self.byte_index(self.cursor);
        self.value.remove(at);
    }

    fn byte_index(&self, position: usize) -> usize {
        self.value
            .char_indices()
            .nth(position)
            .map(|(index, _)| index)
            .unwrap_or(self.value.len())
    }
}
