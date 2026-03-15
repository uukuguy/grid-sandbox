//! Chat screen — interactive conversation with an agent

use crossterm::event::KeyCode;
use ratatui::prelude::*;
use ratatui::widgets::{List, ListItem};

use crate::commands::AppState;
use crate::tui::event::AppEvent;
use crate::tui::theme::TuiTheme;
use crate::tui::widgets::TextInput;

use super::Screen;

/// A chat message for display.
#[derive(Debug, Clone)]
struct ChatMessage {
    role: String,
    content: String,
}

/// Interactive chat screen with message display and text input.
pub struct ChatScreen {
    messages: Vec<ChatMessage>,
    pub(crate) text_input: TextInput,
    scroll_offset: usize,
}

impl ChatScreen {
    pub fn new() -> Self {
        let mut text_input = TextInput::new("Type a message...");
        text_input.activate();
        Self {
            messages: vec![ChatMessage {
                role: "system".into(),
                content: "Welcome to Octo Chat. Type a message and press Enter.".into(),
            }],
            text_input,
            scroll_offset: 0,
        }
    }

    fn submit_message(&mut self) {
        let value = self.text_input.value().to_string();
        if value.trim().is_empty() {
            return;
        }
        self.messages.push(ChatMessage {
            role: "user".into(),
            content: value.clone(),
        });
        // Simulated response (actual agent integration deferred)
        self.messages.push(ChatMessage {
            role: "assistant".into(),
            content: format!("Echo: {}", value),
        });
        self.text_input.clear();
        self.text_input.activate();
        self.scroll_offset = self.messages.len().saturating_sub(1);
    }

    fn handle_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Enter => self.submit_message(),
            KeyCode::Up => {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
            }
            KeyCode::Down => {
                if self.scroll_offset < self.messages.len().saturating_sub(1) {
                    self.scroll_offset += 1;
                }
            }
            KeyCode::Esc => {
                // Clear input but keep it active (chat input is always active)
                self.text_input.clear();
                self.text_input.activate();
            }
            other => {
                self.text_input.handle_key(other);
            }
        }
    }
}

impl Screen for ChatScreen {
    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &TuiTheme, _state: &AppState) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(3)])
            .split(area);

        // -- Messages area --
        let msg_block = theme.styled_block(" Chat ");
        let inner = msg_block.inner(chunks[0]);
        frame.render_widget(msg_block, chunks[0]);

        let items: Vec<ListItem> = self
            .messages
            .iter()
            .map(|m| {
                let (prefix, style) = match m.role.as_str() {
                    "user" => ("You: ", Style::default().fg(theme.accent)),
                    "assistant" => ("Octo: ", Style::default().fg(theme.text)),
                    _ => ("", Style::default().fg(theme.muted)),
                };
                ListItem::new(Line::from(vec![
                    Span::styled(prefix, style.add_modifier(Modifier::BOLD)),
                    Span::styled(&m.content, style),
                ]))
            })
            .collect();

        let visible_height = inner.height as usize;
        let visible = if items.len() > visible_height {
            let start = self
                .scroll_offset
                .min(items.len().saturating_sub(visible_height));
            items[start..start + visible_height].to_vec()
        } else {
            items
        };

        let list = List::new(visible);
        frame.render_widget(list, inner);

        // -- Input area --
        let input_block = theme.styled_block_active(" Input ");
        self.text_input
            .render(frame, chunks[1], theme, Some(input_block));
    }

    fn handle_event(&mut self, event: &AppEvent) {
        if let AppEvent::Key(key) = event {
            self.handle_key(key.code);
        }
    }

    fn title(&self) -> &str {
        "Chat"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_has_welcome_message() {
        let screen = ChatScreen::new();
        assert_eq!(screen.messages.len(), 1);
        assert_eq!(screen.messages[0].role, "system");
        assert!(screen.messages[0].content.contains("Welcome"));
        assert!(screen.text_input.is_empty());
        assert_eq!(screen.text_input.cursor_position(), 0);
        assert_eq!(screen.scroll_offset, 0);
    }

    #[test]
    fn submit_message_adds_user_and_echo() {
        let mut screen = ChatScreen::new();
        screen.text_input.set_value("hello");
        screen.submit_message();

        assert_eq!(screen.messages.len(), 3);
        assert_eq!(screen.messages[1].role, "user");
        assert_eq!(screen.messages[1].content, "hello");
        assert_eq!(screen.messages[2].role, "assistant");
        assert_eq!(screen.messages[2].content, "Echo: hello");
        assert!(screen.text_input.is_empty());
        assert_eq!(screen.text_input.cursor_position(), 0);
    }

    #[test]
    fn submit_empty_is_noop() {
        let mut screen = ChatScreen::new();
        screen.text_input.set_value("   ");
        screen.submit_message();
        assert_eq!(screen.messages.len(), 1);
    }

    #[test]
    fn char_input_and_backspace() {
        let mut screen = ChatScreen::new();
        screen.handle_key(KeyCode::Char('a'));
        screen.handle_key(KeyCode::Char('b'));
        assert_eq!(screen.text_input.value(), "ab");
        assert_eq!(screen.text_input.cursor_position(), 2);

        screen.handle_key(KeyCode::Backspace);
        assert_eq!(screen.text_input.value(), "a");
        assert_eq!(screen.text_input.cursor_position(), 1);
    }

    #[test]
    fn backspace_at_start_is_noop() {
        let mut screen = ChatScreen::new();
        screen.handle_key(KeyCode::Backspace);
        assert!(screen.text_input.is_empty());
        assert_eq!(screen.text_input.cursor_position(), 0);
    }

    #[test]
    fn cursor_movement() {
        let mut screen = ChatScreen::new();
        screen.text_input.set_value("abc");

        screen.handle_key(KeyCode::Left);
        assert_eq!(screen.text_input.cursor_position(), 2);

        screen.handle_key(KeyCode::Left);
        screen.handle_key(KeyCode::Left);
        assert_eq!(screen.text_input.cursor_position(), 0);

        // Left at 0 stays at 0
        screen.handle_key(KeyCode::Left);
        assert_eq!(screen.text_input.cursor_position(), 0);

        screen.handle_key(KeyCode::Right);
        assert_eq!(screen.text_input.cursor_position(), 1);

        // Right past end stays at end
        screen.text_input.set_value("abc");
        screen.handle_key(KeyCode::Right);
        assert_eq!(screen.text_input.cursor_position(), 3);
    }

    #[test]
    fn escape_clears_input() {
        let mut screen = ChatScreen::new();
        screen.text_input.set_value("some text");
        screen.handle_key(KeyCode::Esc);
        assert!(screen.text_input.is_empty());
        assert_eq!(screen.text_input.cursor_position(), 0);
    }

    #[test]
    fn scroll_up_down() {
        let mut screen = ChatScreen::new();
        screen.text_input.set_value("msg1");
        screen.submit_message();
        screen.text_input.set_value("msg2");
        screen.submit_message();
        // 5 messages total: 1 system + 2*(user+assistant)
        assert_eq!(screen.messages.len(), 5);

        let max = screen.messages.len() - 1;
        assert_eq!(screen.scroll_offset, max);

        screen.handle_key(KeyCode::Up);
        assert_eq!(screen.scroll_offset, max - 1);

        screen.handle_key(KeyCode::Down);
        assert_eq!(screen.scroll_offset, max);

        // Down at max stays at max
        screen.handle_key(KeyCode::Down);
        assert_eq!(screen.scroll_offset, max);
    }

    #[test]
    fn scroll_up_at_zero_stays() {
        let mut screen = ChatScreen::new();
        screen.scroll_offset = 0;
        screen.handle_key(KeyCode::Up);
        assert_eq!(screen.scroll_offset, 0);
    }

    #[test]
    fn title_is_chat() {
        let screen = ChatScreen::new();
        assert_eq!(screen.title(), "Chat");
    }

    #[test]
    fn enter_submits_from_event() {
        let mut screen = ChatScreen::new();
        screen.text_input.set_value("test");

        let key = crossterm::event::KeyEvent::new(
            KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_event(&AppEvent::Key(key));

        assert_eq!(screen.messages.len(), 3);
        assert!(screen.text_input.is_empty());
    }

    #[test]
    fn non_key_event_is_ignored() {
        let mut screen = ChatScreen::new();
        screen.handle_event(&AppEvent::Tick);
        assert_eq!(screen.messages.len(), 1);
    }
}
