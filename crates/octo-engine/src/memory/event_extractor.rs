//! Event extractor — extracts structured events from tool call chains using LLM.
//!
//! Scans conversation messages for ToolUse + ToolResult pairs and asks an LLM
//! to identify meaningful actions (not pure queries) and structure them as
//! [`EventData`] records for episodic memory storage.

use anyhow::Result;
use tracing::{debug, warn};

use octo_types::{ChatMessage, CompletionRequest, ContentBlock, EventData};

use crate::providers::Provider;

/// Extracts structured events from tool call chains in conversation history.
pub struct EventExtractor;

impl EventExtractor {
    /// Extract structured events from messages using LLM analysis.
    ///
    /// Filters messages for ToolUse/ToolResult content blocks, builds a
    /// summarized tool chain, then asks the LLM to identify meaningful
    /// actions (ignoring pure reads/queries).
    ///
    /// Returns an empty Vec if no tool calls found or LLM extraction fails.
    pub async fn extract_events(
        provider: &dyn Provider,
        messages: &[ChatMessage],
        model: &str,
    ) -> Result<Vec<EventData>> {
        let tool_chain_text = Self::build_tool_chain_summary(messages);
        if tool_chain_text.is_empty() {
            debug!("No tool calls found in conversation, skipping event extraction");
            return Ok(vec![]);
        }

        let prompt = Self::build_extraction_prompt(&tool_chain_text);

        let request = CompletionRequest {
            model: model.to_string(),
            system: Some("You are a structured data extractor. Output valid JSON only.".into()),
            messages: vec![ChatMessage::user(prompt)],
            max_tokens: 2048,
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

        Self::parse_events(&text)
    }

    /// Build a concise summary of tool calls and their results from messages.
    fn build_tool_chain_summary(messages: &[ChatMessage]) -> String {
        let mut entries = Vec::new();
        // Collect ToolUse blocks with their IDs
        let mut tool_uses: Vec<(String, String, String)> = Vec::new(); // (id, name, input_summary)
        let mut tool_results: Vec<(String, String, bool)> = Vec::new(); // (tool_use_id, content_summary, is_error)

        for msg in messages {
            for block in &msg.content {
                match block {
                    ContentBlock::ToolUse { id, name, input } => {
                        let input_summary = truncate_json(input, 200);
                        tool_uses.push((id.clone(), name.clone(), input_summary));
                    }
                    ContentBlock::ToolResult {
                        tool_use_id,
                        content,
                        is_error,
                    } => {
                        let content_summary = truncate_str(content, 300);
                        tool_results.push((tool_use_id.clone(), content_summary, *is_error));
                    }
                    _ => {}
                }
            }
        }

        // Match tool uses with their results
        for (id, name, input) in &tool_uses {
            let result = tool_results
                .iter()
                .find(|(tid, _, _)| tid == id)
                .map(|(_, content, is_err)| {
                    if *is_err {
                        format!("ERROR: {content}")
                    } else {
                        content.clone()
                    }
                })
                .unwrap_or_else(|| "(no result)".to_string());

            entries.push(format!(
                "- Tool: {name}\n  Input: {input}\n  Result: {result}"
            ));
        }

        entries.join("\n\n")
    }

    fn build_extraction_prompt(tool_chain_text: &str) -> String {
        format!(
            r#"Extract structured events from the following tool call results.
An event is an action with a clear outcome (not a read/query operation).
Skip pure read operations like file_read, grep, glob, web_fetch (unless they discover something significant).
Focus on actions that change state: file_write, bash commands that modify things, memory_store, etc.

Tool chain:
{tool_chain_text}

Return a JSON array of events. Each event has:
- event_type: string (e.g. "create", "modify", "delete", "deploy", "configure", "register", "install", "fix")
- target: string (what was acted on, e.g. "database schema", "auth module", "user account")
- outcome: string ("success", "failure", "partial")
- artifacts: object (key data produced, e.g. {{"file": "src/main.rs", "lines_changed": 42}})
- tool_chain: array of tool names used

If no meaningful events found, return an empty array: []
Return ONLY the JSON array, no markdown fences or explanation."#
        )
    }

    /// Parse LLM response text into EventData records.
    fn parse_events(text: &str) -> Result<Vec<EventData>> {
        // Strip markdown code fences if present
        let cleaned = text
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        if cleaned.is_empty() || cleaned == "[]" {
            return Ok(vec![]);
        }

        match serde_json::from_str::<Vec<RawEvent>>(cleaned) {
            Ok(raw_events) => {
                let events = raw_events
                    .into_iter()
                    .map(|r| EventData {
                        event_type: r.event_type,
                        target: r.target,
                        outcome: r.outcome,
                        artifacts: r.artifacts.unwrap_or(serde_json::json!({})),
                        tool_chain: r.tool_chain.unwrap_or_default(),
                    })
                    .collect();
                Ok(events)
            }
            Err(e) => {
                warn!("Failed to parse event extraction response: {e}");
                debug!("Raw response: {cleaned}");
                Ok(vec![])
            }
        }
    }
}

/// Intermediate deserialization struct (tolerant of missing optional fields).
#[derive(serde::Deserialize)]
struct RawEvent {
    event_type: String,
    target: String,
    outcome: String,
    artifacts: Option<serde_json::Value>,
    tool_chain: Option<Vec<String>>,
}

/// Truncate a JSON value to approximately `max_chars` characters.
fn truncate_json(value: &serde_json::Value, max_chars: usize) -> String {
    let s = value.to_string();
    truncate_str(&s, max_chars)
}

/// Truncate a string, adding "..." if truncated.
fn truncate_str(s: &str, max_chars: usize) -> String {
    if s.len() <= max_chars {
        s.to_string()
    } else {
        format!("{}...", &s[..max_chars])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_events_valid_json() {
        let json = r#"[
            {
                "event_type": "create",
                "target": "auth module",
                "outcome": "success",
                "artifacts": {"file": "src/auth.rs"},
                "tool_chain": ["file_write", "bash"]
            }
        ]"#;
        let events = EventExtractor::parse_events(json).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "create");
        assert_eq!(events[0].target, "auth module");
        assert_eq!(events[0].outcome, "success");
        assert_eq!(events[0].tool_chain, vec!["file_write", "bash"]);
    }

    #[test]
    fn test_parse_events_with_code_fences() {
        let json = "```json\n[{\"event_type\":\"fix\",\"target\":\"login bug\",\"outcome\":\"success\"}]\n```";
        let events = EventExtractor::parse_events(json).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "fix");
    }

    #[test]
    fn test_parse_events_empty_array() {
        let events = EventExtractor::parse_events("[]").unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn test_parse_events_empty_string() {
        let events = EventExtractor::parse_events("").unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn test_parse_events_invalid_json_returns_empty() {
        let events = EventExtractor::parse_events("not json").unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn test_parse_events_missing_optional_fields() {
        let json = r#"[{"event_type":"deploy","target":"staging","outcome":"success"}]"#;
        let events = EventExtractor::parse_events(json).unwrap();
        assert_eq!(events.len(), 1);
        assert!(events[0].tool_chain.is_empty());
        assert_eq!(events[0].artifacts, serde_json::json!({}));
    }

    #[test]
    fn test_build_tool_chain_summary_no_tools() {
        let messages = vec![
            ChatMessage::user("Hello"),
            ChatMessage::assistant("Hi there"),
        ];
        let summary = EventExtractor::build_tool_chain_summary(&messages);
        assert!(summary.is_empty());
    }

    #[test]
    fn test_build_tool_chain_summary_with_tools() {
        let messages = vec![ChatMessage {
            role: octo_types::MessageRole::Assistant,
            content: vec![ContentBlock::ToolUse {
                id: "tu_1".into(),
                name: "file_write".into(),
                input: serde_json::json!({"path": "test.rs", "content": "fn main() {}"}),
            }],
        }, ChatMessage {
            role: octo_types::MessageRole::User,
            content: vec![ContentBlock::ToolResult {
                tool_use_id: "tu_1".into(),
                content: "File written successfully".into(),
                is_error: false,
            }],
        }];
        let summary = EventExtractor::build_tool_chain_summary(&messages);
        assert!(summary.contains("file_write"));
        assert!(summary.contains("File written successfully"));
    }

    #[test]
    fn test_truncate_str() {
        assert_eq!(truncate_str("short", 10), "short");
        assert_eq!(truncate_str("a long string here", 6), "a long...");
    }
}
