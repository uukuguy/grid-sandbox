//! Dynamic tool output formatter registry.
//!
//! Maps tool names to `ToolFormatter` implementations for rich,
//! tool-specific rendering of tool results in the conversation view.

use ratatui::{
    style::Style,
    text::{Line, Span},
};

use super::base::{FormattedOutput, ToolFormatter};
use super::style_tokens;

/// Default formatter for tools without a specialized formatter.
pub struct GenericFormatter;

impl ToolFormatter for GenericFormatter {
    fn format<'a>(&self, _tool_name: &str, output: &str) -> FormattedOutput<'a> {
        let total = output.lines().count();
        let max_lines = 20;

        let header = Line::from(vec![Span::styled(
            format!("  Tool output ({total} lines)"),
            Style::default().fg(style_tokens::SUBTLE),
        )]);

        let lines: Vec<&str> = output.lines().collect();
        let truncated = lines.len() > max_lines;
        let display = if truncated {
            &lines[..max_lines]
        } else {
            &lines[..]
        };

        let body: Vec<Line<'a>> = display
            .iter()
            .map(|l| Line::from(Span::raw(format!("    {l}"))))
            .collect();

        let footer = if truncated {
            let remaining = lines.len() - max_lines;
            Some(Line::from(Span::styled(
                format!("    ... ({remaining} more lines)"),
                Style::default().fg(style_tokens::GREY),
            )))
        } else {
            None
        };

        FormattedOutput {
            header,
            body,
            footer,
        }
    }

    fn handles(&self, _tool_name: &str) -> bool {
        true // fallback handles everything
    }
}

/// Registry that maps tool names to formatter implementations.
pub struct ToolFormatterRegistry {
    formatters: Vec<Box<dyn ToolFormatter>>,
    default: GenericFormatter,
}

impl ToolFormatterRegistry {
    /// Create a new registry with built-in formatters.
    pub fn new() -> Self {
        let formatters: Vec<Box<dyn ToolFormatter>> = vec![
            Box::new(super::bash_formatter::BashFormatter),
            Box::new(super::file_formatter::FileFormatter),
        ];
        Self {
            formatters,
            default: GenericFormatter,
        }
    }

    /// Look up and format tool output using the appropriate formatter.
    pub fn format<'a>(&self, tool_name: &str, output: &str) -> FormattedOutput<'a> {
        for formatter in &self.formatters {
            if formatter.handles(tool_name) {
                return formatter.format(tool_name, output);
            }
        }
        self.default.format(tool_name, output)
    }

    /// Format a collapsed one-line summary for a tool result.
    pub fn format_collapsed(&self, tool_name: &str, output: &str) -> Line<'static> {
        let line_count = output.lines().count();
        Line::from(vec![
            Span::styled(
                "  \u{25B6} ",
                Style::default().fg(style_tokens::ACCENT),
            ),
            Span::styled(
                format!("\u{2699} {tool_name} "),
                Style::default().fg(style_tokens::GREY),
            ),
            Span::styled(
                format!("\u{2713} \u{2014} {line_count} lines "),
                Style::default().fg(style_tokens::SUBTLE),
            ),
            Span::styled(
                "(Ctrl+O cycle | Ctrl+Shift+O all)",
                Style::default()
                    .fg(style_tokens::SUBTLE)
                    .add_modifier(ratatui::style::Modifier::DIM),
            ),
        ])
    }
}

impl Default for ToolFormatterRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_returns_bash_formatter() {
        let registry = ToolFormatterRegistry::new();
        let result = registry.format("bash", "$ ls\nfile1\nfile2\nExit code: 0");
        let header_text: String = result
            .header
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        assert!(header_text.contains("$"), "Bash formatter should show $ prefix");
    }

    #[test]
    fn registry_returns_file_formatter_for_read() {
        let registry = ToolFormatterRegistry::new();
        let result = registry.format("Read", "line1\nline2\nline3");
        let header_text: String = result
            .header
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        assert!(
            header_text.contains("3 lines"),
            "File formatter should show line count"
        );
    }

    #[test]
    fn registry_unknown_tool_uses_default() {
        let registry = ToolFormatterRegistry::new();
        let result = registry.format("unknown_mcp_tool", "some output\nmore output");
        let header_text: String = result
            .header
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        assert!(
            header_text.contains("2 lines"),
            "Generic formatter should show line count"
        );
    }

    #[test]
    fn registry_collapsed_format() {
        let registry = ToolFormatterRegistry::new();
        let line = registry.format_collapsed("bash", "line1\nline2\nline3");
        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text.contains("3 lines"));
        assert!(text.contains("bash"));
    }

    #[test]
    fn generic_formatter_truncates() {
        let long_output: String = (0..30)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let formatter = GenericFormatter;
        let result = formatter.format("test", &long_output);
        assert_eq!(result.body.len(), 20);
        assert!(result.footer.is_some());
    }
}
