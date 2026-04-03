//! TraceEvent timeline types for white-box evaluation tracing.

use serde::{Deserialize, Serialize};

/// A single event in the evaluation execution timeline.
/// Captures the full execution flow for debugging and failure analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TraceEvent {
    /// Agent starts a new iteration round
    RoundStart {
        round: u32,
        timestamp_ms: u64,
    },

    /// LLM call metadata (no full response content)
    LlmCall {
        round: u32,
        input_tokens: u64,
        output_tokens: u64,
        duration_ms: u64,
        model: String,
    },

    /// Agent's reasoning/thinking process (extended thinking)
    Thinking {
        round: u32,
        content: String,
    },

    /// Tool call with full input/output
    ToolCall {
        round: u32,
        tool_name: String,
        input: serde_json::Value,
        output: String,
        success: bool,
        duration_ms: u64,
    },

    /// Error event
    Error {
        round: u32,
        source: String,
        message: String,
    },

    /// Security policy blocked an action
    SecurityBlocked {
        round: u32,
        tool: String,
        risk_level: String,
        reason: String,
    },

    /// Context degradation occurred
    ContextDegraded {
        round: u32,
        stage: String,
        usage_pct: f32,
    },

    /// Token budget snapshot
    BudgetSnapshot {
        round: u32,
        input_used: u64,
        output_used: u64,
        limit: u64,
    },

    /// LoopGuard verdict
    LoopGuardVerdict {
        round: u32,
        verdict: String,
        reason: String,
    },

    /// Agent completed execution
    Completed {
        rounds: u32,
        stop_reason: String,
        total_duration_ms: u64,
    },
}

/// Truncate a string to the given max length, respecting UTF-8 char boundaries.
pub fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let boundary = s.floor_char_boundary(max_len);
        s[..boundary].to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_event_serialization() {
        let event = TraceEvent::RoundStart {
            round: 1,
            timestamp_ms: 1000,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"RoundStart\""));
        let deserialized: TraceEvent = serde_json::from_str(&json).unwrap();
        match deserialized {
            TraceEvent::RoundStart { round, .. } => assert_eq!(round, 1),
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_truncate_str_ascii() {
        assert_eq!(truncate_str("hello world", 5), "hello");
        assert_eq!(truncate_str("hi", 10), "hi");
    }

    #[test]
    fn test_truncate_str_unicode() {
        let s = "你好世界hello";
        let truncated = truncate_str(s, 6);
        // Each Chinese char is 3 bytes, so 6 bytes = 2 chars
        assert_eq!(truncated, "你好");
    }

    #[test]
    fn test_all_variants_serialize() {
        let events = vec![
            TraceEvent::RoundStart { round: 1, timestamp_ms: 0 },
            TraceEvent::LlmCall { round: 1, input_tokens: 100, output_tokens: 50, duration_ms: 500, model: "test".into() },
            TraceEvent::Thinking { round: 1, content: "thinking...".into() },
            TraceEvent::ToolCall { round: 1, tool_name: "bash".into(), input: serde_json::json!({"cmd": "ls"}), output: "file.txt".into(), success: true, duration_ms: 100 },
            TraceEvent::Error { round: 1, source: "llm".into(), message: "timeout".into() },
            TraceEvent::SecurityBlocked { round: 1, tool: "bash".into(), risk_level: "High".into(), reason: "blocked".into() },
            TraceEvent::ContextDegraded { round: 1, stage: "soft_trim".into(), usage_pct: 85.0 },
            TraceEvent::BudgetSnapshot { round: 1, input_used: 5000, output_used: 2000, limit: 10000 },
            TraceEvent::LoopGuardVerdict { round: 1, verdict: "allow".into(), reason: "ok".into() },
            TraceEvent::Completed { rounds: 3, stop_reason: "EndTurn".into(), total_duration_ms: 5000 },
        ];
        for event in &events {
            let json = serde_json::to_string(event).unwrap();
            let _: TraceEvent = serde_json::from_str(&json).unwrap();
        }
    }
}
