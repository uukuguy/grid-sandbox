//! Session summarizer — generates concise session summaries using LLM.
//!
//! At session end, [`SessionSummarizer`] takes the conversation history and
//! produces a structured summary including key topics, event count, and a
//! human-readable text summary for cross-session context injection.

use anyhow::Result;
use tracing::{debug, warn};

use octo_types::{ChatMessage, CompletionRequest, ContentBlock};

use crate::providers::Provider;

/// Result of session summarization.
#[derive(Debug, Clone)]
pub struct SummaryResult {
    /// Human-readable summary text (2-3 sentences).
    pub text: String,
    /// Key topics discussed in the session.
    pub key_topics: Vec<String>,
    /// Number of significant events/actions in the session.
    pub event_count: usize,
}

/// Generates session summaries from conversation history using LLM.
pub struct SessionSummarizer;

impl SessionSummarizer {
    /// Generate a structured summary of the conversation.
    ///
    /// Uses the provided LLM to analyze the full conversation and produce
    /// a concise summary with key topics and event count.
    pub async fn summarize(
        provider: &dyn Provider,
        messages: &[ChatMessage],
        model: &str,
    ) -> Result<SummaryResult> {
        if messages.is_empty() {
            return Ok(SummaryResult {
                text: String::new(),
                key_topics: vec![],
                event_count: 0,
            });
        }

        let conversation_text = Self::build_conversation_summary(messages);
        let prompt = Self::build_summarization_prompt(&conversation_text);

        let request = CompletionRequest {
            model: model.to_string(),
            system: Some("You are a concise summarizer. Output valid JSON only.".into()),
            messages: vec![ChatMessage::user(prompt)],
            max_tokens: 1024,
            temperature: Some(0.0),
            ..Default::default()
        };

        let response = provider.complete(request).await?;
        let text = response
            .content
            .iter()
            .filter_map(|b| match b {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<String>();

        Self::parse_summary(&text)
    }

    /// Build a condensed text representation of the conversation for the LLM.
    fn build_conversation_summary(messages: &[ChatMessage]) -> String {
        let mut parts = Vec::new();
        let max_message_chars = 500;

        for msg in messages {
            let role = match msg.role {
                octo_types::MessageRole::User => "User",
                octo_types::MessageRole::Assistant => "Assistant",
                octo_types::MessageRole::System => continue, // skip system messages
            };

            let mut text_parts = Vec::new();
            let mut tool_names = Vec::new();

            for block in &msg.content {
                match block {
                    ContentBlock::Text { text } => {
                        if !text.is_empty() {
                            let truncated = if text.len() > max_message_chars {
                                format!("{}...", &text[..max_message_chars])
                            } else {
                                text.clone()
                            };
                            text_parts.push(truncated);
                        }
                    }
                    ContentBlock::ToolUse { name, .. } => {
                        tool_names.push(name.as_str());
                    }
                    ContentBlock::ToolResult { content, is_error, .. } => {
                        let status = if *is_error { "ERROR" } else { "OK" };
                        let truncated = if content.len() > 100 {
                            format!("{}...", &content[..100])
                        } else {
                            content.clone()
                        };
                        text_parts.push(format!("[{status}] {truncated}"));
                    }
                    _ => {}
                }
            }

            if !tool_names.is_empty() {
                text_parts.push(format!("[Tools: {}]", tool_names.join(", ")));
            }

            if !text_parts.is_empty() {
                parts.push(format!("{role}: {}", text_parts.join(" ")));
            }
        }

        // Limit total length to ~4000 chars for the LLM
        let joined = parts.join("\n");
        if joined.len() > 4000 {
            // Keep first and last parts
            let half = 1800;
            format!(
                "{}\n\n... (conversation truncated) ...\n\n{}",
                &joined[..half],
                &joined[joined.len() - half..]
            )
        } else {
            joined
        }
    }

    fn build_summarization_prompt(conversation_text: &str) -> String {
        format!(
            r#"Summarize this conversation in 2-3 sentences. Include:
- What was done and the outcomes
- Key decisions made
- Any important artifacts (files, accounts, configs created or modified)

Conversation:
{conversation_text}

Return JSON: {{"text": "summary text", "key_topics": ["topic1", "topic2"], "event_count": N}}
where event_count is the number of significant actions/changes made.
Return ONLY the JSON object, no markdown fences or explanation."#
        )
    }

    /// Parse LLM response into SummaryResult.
    fn parse_summary(text: &str) -> Result<SummaryResult> {
        let cleaned = text
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        match serde_json::from_str::<RawSummary>(cleaned) {
            Ok(raw) => Ok(SummaryResult {
                text: raw.text,
                key_topics: raw.key_topics.unwrap_or_default(),
                event_count: raw.event_count.unwrap_or(0),
            }),
            Err(e) => {
                warn!("Failed to parse summary response: {e}");
                debug!("Raw response: {cleaned}");
                // Fallback: use the raw text as summary
                Ok(SummaryResult {
                    text: cleaned.to_string(),
                    key_topics: vec![],
                    event_count: 0,
                })
            }
        }
    }
}

/// Intermediate deserialization struct.
#[derive(serde::Deserialize)]
struct RawSummary {
    text: String,
    key_topics: Option<Vec<String>>,
    event_count: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_summary_valid() {
        let json = r#"{"text": "Fixed auth bug and deployed to staging.", "key_topics": ["auth", "deployment"], "event_count": 2}"#;
        let result = SessionSummarizer::parse_summary(json).unwrap();
        assert_eq!(result.text, "Fixed auth bug and deployed to staging.");
        assert_eq!(result.key_topics, vec!["auth", "deployment"]);
        assert_eq!(result.event_count, 2);
    }

    #[test]
    fn test_parse_summary_with_code_fences() {
        let json = "```json\n{\"text\": \"Summary here.\", \"key_topics\": [], \"event_count\": 0}\n```";
        let result = SessionSummarizer::parse_summary(json).unwrap();
        assert_eq!(result.text, "Summary here.");
    }

    #[test]
    fn test_parse_summary_missing_optional_fields() {
        let json = r#"{"text": "Just a summary."}"#;
        let result = SessionSummarizer::parse_summary(json).unwrap();
        assert_eq!(result.text, "Just a summary.");
        assert!(result.key_topics.is_empty());
        assert_eq!(result.event_count, 0);
    }

    #[test]
    fn test_parse_summary_invalid_json_fallback() {
        let result = SessionSummarizer::parse_summary("Not valid JSON at all").unwrap();
        assert_eq!(result.text, "Not valid JSON at all");
        assert!(result.key_topics.is_empty());
    }

    #[test]
    fn test_build_conversation_summary_empty() {
        let summary = SessionSummarizer::build_conversation_summary(&[]);
        assert!(summary.is_empty());
    }

    #[test]
    fn test_build_conversation_summary_basic() {
        let messages = vec![
            ChatMessage::user("Fix the login bug"),
            ChatMessage::assistant("I'll look into it."),
        ];
        let summary = SessionSummarizer::build_conversation_summary(&messages);
        assert!(summary.contains("User: Fix the login bug"));
        assert!(summary.contains("Assistant: I'll look into it."));
    }

    #[test]
    fn test_build_conversation_summary_skips_system() {
        let messages = vec![ChatMessage {
            role: octo_types::MessageRole::System,
            content: vec![ContentBlock::Text {
                text: "System prompt".into(),
            }],
        }];
        let summary = SessionSummarizer::build_conversation_summary(&messages);
        assert!(summary.is_empty());
    }

    #[test]
    fn test_summarize_empty_messages() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // We can't call summarize without a real provider, but we can test
            // that empty messages return an empty result without calling the LLM.
            // This is tested via the early return in summarize().
            let messages: Vec<ChatMessage> = vec![];
            // Verify the conversation summary is empty
            let summary = SessionSummarizer::build_conversation_summary(&messages);
            assert!(summary.is_empty());
        });
    }
}
