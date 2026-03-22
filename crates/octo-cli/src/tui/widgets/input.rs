//! User input/prompt widget with multiline editing and cursor rendering.
//!
//! Ported from opendev-tui. Displays a separator line with mode indicator,
//! queue count, and multiline input area with visible cursor.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::tui::formatters::style_tokens;

/// Widget for the user input area.
pub struct InputWidget<'a> {
    buffer: &'a str,
    cursor: usize,
    mode: &'a str,
    pending_count: usize,
}

/// Result of rendering the input widget, including cursor position for IME.
pub struct InputRenderResult {
    /// Absolute cursor position (x, y) for terminal IME placement.
    pub cursor_position: Option<(u16, u16)>,
}

impl<'a> InputWidget<'a> {
    pub fn new(buffer: &'a str, cursor: usize, mode: &'a str, pending_count: usize) -> Self {
        Self {
            buffer,
            cursor,
            mode,
            pending_count,
        }
    }

    /// Render the widget and return cursor position for IME.
    pub fn render_with_cursor(self, area: Rect, buf: &mut Buffer) -> InputRenderResult {
        let cursor_pos = self.compute_cursor_position(area);
        Widget::render(self, area, buf);
        InputRenderResult {
            cursor_position: cursor_pos,
        }
    }

    /// Compute absolute cursor position for IME placement.
    fn compute_cursor_position(&self, area: Rect) -> Option<(u16, u16)> {
        if area.height < 2 {
            return None;
        }
        let text_y = area.y + 1; // below separator
        let prefix_width = 2u16; // "❯ " or "  "

        if self.buffer.is_empty() {
            // Cursor at start of placeholder
            return Some((area.x + prefix_width, text_y));
        }

        let input_lines: Vec<&str> = self.buffer.split('\n').collect();
        let mut cursor_line = 0usize;
        let mut cursor_col = 0usize;
        let mut pos = 0usize;
        for (i, line) in input_lines.iter().enumerate() {
            if self.cursor <= pos + line.len() {
                cursor_line = i;
                cursor_col = self.cursor - pos;
                break;
            }
            pos += line.len() + 1;
            if i == input_lines.len() - 1 {
                cursor_line = i;
                cursor_col = line.len();
            }
        }

        let x = area.x + prefix_width + cursor_col as u16;
        let y = text_y + cursor_line as u16;
        Some((x, y))
    }
}

impl Widget for InputWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 2 {
            return;
        }

        let accent = if self.mode == "PLAN" {
            style_tokens::GREEN_LIGHT
        } else {
            style_tokens::ACCENT
        };

        let placeholder = "Type a message...";

        // Row 0: separator line with embedded mode indicator
        let mode_label = match self.mode {
            "NORMAL" => "Normal",
            "PLAN" => "Plan",
            other => other,
        };
        let mode_text = format!(" {mode_label} ");
        let hint_text = "(Alt+Enter: newline) ";
        let prefix_dashes = 2;

        let queue_text = if self.pending_count > 0 {
            format!(
                "\u{2500}\u{2500} {} message{} queued (ESC) ",
                self.pending_count,
                if self.pending_count == 1 { "" } else { "s" }
            )
        } else {
            String::new()
        };

        let used = prefix_dashes + mode_text.len() + hint_text.len() + queue_text.len();
        let remaining_dashes = (area.width as usize).saturating_sub(used);

        let sep_style = Style::default().fg(accent);
        let mut spans = vec![
            Span::styled("\u{2500}\u{2500} ", sep_style),
            Span::styled(
                mode_text,
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled(hint_text, Style::default().fg(style_tokens::GREY)),
        ];
        if !queue_text.is_empty() {
            spans.push(Span::styled(
                queue_text,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ));
        }
        spans.push(Span::styled("\u{2500}".repeat(remaining_dashes), sep_style));
        let sep_line = Line::from(spans);
        buf.set_line(area.left(), area.top(), &sep_line, area.width);

        // Bottom border line
        let bottom_row = area.y + area.height.saturating_sub(1);
        if bottom_row > area.top() {
            let bottom_line = Line::from(Span::styled(
                "\u{2500}".repeat(area.width as usize),
                sep_style,
            ));
            buf.set_line(area.left(), bottom_row, &bottom_line, area.width);
        }

        // Rows between separator and bottom border: multiline input
        let text_height = area.height.saturating_sub(2); // -1 top sep, -1 bottom border
        if text_height == 0 {
            return;
        }
        let text_area = Rect {
            x: area.x,
            y: area.y + 1,
            width: area.width,
            height: text_height,
        };

        if self.buffer.is_empty() {
            let prefix = Span::styled(
                "\u{276f} ".to_string(),
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            );
            let cursor_style = Style::default().fg(Color::Black).bg(Color::White);
            // Show block cursor on first char of placeholder
            let first_char = &placeholder[..1];
            let rest_placeholder = &placeholder[1..];
            let content = vec![
                prefix,
                Span::styled(first_char.to_string(), cursor_style),
                Span::styled(rest_placeholder, Style::default().fg(style_tokens::SUBTLE)),
            ];
            Paragraph::new(Line::from(content)).render(text_area, buf);
        } else {
            let input_lines: Vec<&str> = self.buffer.split('\n').collect();

            // Compute cursor line and column
            let mut cursor_line = 0;
            let mut cursor_col = 0;
            let mut pos = 0;
            for (i, line) in input_lines.iter().enumerate() {
                if self.cursor <= pos + line.len() {
                    cursor_line = i;
                    cursor_col = self.cursor - pos;
                    break;
                }
                pos += line.len() + 1;
                if i == input_lines.len() - 1 {
                    cursor_line = i;
                    cursor_col = line.len();
                }
            }

            let prefix_style = Style::default().fg(accent).add_modifier(Modifier::BOLD);
            let cursor_style = Style::default().fg(Color::Black).bg(Color::White);

            for (i, line_text) in input_lines.iter().enumerate() {
                if i as u16 >= text_height {
                    break;
                }
                let row = text_area.y + i as u16;
                let pfx = if i == 0 { "\u{276f} " } else { "  " };

                if i == cursor_line {
                    let before = &line_text[..cursor_col];
                    let (cursor_char, after) = if cursor_col < line_text.len() {
                        let ch = line_text[cursor_col..].chars().next().unwrap();
                        let end = cursor_col + ch.len_utf8();
                        (&line_text[cursor_col..end], &line_text[end..])
                    } else {
                        (" ", "")
                    };
                    let spans = Line::from(vec![
                        Span::styled(pfx, prefix_style),
                        Span::raw(before.to_string()),
                        Span::styled(cursor_char.to_string(), cursor_style),
                        Span::raw(after.to_string()),
                    ]);
                    buf.set_line(text_area.x, row, &spans, text_area.width);
                } else {
                    let spans = Line::from(vec![
                        Span::styled(pfx, prefix_style),
                        Span::raw(line_text.to_string()),
                    ]);
                    buf.set_line(text_area.x, row, &spans, text_area.width);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_widget_creation() {
        let _widget = InputWidget::new("hello", 3, "NORMAL", 0);
    }

    #[test]
    fn test_input_widget_empty() {
        let _widget = InputWidget::new("", 0, "NORMAL", 0);
    }

    #[test]
    fn test_queue_indicator_in_separator() {
        let area = Rect::new(0, 0, 60, 3);
        let mut buf = Buffer::empty(area);

        let widget = InputWidget::new("", 0, "NORMAL", 2);
        widget.render(area, &mut buf);

        let rendered: String = (0..area.width)
            .map(|x| {
                buf.cell((x, 0))
                    .map_or(' ', |c| c.symbol().chars().next().unwrap_or(' '))
            })
            .collect();
        assert!(
            rendered.contains("2 messages queued"),
            "Expected '2 messages queued' in separator line, got: {rendered:?}"
        );
    }

    #[test]
    fn test_queue_indicator_single_message() {
        let area = Rect::new(0, 0, 60, 3);
        let mut buf = Buffer::empty(area);

        let widget = InputWidget::new("", 0, "NORMAL", 1);
        widget.render(area, &mut buf);

        let rendered: String = (0..area.width)
            .map(|x| {
                buf.cell((x, 0))
                    .map_or(' ', |c| c.symbol().chars().next().unwrap_or(' '))
            })
            .collect();
        assert!(
            rendered.contains("1 message queued"),
            "Expected '1 message queued' in separator line, got: {rendered:?}"
        );
        assert!(
            !rendered.contains("1 messages"),
            "Should use singular 'message' for count=1"
        );
    }
}
