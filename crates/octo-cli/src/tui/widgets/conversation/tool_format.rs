//! Tool call and result formatting for conversation display.
//!
//! Adapted from opendev-tui. Uses serde_json::Value for tool inputs
//! and String for tool results (matching octo-types ContentBlock).

use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use std::collections::HashMap;

use crate::tui::formatters::style_tokens;
use crate::tui::formatters::tool_registry::format_tool_call_parts;
use crate::tui::widgets::spinner::COMPLETED_CHAR;

/// Convert a serde_json::Value to HashMap for tool_registry API.
fn value_to_hashmap(input: &serde_json::Value) -> HashMap<String, serde_json::Value> {
    if let Some(obj) = input.as_object() {
        obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    } else {
        HashMap::new()
    }
}

/// Format a ToolUse content block as a styled line with verb(arg) pattern.
pub(super) fn format_tool_use(name: &str, input: &serde_json::Value) -> Line<'static> {
    let args = value_to_hashmap(input);
    let (verb, arg) = format_tool_call_parts(name, &args);

    Line::from(vec![
        Span::styled(
            format!("{COMPLETED_CHAR} "),
            Style::default().fg(style_tokens::GREEN_BRIGHT),
        ),
        Span::styled(
            verb,
            Style::default()
                .fg(style_tokens::PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("({arg})"),
            Style::default().fg(style_tokens::SUBTLE),
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_tool_use_bash() {
        let input = serde_json::json!({"command": "ls -la"});
        let line = format_tool_use("bash", &input);
        assert!(!line.spans.is_empty());
    }

    #[test]
    fn test_format_tool_use_read_file() {
        let input = serde_json::json!({"file_path": "/src/main.rs"});
        let line = format_tool_use("read_file", &input);
        let text: String = line.spans.iter().map(|s| s.content.to_string()).collect();
        assert!(text.contains('\u{23fa}'));
    }
}
