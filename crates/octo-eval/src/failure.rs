//! Failure classification for automatic root-cause attribution of eval failures.

use serde::{Deserialize, Serialize};

/// Classification of why an evaluation task failed.
/// Used for automatic root-cause attribution — separates infrastructure issues
/// from harness bugs from real capability gaps.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "class")]
pub enum FailureClass {
    // ── Infrastructure issues (don't reflect model capability) ──

    /// Network/connection error to LLM provider
    NetworkError {
        provider: String,
        error: String,
    },

    /// Task exceeded timeout
    Timeout {
        elapsed_secs: u64,
        last_event: String,
    },

    /// Provider rate limit hit (429)
    ProviderRateLimit {
        provider: String,
    },

    // ── Harness/scorer issues (possible eval framework bugs) ──

    /// Scorer expected format doesn't match actual output format
    ScorerMismatch {
        expected: String,
        actual: String,
    },

    /// Agent produced no output at all
    EmptyOutput,

    /// Eval framework itself errored
    HarnessError {
        message: String,
    },

    // ── Real capability gaps (valuable signal) ──

    /// Agent used the wrong tool
    WrongTool {
        expected: String,
        actual: String,
    },

    /// Agent used correct tool but wrong arguments
    WrongArgs {
        tool: String,
        mismatch: String,
    },

    /// Agent's reasoning was incorrect
    ReasoningError {
        thinking_snippet: String,
    },

    /// Security policy should have blocked but didn't
    SecurityBypassed {
        tool: String,
    },

    /// Security policy blocked something that should have been allowed
    SecurityOverblocked {
        tool: String,
    },

    /// Context window overflow caused degradation
    ContextOverflow {
        degradation_stage: String,
    },

    /// Agent entered a loop (repeated same tool calls)
    LoopDetected {
        tool: String,
        count: u32,
    },

    /// Agent didn't have enough rounds to complete
    InsufficientRounds {
        used: u32,
        needed_estimate: u32,
    },
}

impl FailureClass {
    /// Returns the broad category of this failure
    pub fn category(&self) -> &'static str {
        match self {
            Self::NetworkError { .. }
            | Self::Timeout { .. }
            | Self::ProviderRateLimit { .. } => "infrastructure",

            Self::ScorerMismatch { .. }
            | Self::EmptyOutput
            | Self::HarnessError { .. } => "harness",

            _ => "capability",
        }
    }

    /// Returns a short label for display
    pub fn label(&self) -> &'static str {
        match self {
            Self::NetworkError { .. } => "network_error",
            Self::Timeout { .. } => "timeout",
            Self::ProviderRateLimit { .. } => "rate_limit",
            Self::ScorerMismatch { .. } => "scorer_mismatch",
            Self::EmptyOutput => "empty_output",
            Self::HarnessError { .. } => "harness_error",
            Self::WrongTool { .. } => "wrong_tool",
            Self::WrongArgs { .. } => "wrong_args",
            Self::ReasoningError { .. } => "reasoning_error",
            Self::SecurityBypassed { .. } => "security_bypassed",
            Self::SecurityOverblocked { .. } => "security_overblocked",
            Self::ContextOverflow { .. } => "context_overflow",
            Self::LoopDetected { .. } => "loop_detected",
            Self::InsufficientRounds { .. } => "insufficient_rounds",
        }
    }
}

impl std::fmt::Display for FailureClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NetworkError { provider, error } => write!(f, "Network error ({}): {}", provider, error),
            Self::Timeout { elapsed_secs, .. } => write!(f, "Timeout after {}s", elapsed_secs),
            Self::ProviderRateLimit { provider } => write!(f, "Rate limited by {}", provider),
            Self::ScorerMismatch { expected, actual } => write!(f, "Scorer mismatch: expected '{}', got '{}'", expected, actual),
            Self::EmptyOutput => write!(f, "Empty output"),
            Self::HarnessError { message } => write!(f, "Harness error: {}", message),
            Self::WrongTool { expected, actual } => write!(f, "Wrong tool: expected '{}', used '{}'", expected, actual),
            Self::WrongArgs { tool, mismatch } => write!(f, "Wrong args for '{}': {}", tool, mismatch),
            Self::ReasoningError { thinking_snippet } => write!(f, "Reasoning error: {}", thinking_snippet),
            Self::SecurityBypassed { tool } => write!(f, "Security bypassed for '{}'", tool),
            Self::SecurityOverblocked { tool } => write!(f, "Security overblocked '{}'", tool),
            Self::ContextOverflow { degradation_stage } => write!(f, "Context overflow at stage '{}'", degradation_stage),
            Self::LoopDetected { tool, count } => write!(f, "Loop detected: '{}' called {} times", tool, count),
            Self::InsufficientRounds { used, needed_estimate } => write!(f, "Insufficient rounds: used {}, estimated need {}", used, needed_estimate),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_failure_class_categories() {
        assert_eq!(FailureClass::NetworkError { provider: "openai".into(), error: "connection refused".into() }.category(), "infrastructure");
        assert_eq!(FailureClass::Timeout { elapsed_secs: 120, last_event: "ToolCall".into() }.category(), "infrastructure");
        assert_eq!(FailureClass::ProviderRateLimit { provider: "anthropic".into() }.category(), "infrastructure");
        assert_eq!(FailureClass::ScorerMismatch { expected: "a".into(), actual: "b".into() }.category(), "harness");
        assert_eq!(FailureClass::EmptyOutput.category(), "harness");
        assert_eq!(FailureClass::HarnessError { message: "oops".into() }.category(), "harness");
        assert_eq!(FailureClass::WrongTool { expected: "bash".into(), actual: "file_read".into() }.category(), "capability");
        assert_eq!(FailureClass::LoopDetected { tool: "bash".into(), count: 5 }.category(), "capability");
    }

    #[test]
    fn test_failure_class_labels() {
        assert_eq!(FailureClass::EmptyOutput.label(), "empty_output");
        assert_eq!(FailureClass::WrongTool { expected: "a".into(), actual: "b".into() }.label(), "wrong_tool");
        assert_eq!(FailureClass::InsufficientRounds { used: 3, needed_estimate: 5 }.label(), "insufficient_rounds");
    }

    #[test]
    fn test_failure_class_serialization() {
        let fc = FailureClass::WrongTool { expected: "bash".into(), actual: "file_read".into() };
        let json = serde_json::to_string(&fc).unwrap();
        assert!(json.contains("\"class\":\"WrongTool\""));
        let deserialized: FailureClass = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, fc);
    }

    #[test]
    fn test_failure_class_display() {
        let fc = FailureClass::Timeout { elapsed_secs: 120, last_event: "ToolCall".into() };
        assert_eq!(format!("{}", fc), "Timeout after 120s");
    }

    #[test]
    fn test_all_variants_serialize_roundtrip() {
        let variants = vec![
            FailureClass::NetworkError { provider: "p".into(), error: "e".into() },
            FailureClass::Timeout { elapsed_secs: 1, last_event: "x".into() },
            FailureClass::ProviderRateLimit { provider: "p".into() },
            FailureClass::ScorerMismatch { expected: "a".into(), actual: "b".into() },
            FailureClass::EmptyOutput,
            FailureClass::HarnessError { message: "m".into() },
            FailureClass::WrongTool { expected: "a".into(), actual: "b".into() },
            FailureClass::WrongArgs { tool: "t".into(), mismatch: "m".into() },
            FailureClass::ReasoningError { thinking_snippet: "s".into() },
            FailureClass::SecurityBypassed { tool: "t".into() },
            FailureClass::SecurityOverblocked { tool: "t".into() },
            FailureClass::ContextOverflow { degradation_stage: "s".into() },
            FailureClass::LoopDetected { tool: "t".into(), count: 3 },
            FailureClass::InsufficientRounds { used: 2, needed_estimate: 5 },
        ];
        for v in &variants {
            let json = serde_json::to_string(v).unwrap();
            let back: FailureClass = serde_json::from_str(&json).unwrap();
            assert_eq!(&back, v);
        }
    }
}
