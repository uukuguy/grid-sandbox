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

/// Automatic failure root-cause classifier.
/// Analyzes timeline events and score to determine why a task failed.
pub struct FailureClassifier;

impl FailureClassifier {
    /// Classify the failure reason from timeline events and score.
    /// Returns None if the task passed.
    pub fn classify(
        timeline: &[crate::trace::TraceEvent],
        score: &crate::score::EvalScore,
    ) -> Option<FailureClass> {
        use crate::trace::TraceEvent;

        if score.passed {
            return None;
        }

        // 1. Empty timeline → EmptyOutput
        if timeline.is_empty() {
            return Some(FailureClass::EmptyOutput);
        }

        // 2. Check for network errors
        for event in timeline {
            if let TraceEvent::Error {
                source, message, ..
            } = event
            {
                let msg_lower = message.to_lowercase();
                if msg_lower.contains("network")
                    || msg_lower.contains("connection")
                    || msg_lower.contains("dns")
                    || msg_lower.contains("tcp")
                    || msg_lower.contains("ssl")
                    || msg_lower.contains("tls")
                {
                    return Some(FailureClass::NetworkError {
                        provider: source.clone(),
                        error: message.clone(),
                    });
                }
                if msg_lower.contains("rate limit")
                    || msg_lower.contains("429")
                    || msg_lower.contains("too many requests")
                {
                    return Some(FailureClass::ProviderRateLimit {
                        provider: source.clone(),
                    });
                }
            }
        }

        // 3. Check for timeout (last event or score details)
        if let crate::score::ScoreDetails::Timeout { elapsed_secs } = &score.details {
            let last_event = timeline
                .last()
                .map(|e| format!("{:?}", std::mem::discriminant(e)))
                .unwrap_or_else(|| "none".into());
            return Some(FailureClass::Timeout {
                elapsed_secs: *elapsed_secs,
                last_event,
            });
        }

        // 4. Check for security blocks
        let security_blocks: Vec<&TraceEvent> = timeline
            .iter()
            .filter(|e| matches!(e, TraceEvent::SecurityBlocked { .. }))
            .collect();
        if !security_blocks.is_empty() {
            if let TraceEvent::SecurityBlocked { tool, .. } = &security_blocks[0] {
                return Some(FailureClass::SecurityOverblocked {
                    tool: tool.clone(),
                });
            }
        }

        // 5. Check for loop detection (LoopGuardVerdict with "block")
        for event in timeline {
            if let TraceEvent::LoopGuardVerdict {
                verdict, ..
            } = event
            {
                if verdict == "block" {
                    let tool_name = Self::most_common_tool(timeline);
                    let count = Self::tool_call_count(timeline, &tool_name);
                    return Some(FailureClass::LoopDetected {
                        tool: tool_name,
                        count,
                    });
                }
            }
        }

        // 6. Check for context degradation
        for event in timeline {
            if let TraceEvent::ContextDegraded { stage, .. } = event {
                return Some(FailureClass::ContextOverflow {
                    degradation_stage: stage.clone(),
                });
            }
        }

        // 7. Check tool mismatch (based on ScoreDetails)
        match &score.details {
            crate::score::ScoreDetails::ToolCallMatch {
                expected_tool,
                actual_tool,
                ..
            } => {
                if let Some(actual) = actual_tool {
                    if actual != expected_tool {
                        return Some(FailureClass::WrongTool {
                            expected: expected_tool.clone(),
                            actual: actual.clone(),
                        });
                    }
                    return Some(FailureClass::WrongArgs {
                        tool: expected_tool.clone(),
                        mismatch: "argument mismatch".into(),
                    });
                }
                return Some(FailureClass::WrongTool {
                    expected: expected_tool.clone(),
                    actual: "none".into(),
                });
            }
            crate::score::ScoreDetails::FunctionCallMatch {
                expected_call,
                actual_tool,
                ..
            } => {
                if let Some(actual) = actual_tool {
                    if actual != expected_call {
                        return Some(FailureClass::WrongTool {
                            expected: expected_call.clone(),
                            actual: actual.clone(),
                        });
                    }
                    return Some(FailureClass::WrongArgs {
                        tool: expected_call.clone(),
                        mismatch: "argument mismatch".into(),
                    });
                }
                return Some(FailureClass::WrongTool {
                    expected: expected_call.clone(),
                    actual: "none".into(),
                });
            }
            crate::score::ScoreDetails::AstMatch {
                expected_tool,
                actual_tool,
                mismatched_fields,
                ..
            } => {
                if let Some(actual) = actual_tool {
                    if actual != expected_tool {
                        return Some(FailureClass::WrongTool {
                            expected: expected_tool.clone(),
                            actual: actual.clone(),
                        });
                    }
                    return Some(FailureClass::WrongArgs {
                        tool: expected_tool.clone(),
                        mismatch: format!(
                            "mismatched fields: {}",
                            mismatched_fields.join(", ")
                        ),
                    });
                }
                return Some(FailureClass::WrongTool {
                    expected: expected_tool.clone(),
                    actual: "none".into(),
                });
            }
            _ => {}
        }

        // 8. Check for reasoning errors (has Thinking events but still failed)
        let has_thinking = timeline
            .iter()
            .any(|e| matches!(e, TraceEvent::Thinking { .. }));
        if has_thinking {
            let snippet = timeline
                .iter()
                .filter_map(|e| {
                    if let TraceEvent::Thinking { content, .. } = e {
                        Some(content.as_str())
                    } else {
                        None
                    }
                })
                .last()
                .unwrap_or("")
                .chars()
                .take(200)
                .collect::<String>();
            return Some(FailureClass::ReasoningError {
                thinking_snippet: snippet,
            });
        }

        // 9. Fallback: InsufficientRounds
        let rounds = timeline
            .iter()
            .filter(|e| matches!(e, TraceEvent::RoundStart { .. }))
            .count() as u32;
        Some(FailureClass::InsufficientRounds {
            used: rounds,
            needed_estimate: rounds + 2,
        })
    }

    fn most_common_tool(timeline: &[crate::trace::TraceEvent]) -> String {
        use std::collections::HashMap;
        let mut counts: HashMap<&str, u32> = HashMap::new();
        for event in timeline {
            if let crate::trace::TraceEvent::ToolCall { tool_name, .. } = event {
                *counts.entry(tool_name.as_str()).or_default() += 1;
            }
        }
        counts
            .into_iter()
            .max_by_key(|(_, c)| *c)
            .map(|(name, _)| name.to_string())
            .unwrap_or_else(|| "unknown".into())
    }

    fn tool_call_count(timeline: &[crate::trace::TraceEvent], tool: &str) -> u32 {
        timeline
            .iter()
            .filter(|e| {
                matches!(e, crate::trace::TraceEvent::ToolCall { tool_name, .. } if tool_name == tool)
            })
            .count() as u32
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

    #[test]
    fn test_classifier_passed_returns_none() {
        use crate::score::{EvalScore, ScoreDetails};
        let score = EvalScore::pass(1.0, ScoreDetails::Custom { message: "ok".into() });
        assert!(FailureClassifier::classify(&[], &score).is_none());
    }

    #[test]
    fn test_classifier_empty_timeline() {
        use crate::score::{EvalScore, ScoreDetails};
        let score = EvalScore::fail(0.0, ScoreDetails::Custom { message: "fail".into() });
        let result = FailureClassifier::classify(&[], &score);
        assert!(matches!(result, Some(FailureClass::EmptyOutput)));
    }

    #[test]
    fn test_classifier_network_error() {
        use crate::score::{EvalScore, ScoreDetails};
        use crate::trace::TraceEvent;
        let timeline = vec![TraceEvent::Error {
            round: 1,
            source: "llm".into(),
            message: "network connection refused".into(),
        }];
        let score = EvalScore::fail(0.0, ScoreDetails::Custom { message: "fail".into() });
        let result = FailureClassifier::classify(&timeline, &score);
        assert!(matches!(result, Some(FailureClass::NetworkError { .. })));
    }

    #[test]
    fn test_classifier_timeout() {
        use crate::score::{EvalScore, ScoreDetails};
        use crate::trace::TraceEvent;
        let timeline = vec![TraceEvent::RoundStart {
            round: 1,
            timestamp_ms: 0,
        }];
        let score = EvalScore::fail(0.0, ScoreDetails::Timeout { elapsed_secs: 120 });
        let result = FailureClassifier::classify(&timeline, &score);
        assert!(matches!(
            result,
            Some(FailureClass::Timeout {
                elapsed_secs: 120,
                ..
            })
        ));
    }

    #[test]
    fn test_classifier_wrong_tool() {
        use crate::score::{EvalScore, ScoreDetails};
        use crate::trace::TraceEvent;
        let timeline = vec![TraceEvent::ToolCall {
            round: 1,
            tool_name: "file_read".into(),
            input: serde_json::json!({}),
            output: "".into(),
            success: true,
            duration_ms: 10,
        }];
        let score = EvalScore::fail(
            0.0,
            ScoreDetails::ToolCallMatch {
                expected_tool: "bash".into(),
                actual_tool: Some("file_read".into()),
                arg_match_rate: 0.0,
            },
        );
        let result = FailureClassifier::classify(&timeline, &score);
        assert!(matches!(result, Some(FailureClass::WrongTool { .. })));
    }

    #[test]
    fn test_classifier_insufficient_rounds() {
        use crate::score::{EvalScore, ScoreDetails};
        use crate::trace::TraceEvent;
        let timeline = vec![
            TraceEvent::RoundStart {
                round: 1,
                timestamp_ms: 0,
            },
            TraceEvent::Completed {
                rounds: 1,
                stop_reason: "EndTurn".into(),
                total_duration_ms: 100,
            },
        ];
        let score = EvalScore::fail(0.0, ScoreDetails::Custom { message: "fail".into() });
        let result = FailureClassifier::classify(&timeline, &score);
        assert!(matches!(
            result,
            Some(FailureClass::InsufficientRounds { used: 1, .. })
        ));
    }
}
