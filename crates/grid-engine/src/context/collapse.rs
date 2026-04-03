//! Context Collapse — granularity-level message folding (AP-T9).
//!
//! Before resorting to full LLM-based compaction, collapse replaces low-priority
//! messages with one-line summaries to free tokens. Messages are scored by
//! importance (user=100, system=90, assistant varies by content type), and the
//! lowest-scored messages are collapsed first.

use grid_types::{ChatMessage, ContentBlock, MessageRole};
use tracing::debug;

/// Collapser that replaces low-importance messages with one-line summaries.
pub struct ContextCollapser {
    /// Number of recent conversation turns (user+assistant pairs) to protect.
    pub keep_recent_turns: usize,
}

impl Default for ContextCollapser {
    fn default() -> Self {
        Self {
            keep_recent_turns: 3,
        }
    }
}

impl ContextCollapser {
    /// Create a new collapser with a custom protection window.
    pub fn new(keep_recent_turns: usize) -> Self {
        Self { keep_recent_turns }
    }

    /// Collapse messages to free tokens, targeting `target_tokens` usage.
    ///
    /// Returns the number of messages collapsed.
    pub fn collapse(
        &self,
        messages: &mut Vec<ChatMessage>,
        target_tokens: usize,
        current_tokens: usize,
    ) -> usize {
        if current_tokens <= target_tokens {
            return 0;
        }

        let tokens_to_free = current_tokens - target_tokens;
        // Protect the last N*2 messages (user+assistant pairs)
        let protect_from = messages.len().saturating_sub(self.keep_recent_turns * 2);

        // Score all collapsible messages
        let mut scored: Vec<(usize, f32, usize)> = Vec::new(); // (index, score, est_tokens)
        for (i, msg) in messages.iter().enumerate() {
            if i >= protect_from {
                break; // Protect recent messages
            }
            let score = Self::score_message(msg);
            if score >= 90.0 {
                continue; // Never collapse user/system messages
            }
            let tokens = estimate_message_tokens(msg);
            if tokens < 50 {
                continue; // Not worth collapsing tiny messages
            }
            scored.push((i, score, tokens));
        }

        // Sort by score ascending (lowest = least important = collapsed first)
        scored.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut freed = 0usize;
        let mut collapsed_count = 0usize;
        for (idx, _score, tokens) in &scored {
            if freed >= tokens_to_free {
                break;
            }
            let summary = Self::make_summary(&messages[*idx]);
            let summary_tokens = summary.len() / 4;
            let savings = tokens.saturating_sub(summary_tokens);
            if savings == 0 {
                continue;
            }
            messages[*idx].content = vec![ContentBlock::Text { text: summary }];
            freed += savings;
            collapsed_count += 1;
        }

        if collapsed_count > 0 {
            debug!(
                collapsed_count,
                freed_tokens = freed,
                "Context collapse freed messages"
            );
        }

        collapsed_count
    }

    /// Score a message's importance (0-100, higher = more important).
    pub fn score_message(msg: &ChatMessage) -> f32 {
        match msg.role {
            MessageRole::User => 100.0,   // User messages: never collapse
            MessageRole::System => 90.0,  // System messages: almost never
            MessageRole::Assistant => {
                let text = msg.text_content();
                let has_code =
                    text.contains("```") || text.contains("fn ") || text.contains("def ");
                let has_error =
                    text.to_lowercase().contains("error") || text.contains("fix");

                let is_tool_result = msg
                    .content
                    .iter()
                    .any(|b| matches!(b, ContentBlock::ToolResult { .. }));
                let result_len: usize = msg
                    .content
                    .iter()
                    .filter_map(|b| {
                        if let ContentBlock::ToolResult { content, .. } = b {
                            Some(content.len())
                        } else {
                            None
                        }
                    })
                    .sum();

                match (is_tool_result, result_len) {
                    (true, len) if len > 2000 => 20.0, // Large tool results: collapse first
                    (true, len) if len > 500 => 40.0,  // Medium tool results
                    (true, _) => 60.0,                  // Small tool results
                    _ if has_code => 80.0,              // Code responses: protect
                    _ if has_error => 70.0,             // Error info: somewhat protect
                    _ => 50.0,                          // Regular text
                }
            }
        }
    }

    /// Generate a one-line summary for a message.
    fn make_summary(msg: &ChatMessage) -> String {
        match msg.role {
            MessageRole::Assistant => {
                let tool_names: Vec<&str> = msg
                    .content
                    .iter()
                    .filter_map(|b| {
                        if let ContentBlock::ToolUse { name, .. } = b {
                            Some(name.as_str())
                        } else {
                            None
                        }
                    })
                    .collect();

                if !tool_names.is_empty() {
                    let result_lens: Vec<usize> = msg
                        .content
                        .iter()
                        .filter_map(|b| {
                            if let ContentBlock::ToolResult { content, .. } = b {
                                Some(content.len())
                            } else {
                                None
                            }
                        })
                        .collect();
                    let total_chars: usize = result_lens.iter().sum();
                    format!(
                        "[Collapsed: {}() \u{2192} {} chars output]",
                        tool_names.join(", "),
                        total_chars
                    )
                } else {
                    let text = msg.text_content();
                    let preview = if text.len() > 80 {
                        &text[..80]
                    } else {
                        &text
                    };
                    format!("[Collapsed: {}...]", preview.trim())
                }
            }
            _ => {
                // Should not reach here (user/system are never collapsed)
                let text = msg.text_content();
                let preview = if text.len() > 60 {
                    &text[..60]
                } else {
                    &text
                };
                format!("[Collapsed: {}...]", preview.trim())
            }
        }
    }
}

/// Rough token estimate for a message (chars / 4).
fn estimate_message_tokens(msg: &ChatMessage) -> usize {
    msg.content
        .iter()
        .map(|b| match b {
            ContentBlock::Text { text } => text.len() / 4,
            ContentBlock::ToolUse { input, name, id } => {
                (name.len() + id.len() + input.to_string().len()) / 4
            }
            ContentBlock::ToolResult { content, .. } => content.len() / 4,
            ContentBlock::Image { data, .. } => data.len() / 100, // Images are pre-processed
            _ => 20, // Default estimate for other types
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_msg(role: MessageRole, text: &str) -> ChatMessage {
        ChatMessage {
            role,
            content: vec![ContentBlock::Text {
                text: text.to_string(),
            }],
        }
    }

    fn make_tool_result_msg(tool_name: &str, result: &str) -> ChatMessage {
        ChatMessage {
            role: MessageRole::Assistant,
            content: vec![
                ContentBlock::ToolUse {
                    id: "t1".to_string(),
                    name: tool_name.to_string(),
                    input: serde_json::json!({}),
                },
                ContentBlock::ToolResult {
                    tool_use_id: "t1".to_string(),
                    content: result.to_string(),
                    is_error: false,
                },
            ],
        }
    }

    #[test]
    fn test_score_user_message() {
        let msg = make_msg(MessageRole::User, "hello");
        assert_eq!(ContextCollapser::score_message(&msg), 100.0);
    }

    #[test]
    fn test_score_system_message() {
        let msg = make_msg(MessageRole::System, "system prompt");
        assert_eq!(ContextCollapser::score_message(&msg), 90.0);
    }

    #[test]
    fn test_score_large_tool_result() {
        let big_result = "x".repeat(3000);
        let msg = make_tool_result_msg("bash", &big_result);
        assert_eq!(ContextCollapser::score_message(&msg), 20.0);
    }

    #[test]
    fn test_score_code_response() {
        let msg = make_msg(MessageRole::Assistant, "Here is the fix:\n```rust\nfn main() {}\n```");
        assert_eq!(ContextCollapser::score_message(&msg), 80.0);
    }

    #[test]
    fn test_collapse_frees_tokens() {
        let mut messages = vec![
            make_msg(MessageRole::User, "search for files"),
            make_tool_result_msg("bash", &"line\n".repeat(500)),
            make_msg(MessageRole::User, "now edit"),
            make_tool_result_msg("file_read", &"content\n".repeat(500)),
            make_msg(MessageRole::User, "recent question"),
            make_msg(MessageRole::Assistant, "recent answer"),
        ];

        let collapser = ContextCollapser::new(1); // protect last 1 turn
        let current = 5000;
        let target = 2000;
        let collapsed = collapser.collapse(&mut messages, target, current);

        assert!(collapsed > 0, "Should have collapsed some messages");
        // Check that collapsed messages contain summary markers
        let has_collapsed = messages.iter().any(|m| m.text_content().contains("[Collapsed:"));
        assert!(has_collapsed, "Should have collapsed markers");
    }

    #[test]
    fn test_collapse_protects_recent() {
        let mut messages = vec![
            make_msg(MessageRole::User, "old question"),
            make_tool_result_msg("bash", &"x".repeat(1000)),
            make_msg(MessageRole::User, "recent"),
            make_msg(MessageRole::Assistant, "recent answer"),
        ];

        let collapser = ContextCollapser::new(1); // protect last 1 turn
        collapser.collapse(&mut messages, 100, 5000);

        // Recent turn should NOT be collapsed
        assert!(
            !messages[2].text_content().contains("[Collapsed:"),
            "Recent user msg should be protected"
        );
        assert!(
            !messages[3].text_content().contains("[Collapsed:"),
            "Recent assistant msg should be protected"
        );
    }

    #[test]
    fn test_collapse_no_op_when_under_target() {
        let mut messages = vec![
            make_msg(MessageRole::User, "hi"),
            make_msg(MessageRole::Assistant, "hello"),
        ];
        let collapser = ContextCollapser::default();
        let collapsed = collapser.collapse(&mut messages, 5000, 100);
        assert_eq!(collapsed, 0);
    }

    #[test]
    fn test_collapse_never_collapses_user() {
        let mut messages = vec![
            make_msg(MessageRole::User, &"important question ".repeat(100)),
            make_msg(MessageRole::Assistant, &"x".repeat(2000)),
            make_msg(MessageRole::User, "latest"),
            make_msg(MessageRole::Assistant, "latest answer"),
        ];
        let collapser = ContextCollapser::new(1);
        collapser.collapse(&mut messages, 100, 5000);

        // User messages should never be collapsed
        assert!(
            !messages[0].text_content().contains("[Collapsed:"),
            "User messages should never be collapsed"
        );
    }
}
