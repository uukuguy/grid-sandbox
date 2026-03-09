use octo_types::StopReason;

/// Structured return result for AgentLoop (Opus §3.2)
#[derive(Debug, Clone, Default)]
pub struct AgentLoopResult {
    pub rounds: u32,
    pub tool_calls: u32,
    pub stop_reason: NormalizedStopReason,
}

/// Normalized stop reason (ZeroClaw pattern) — covers all agent-level stop reasons.
/// This is distinct from octo_types::StopReason which is provider-level.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum NormalizedStopReason {
    #[default]
    EndTurn,
    ToolCall,
    MaxTokens,
    MaxIterations,
    ContextOverflow,
    SafetyBlocked,
    Cancelled,
    Error,
}

impl From<StopReason> for NormalizedStopReason {
    fn from(sr: StopReason) -> Self {
        match sr {
            StopReason::EndTurn => Self::EndTurn,
            StopReason::ToolUse => Self::ToolCall,
            StopReason::MaxTokens => Self::MaxTokens,
            StopReason::StopSequence => Self::EndTurn,
        }
    }
}

impl From<Option<StopReason>> for NormalizedStopReason {
    fn from(sr: Option<StopReason>) -> Self {
        match sr {
            Some(r) => r.into(),
            None => Self::EndTurn,
        }
    }
}

impl NormalizedStopReason {
    /// Parse from a raw string (as returned by some providers).
    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "end_turn" | "stop" => Self::EndTurn,
            "tool_use" | "tool_calls" => Self::ToolCall,
            "max_tokens" | "length" => Self::MaxTokens,
            "stop_sequence" | "content_filter" => Self::EndTurn,
            _ => Self::EndTurn,
        }
    }

    /// Whether this stop reason indicates the turn is complete
    /// (no further processing needed).
    pub fn is_terminal(&self) -> bool {
        !matches!(self, Self::ToolCall | Self::MaxTokens)
    }
}
