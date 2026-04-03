//! Base formatter traits and types.

use ratatui::text::Line;

/// Result of formatting a tool output for display.
#[derive(Debug, Clone)]
pub struct FormattedOutput<'a> {
    /// Header line (tool name, file path, etc.)
    pub header: Line<'a>,
    /// Body lines (the actual content)
    pub body: Vec<Line<'a>>,
    /// Footer line (summary, stats, etc.)
    pub footer: Option<Line<'a>>,
}

/// Trait for tool-specific output formatters.
pub trait ToolFormatter {
    /// Format a tool result for display.
    fn format<'a>(&self, tool_name: &str, output: &str) -> FormattedOutput<'a>;

    /// Whether this formatter handles the given tool name.
    fn handles(&self, tool_name: &str) -> bool;
}

/// Truncate text to a maximum number of lines, adding a summary.
pub fn truncate_lines(text: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() <= max_lines {
        return text.to_string();
    }

    let head_count = max_lines * 2 / 3;
    let tail_count = max_lines - head_count - 1;
    let omitted = lines.len() - head_count - tail_count;

    let mut result = lines[..head_count].join("\n");
    result.push_str(&format!("\n... ({omitted} lines omitted) ...\n"));
    result.push_str(&lines[lines.len() - tail_count..].join("\n"));
    result
}

/// Indent each line by the given number of spaces.
pub fn indent(text: &str, spaces: usize) -> String {
    let prefix = " ".repeat(spaces);
    text.lines()
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("{prefix}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Strip `<system-reminder>` and `<context>` XML tags and their content from display text.
///
/// System reminders and context blocks are injected into messages for the LLM
/// but should not be shown to the user in the conversation view.
pub fn strip_system_reminders(text: &str) -> String {
    let result = strip_xml_tag(text, "system-reminder");
    let result = strip_xml_tag(&result, "context");

    // Collapse runs of 2+ newlines (left over from removal) into a single newline
    let mut cleaned = result;
    while cleaned.contains("\n\n") {
        cleaned = cleaned.replace("\n\n", "\n");
    }

    cleaned.trim().to_string()
}

/// Strip a specific XML tag and its content from text.
fn strip_xml_tag(text: &str, tag: &str) -> String {
    let open_tag = format!("<{tag}>");
    let close_tag = format!("</{tag}>");
    let mut result = String::with_capacity(text.len());
    let mut remaining = text;

    while !remaining.is_empty() {
        if let Some(start) = remaining.find(&open_tag) {
            result.push_str(&remaining[..start]);
            let after_open = &remaining[start..];
            if let Some(end) = after_open.find(&close_tag) {
                remaining = &after_open[end + close_tag.len()..];
            } else {
                // No closing tag — strip from open tag to end
                break;
            }
        } else {
            result.push_str(remaining);
            break;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_lines_short() {
        let text = "line 1\nline 2\nline 3";
        assert_eq!(truncate_lines(text, 10), text);
    }

    #[test]
    fn test_truncate_lines_long() {
        let lines: Vec<String> = (0..100).map(|i| format!("line {i}")).collect();
        let text = lines.join("\n");
        let result = truncate_lines(&text, 20);
        assert!(result.contains("omitted"));
        assert!(result.lines().count() <= 21);
    }

    #[test]
    fn test_indent() {
        let text = "line 1\nline 2\n\nline 3";
        let result = indent(text, 4);
        assert!(result.starts_with("    line 1"));
        assert!(result.contains("\n    line 2"));
        assert!(result.contains("\n\n"));
    }

    #[test]
    fn test_strip_system_reminders() {
        let input = "Hello<system-reminder>secret</system-reminder> world";
        let result = strip_system_reminders(input);
        assert!(result.contains("Hello"));
        assert!(result.contains("world"));
        assert!(!result.contains("secret"));
    }

    #[test]
    fn test_strip_system_reminders_multiple() {
        let input = "a<system-reminder>1</system-reminder>b<system-reminder>2</system-reminder>c";
        let result = strip_system_reminders(input);
        assert_eq!(result, "abc");
    }

    #[test]
    fn test_strip_system_reminders_none() {
        let input = "no tags here";
        let result = strip_system_reminders(input);
        assert_eq!(result, "no tags here");
    }

    #[test]
    fn test_strip_context_tags() {
        let input = "<context>\n<datetime>2026-03-22 23:37 +08:00</datetime>\n</context>\nHello";
        let result = strip_system_reminders(input);
        assert_eq!(result, "Hello");
    }

    #[test]
    fn test_strip_context_only() {
        let input = "<context>\n<datetime>2026-03-22 23:37 +08:00</datetime>\n<user_profile>test</user_profile>\n</context>";
        let result = strip_system_reminders(input);
        assert_eq!(result, "");
    }
}
