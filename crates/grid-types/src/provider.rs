use serde::{Deserialize, Serialize};

use crate::message::{ChatMessage, ContentBlock};
use crate::tool::ToolSpec;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub max_tokens: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndTurn,
    ToolUse,
    MaxTokens,
    StopSequence,
}

/// Provider-layer tool selection constraint.
///
/// Maps to OpenAI's `tool_choice` and Anthropic's `tool_choice` fields.
/// Used by grid-engine to enforce "the LLM MUST call a tool this turn"
/// after an intermediate-ack / workflow-continuation trigger (D87 / L2b).
///
/// Providers that don't support a given variant should fall back to `Auto`.
#[derive(Debug, Clone, PartialEq)]
pub enum ToolChoice {
    /// Default: LLM picks whether to call a tool or reply with text.
    Auto,
    /// Force the LLM to produce at least one tool_use block this turn.
    /// OpenAI: `"required"`. Anthropic: `{"type": "any"}`.
    Required,
    /// Force the LLM to call a specific tool by name.
    /// OpenAI: `{"type": "function", "function": {"name": "..."}}`.
    /// Anthropic: `{"type": "tool", "name": "..."}`.
    Specific(String),
    /// Forbid tool calls entirely (text-only response).
    /// OpenAI: `"none"`. Anthropic: `{"type": "none"}`.
    None,
}

impl Default for ToolChoice {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone)]
pub struct CompletionRequest {
    pub model: String,
    pub system: Option<String>,
    pub messages: Vec<ChatMessage>,
    pub max_tokens: u32,
    pub temperature: Option<f32>,
    pub tools: Vec<ToolSpec>,
    pub stream: bool,
    /// Tool selection constraint. `None` means provider default (equivalent
    /// to `ToolChoice::Auto`). Set to `Some(ToolChoice::Required)` to force
    /// tool use on a given turn.
    pub tool_choice: Option<ToolChoice>,
}

impl Default for CompletionRequest {
    fn default() -> Self {
        Self {
            model: "claude-sonnet-4-20250514".into(),
            system: None,
            messages: Vec::new(),
            max_tokens: 4096,
            temperature: None,
            tools: Vec::new(),
            stream: false,
            tool_choice: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompletionResponse {
    pub id: String,
    pub content: Vec<ContentBlock>,
    pub stop_reason: Option<StopReason>,
    pub usage: TokenUsage,
}

#[derive(Debug, Clone)]
pub enum StreamEvent {
    MessageStart {
        id: String,
    },
    TextDelta {
        text: String,
    },
    ThinkingDelta {
        text: String,
    },
    ToolUseStart {
        index: usize,
        id: String,
        name: String,
    },
    ToolUseInputDelta {
        index: usize,
        partial_json: String,
    },
    ToolUseComplete {
        index: usize,
        id: String,
        name: String,
        input: serde_json::Value,
    },
    MessageStop {
        stop_reason: StopReason,
        usage: TokenUsage,
    },
}
