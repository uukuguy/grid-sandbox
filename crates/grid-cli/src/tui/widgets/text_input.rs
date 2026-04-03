//! Reusable single-line text input widget for TUI screens.

use crossterm::event::KeyCode;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Paragraph};

use crate::tui::theme::TuiTheme;

/// A reusable single-line text input widget with cursor navigation.
pub struct TextInput {
    input: String,
    cursor: usize,
    placeholder: String,
    active: bool,
}

impl TextInput {
    /// Create a new text input with the given placeholder text.
    pub fn new(placeholder: impl Into<String>) -> Self {
        Self {
            input: String::new(),
            cursor: 0,
            placeholder: placeholder.into(),
            active: false,
        }
    }

    /// Return the current input value.
    pub fn value(&self) -> &str {
        &self.input
    }

    /// Whether the input is empty.
    pub fn is_empty(&self) -> bool {
        self.input.is_empty()
    }

    /// Whether the input is currently active (focused).
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Return the current cursor position (char index).
    pub fn cursor_position(&self) -> usize {
        self.cursor
    }

    /// Activate (focus) the input.
    pub fn activate(&mut self) {
        self.active = true;
    }

    /// Deactivate (unfocus) the input.
    pub fn deactivate(&mut self) {
        self.active = false;
    }

    /// Clear the input text and deactivate.
    pub fn clear(&mut self) {
        self.input.clear();
        self.cursor = 0;
        self.active = false;
    }

    /// Set the input value and move cursor to end.
    pub fn set_value(&mut self, value: impl Into<String>) {
        self.input = value.into();
        self.cursor = self.input.chars().count();
    }

    /// Handle a key event. Returns `true` if the key was consumed.
    ///
    /// When inactive, all keys are ignored (returns `false`).
    pub fn handle_key(&mut self, key: KeyCode) -> bool {
        if !self.active {
            return false;
        }

        match key {
            KeyCode::Char(c) => {
                let byte_idx = self.char_to_byte_index(self.cursor);
                self.input.insert(byte_idx, c);
                self.cursor += 1;
                true
            }
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    let byte_idx = self.char_to_byte_index(self.cursor);
                    let next_byte_idx = self.char_to_byte_index(self.cursor + 1);
                    self.input.drain(byte_idx..next_byte_idx);
                }
                true
            }
            KeyCode::Left => {
                self.cursor = self.cursor.saturating_sub(1);
                true
            }
            KeyCode::Right => {
                let char_count = self.input.chars().count();
                if self.cursor < char_count {
                    self.cursor += 1;
                }
                true
            }
            KeyCode::Home => {
                self.cursor = 0;
                true
            }
            KeyCode::End => {
                self.cursor = self.input.chars().count();
                true
            }
            KeyCode::Esc => {
                self.input.clear();
                self.cursor = 0;
                self.active = false;
                true
            }
            _ => false,
        }
    }

    /// Render the text input widget into the given area.
    ///
    /// If a `block` is provided it wraps the input (border area is accounted
    /// for when positioning the cursor).
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &TuiTheme, block: Option<Block>) {
        let inner = if let Some(ref b) = block {
            b.inner(area)
        } else {
            area
        };

        let (text, style) = if self.input.is_empty() && !self.active {
            (self.placeholder.as_str(), theme.text_dim())
        } else {
            (self.input.as_str(), theme.text_normal())
        };

        let paragraph = if let Some(b) = block {
            Paragraph::new(text).block(b).style(style)
        } else {
            Paragraph::new(text).style(style)
        };

        frame.render_widget(paragraph, area);

        // Place the cursor when active
        if self.active {
            // Compute display width up to cursor (byte offset within the inner area)
            let display_offset: u16 = self
                .input
                .chars()
                .take(self.cursor)
                .map(|c| c.len_utf8() as u16)
                .sum::<u16>()
                .min(inner.width.saturating_sub(1));
            let cursor_x = inner.x + display_offset;
            let cursor_y = inner.y;
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }

    // -- private helpers --

    /// Convert a char index to the corresponding byte index in `self.input`.
    fn char_to_byte_index(&self, char_idx: usize) -> usize {
        self.input
            .char_indices()
            .nth(char_idx)
            .map(|(i, _)| i)
            .unwrap_or(self.input.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_defaults() {
        let ti = TextInput::new("Enter text...");
        assert!(ti.is_empty());
        assert!(!ti.is_active());
        assert_eq!(ti.value(), "");
        assert_eq!(ti.cursor_position(), 0);
    }

    #[test]
    fn test_char_insert() {
        let mut ti = TextInput::new("");
        ti.activate();
        ti.handle_key(KeyCode::Char('a'));
        ti.handle_key(KeyCode::Char('b'));
        ti.handle_key(KeyCode::Char('c'));
        assert_eq!(ti.value(), "abc");
        assert_eq!(ti.cursor_position(), 3);
    }

    #[test]
    fn test_backspace() {
        let mut ti = TextInput::new("");
        ti.activate();
        ti.handle_key(KeyCode::Char('x'));
        ti.handle_key(KeyCode::Char('y'));
        ti.handle_key(KeyCode::Backspace);
        assert_eq!(ti.value(), "x");
        assert_eq!(ti.cursor_position(), 1);
    }

    #[test]
    fn test_backspace_at_start_noop() {
        let mut ti = TextInput::new("");
        ti.activate();
        assert!(ti.handle_key(KeyCode::Backspace));
        assert!(ti.is_empty());
        assert_eq!(ti.cursor_position(), 0);
    }

    #[test]
    fn test_cursor_left_right() {
        let mut ti = TextInput::new("");
        ti.activate();
        ti.set_value("abc");
        // cursor is at end (3)
        assert_eq!(ti.cursor_position(), 3);

        ti.handle_key(KeyCode::Left);
        assert_eq!(ti.cursor_position(), 2);

        ti.handle_key(KeyCode::Left);
        ti.handle_key(KeyCode::Left);
        assert_eq!(ti.cursor_position(), 0);

        // Left at 0 stays at 0
        ti.handle_key(KeyCode::Left);
        assert_eq!(ti.cursor_position(), 0);

        ti.handle_key(KeyCode::Right);
        assert_eq!(ti.cursor_position(), 1);

        // Right past end stays at end
        ti.handle_key(KeyCode::Right);
        ti.handle_key(KeyCode::Right);
        ti.handle_key(KeyCode::Right); // at 3, should stay
        assert_eq!(ti.cursor_position(), 3);
    }

    #[test]
    fn test_home_end() {
        let mut ti = TextInput::new("");
        ti.activate();
        ti.set_value("hello");
        assert_eq!(ti.cursor_position(), 5);

        ti.handle_key(KeyCode::Home);
        assert_eq!(ti.cursor_position(), 0);

        ti.handle_key(KeyCode::End);
        assert_eq!(ti.cursor_position(), 5);
    }

    #[test]
    fn test_esc_clears_and_deactivates() {
        let mut ti = TextInput::new("");
        ti.activate();
        ti.set_value("some text");
        assert!(ti.is_active());

        ti.handle_key(KeyCode::Esc);
        assert!(ti.is_empty());
        assert!(!ti.is_active());
        assert_eq!(ti.cursor_position(), 0);
    }

    #[test]
    fn test_inactive_ignores_keys() {
        let mut ti = TextInput::new("");
        // Not activated
        assert!(!ti.handle_key(KeyCode::Char('a')));
        assert!(!ti.handle_key(KeyCode::Backspace));
        assert!(!ti.handle_key(KeyCode::Left));
        assert!(ti.is_empty());
    }

    #[test]
    fn test_unicode_handling() {
        let mut ti = TextInput::new("");
        ti.activate();
        // Insert a multi-byte character
        ti.handle_key(KeyCode::Char('\u{1F600}')); // grinning face emoji (4 bytes)
        assert_eq!(ti.value(), "\u{1F600}");
        assert_eq!(ti.cursor_position(), 1);

        // Insert ASCII after emoji
        ti.handle_key(KeyCode::Char('a'));
        assert_eq!(ti.value(), "\u{1F600}a");
        assert_eq!(ti.cursor_position(), 2);

        // Backspace removes 'a'
        ti.handle_key(KeyCode::Backspace);
        assert_eq!(ti.value(), "\u{1F600}");
        assert_eq!(ti.cursor_position(), 1);

        // Backspace removes the emoji
        ti.handle_key(KeyCode::Backspace);
        assert!(ti.is_empty());
        assert_eq!(ti.cursor_position(), 0);
    }

    #[test]
    fn test_set_value_moves_cursor_to_end() {
        let mut ti = TextInput::new("placeholder");
        ti.set_value("hello world");
        assert_eq!(ti.value(), "hello world");
        assert_eq!(ti.cursor_position(), 11);
    }
}
