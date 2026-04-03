use grid_types::{ChatMessage, ContentBlock, ToolSpec};

const CHARS_PER_TOKEN: usize = 4;

/// 上下文降级级别（4+1 阶段，参考 CONTEXT_ENGINEERING_DESIGN.md §7.1）
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DegradationLevel {
    /// 使用率 < 60%：无需降级
    None,
    /// 使用率 60%-70%：工具结果头尾裁剪（预警性轻度干预）
    SoftTrim,
    /// 使用率 70%-90%：保留最近 10 条消息
    AutoCompaction,
    /// 使用率 > 90%：保留最近 4 条消息，触发 Memory Flush
    OverflowCompaction,
    /// 压缩后仍超限：截断当前工具结果至 8000 chars
    ToolResultTruncation,
    /// 全部手段失效：返回结构化错误，终止 Agent Loop
    FinalError,
}

#[derive(Clone)]
pub struct ContextBudgetManager {
    /// Model context window in tokens
    context_window: u32,
    /// Reserved for model output (default: 8192)
    output_reserve: u32,
    /// Safety margin (default: 2048)
    safety_margin: u32,
    /// Last actual input_tokens from API response (if available)
    last_actual_usage: Option<u64>,
    /// Message count when last_actual_usage was recorded
    last_usage_msg_count: usize,
}

impl ContextBudgetManager {
    pub fn new(context_window: u32) -> Self {
        Self {
            context_window,
            output_reserve: 8192,
            safety_margin: 2048,
            last_actual_usage: None,
            last_usage_msg_count: 0,
        }
    }

    pub fn with_output_reserve(mut self, reserve: u32) -> Self {
        self.output_reserve = reserve;
        self
    }

    /// Update with actual token usage from API response.
    pub fn update_actual_usage(&mut self, input_tokens: u32, msg_count: usize) {
        self.last_actual_usage = Some(input_tokens as u64);
        self.last_usage_msg_count = msg_count;
    }

    /// Estimate tokens for a string using chars/4 approximation.
    pub fn estimate_tokens(text: &str) -> u32 {
        (text.len() / CHARS_PER_TOKEN) as u32
    }

    /// Estimate tokens for all messages.
    pub fn estimate_messages_tokens(messages: &[ChatMessage]) -> u64 {
        messages
            .iter()
            .map(|m| {
                m.content
                    .iter()
                    .map(|b| match b {
                        ContentBlock::Text { text } => text.len() as u64 / CHARS_PER_TOKEN as u64,
                        ContentBlock::ToolUse { input, name, id } => {
                            (name.len() + id.len() + input.to_string().len()) as u64
                                / CHARS_PER_TOKEN as u64
                        }
                        ContentBlock::ToolResult { content, .. } => {
                            content.len() as u64 / CHARS_PER_TOKEN as u64
                        }
                        ContentBlock::Image { data, .. } => {
                            estimate_image_tokens_fixed(data.len()) as u64
                        }
                        ContentBlock::Document { data, .. } => {
                            data.len() as u64 / CHARS_PER_TOKEN as u64
                        }
                    })
                    .sum::<u64>()
            })
            .sum()
    }

    /// Estimate tokens for tool specs (they count against context window).
    ///
    /// Uses structured JSON-Schema parsing for more accurate estimation
    /// rather than naive string-length division.
    pub fn estimate_tool_specs_tokens(tools: &[ToolSpec]) -> u64 {
        estimate_tool_schema_tokens(tools)
    }

    /// Compute total estimated context usage using dual-track estimation.
    ///
    /// Track 1 (preferred): Use last actual API usage + estimate for new messages since then.
    /// Track 2 (fallback): Pure chars/4 estimation for everything.
    pub fn estimate_total_usage(
        &self,
        system_prompt: &str,
        messages: &[ChatMessage],
        tools: &[ToolSpec],
    ) -> u64 {
        // If we have actual usage data, use it as baseline
        if let Some(actual) = self.last_actual_usage {
            if messages.len() > self.last_usage_msg_count {
                let new_messages = &messages[self.last_usage_msg_count..];
                let new_tokens = Self::estimate_messages_tokens(new_messages);
                return actual + new_tokens;
            }
            return actual;
        }

        // Fallback: estimate everything
        let system_tokens = Self::estimate_tokens(system_prompt) as u64;
        let msg_tokens = Self::estimate_messages_tokens(messages);
        let tool_tokens = Self::estimate_tool_specs_tokens(tools);

        system_tokens + msg_tokens + tool_tokens
    }

    /// Available space for content (total - output_reserve - safety_margin).
    pub fn available_space(&self) -> u64 {
        (self.context_window as u64)
            .saturating_sub(self.output_reserve as u64)
            .saturating_sub(self.safety_margin as u64)
    }

    /// Compute usage ratio (0.0 - 1.0+).
    pub fn usage_ratio(
        &self,
        system_prompt: &str,
        messages: &[ChatMessage],
        tools: &[ToolSpec],
    ) -> f64 {
        let used = self.estimate_total_usage(system_prompt, messages, tools);
        let available = self.available_space();
        if available == 0 {
            return 1.0;
        }
        used as f64 / available as f64
    }

    /// Determine the degradation level based on current usage.
    ///
    /// 注意：ToolResultTruncation 和 FinalError 是升级触发的，不在此函数中返回。
    pub fn compute_degradation_level(
        &self,
        system_prompt: &str,
        messages: &[ChatMessage],
        tools: &[ToolSpec],
    ) -> DegradationLevel {
        let ratio = self.usage_ratio(system_prompt, messages, tools);
        match ratio {
            r if r < 0.60 => DegradationLevel::None,
            r if r < 0.70 => DegradationLevel::SoftTrim,
            r if r < 0.90 => DegradationLevel::AutoCompaction,
            _ => DegradationLevel::OverflowCompaction,
        }
    }

    pub fn context_window(&self) -> u32 {
        self.context_window
    }

    /// Produce a snapshot of the current token budget state.
    pub fn snapshot(
        &self,
        system_prompt: &str,
        messages: &[ChatMessage],
        tools: &[ToolSpec],
    ) -> grid_types::TokenBudgetSnapshot {
        let sys_tokens = Self::estimate_tokens(system_prompt) as usize;
        let history_tokens = Self::estimate_messages_tokens(messages) as usize;
        let tool_tokens = Self::estimate_tool_specs_tokens(tools) as usize;
        let total = self.context_window as usize;
        let used = sys_tokens + history_tokens + tool_tokens;
        let free = total.saturating_sub(used);
        let usage_pct = if total > 0 {
            (used as f32 / total as f32) * 100.0
        } else {
            0.0
        };

        let degradation = match self.compute_degradation_level(system_prompt, messages, tools) {
            DegradationLevel::None => 0,
            DegradationLevel::SoftTrim => 1,
            DegradationLevel::AutoCompaction => 2,
            DegradationLevel::OverflowCompaction => 3,
            DegradationLevel::ToolResultTruncation => 4,
            DegradationLevel::FinalError => 5,
        };

        grid_types::TokenBudgetSnapshot {
            total,
            system_prompt: sys_tokens,
            dynamic_context: tool_tokens,
            history: history_tokens,
            free,
            usage_percent: usage_pct,
            degradation_level: degradation,
        }
    }
}

impl Default for ContextBudgetManager {
    fn default() -> Self {
        Self::new(200_000)
    }
}

// ---------------------------------------------------------------------------
// Image token estimation (T7)
// ---------------------------------------------------------------------------

/// Fixed-tier image token estimation based on base64 data size.
///
/// Rather than naively dividing base64 length by 4 (which grossly
/// overestimates because raw bytes are not text tokens), we use
/// three fixed tiers aligned with Anthropic's documented image token costs.
///
/// - `0..=50_000` bytes   -> 85 tokens  (low-res thumbnail)
/// - `50_001..=500_000`   -> 1600 tokens (standard resolution)
/// - `> 500_000`          -> 3200 tokens (high resolution)
fn estimate_image_tokens_fixed(base64_len: usize) -> usize {
    match base64_len {
        0..=50_000 => 85,
        50_001..=500_000 => 1600,
        _ => 3200,
    }
}

// ---------------------------------------------------------------------------
// Schema token estimation (T9)
// ---------------------------------------------------------------------------

/// Estimate tokens for a collection of tool specs using structured
/// JSON-Schema parsing.
///
/// Reference: <https://docs.anthropic.com/en/docs/build-with-claude/tool-use#token-usage>
fn estimate_tool_schema_tokens(tools: &[ToolSpec]) -> u64 {
    tools.iter().map(|t| estimate_single_tool_tokens(t)).sum()
}

/// Estimate tokens consumed by a single tool definition.
///
/// Layout:
/// - `FUNC_INIT` (7) -- function header boilerplate
/// - `name_tokens` -- tool name
/// - `desc_tokens` -- tool description
/// - Per property: `PROP_OVERHEAD` (3) + key + description + type (1) + enum values
/// - Fallback: if no `"properties"` object, use raw JSON string length / 4
fn estimate_single_tool_tokens(tool: &ToolSpec) -> u64 {
    const FUNC_INIT: u64 = 7;
    const PROP_OVERHEAD: u64 = 3;
    const CPT: u64 = CHARS_PER_TOKEN as u64;

    let name_tokens = tool.name.len() as u64 / CPT + 1;
    let desc_tokens = tool.description.len() as u64 / CPT + 1;

    let prop_tokens = if let Some(props) = tool
        .input_schema
        .get("properties")
        .and_then(|v| v.as_object())
    {
        props
            .iter()
            .map(|(key, value)| {
                let key_tokens = key.len() as u64 / CPT + 1;
                let d_tokens = value
                    .get("description")
                    .and_then(|d| d.as_str())
                    .map(|d| d.len() as u64 / CPT + 1)
                    .unwrap_or(0);
                let enum_tokens = value
                    .get("enum")
                    .and_then(|e| e.as_array())
                    .map(|arr| arr.len() as u64 * 2)
                    .unwrap_or(0);
                // +1 for the type token
                PROP_OVERHEAD + key_tokens + d_tokens + 1 + enum_tokens
            })
            .sum::<u64>()
    } else {
        // Fallback: estimate from raw JSON string length
        tool.input_schema.to_string().len() as u64 / CPT
    };

    FUNC_INIT + name_tokens + desc_tokens + prop_tokens
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- T7: Image token estimation tests --

    #[test]
    fn test_image_token_estimation_small() {
        assert_eq!(estimate_image_tokens_fixed(0), 85);
        assert_eq!(estimate_image_tokens_fixed(1_000), 85);
        assert_eq!(estimate_image_tokens_fixed(50_000), 85);
    }

    #[test]
    fn test_image_token_estimation_large() {
        assert_eq!(estimate_image_tokens_fixed(50_001), 1600);
        assert_eq!(estimate_image_tokens_fixed(500_000), 1600);
        assert_eq!(estimate_image_tokens_fixed(500_001), 3200);
        assert_eq!(estimate_image_tokens_fixed(2_000_000), 3200);
    }

    // -- T9: Schema token estimation tests --

    #[test]
    fn test_schema_tokens_simple_tool() {
        let tool = ToolSpec {
            name: "read_file".into(),
            description: "Read content from a file on disk".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The file path to read"
                    },
                    "encoding": {
                        "type": "string",
                        "description": "Text encoding"
                    }
                },
                "required": ["path"]
            }),
        };
        let tokens = estimate_single_tool_tokens(&tool);
        assert!(tokens > 20, "Expected > 20 tokens, got {tokens}");
        assert!(tokens < 80, "Expected < 80 tokens, got {tokens}");
    }

    #[test]
    fn test_schema_tokens_complex_tool() {
        let tool = ToolSpec {
            name: "execute_command".into(),
            description: "Execute a shell command in the sandbox environment".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to execute"
                    },
                    "shell": {
                        "type": "string",
                        "description": "Shell to use",
                        "enum": ["bash", "zsh", "sh", "fish"]
                    },
                    "timeout": {
                        "type": "integer",
                        "description": "Timeout in seconds"
                    },
                    "working_dir": {
                        "type": "string",
                        "description": "Working directory for command execution"
                    }
                },
                "required": ["command"]
            }),
        };
        let tokens = estimate_single_tool_tokens(&tool);
        assert!(tokens > 40, "Expected > 40 tokens, got {tokens}");
        assert!(tokens < 150, "Expected < 150 tokens, got {tokens}");

        // Verify enum contributes extra tokens
        let simple = ToolSpec {
            name: tool.name.clone(),
            description: tool.description.clone(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to execute"
                    }
                }
            }),
        };
        let simple_tokens = estimate_single_tool_tokens(&simple);
        assert!(
            tokens > simple_tokens,
            "Complex tool ({tokens}) should have more tokens than simple ({simple_tokens})"
        );
    }

    #[test]
    fn test_schema_tokens_no_properties() {
        let tool = ToolSpec {
            name: "get_time".into(),
            description: "Return the current UTC time".into(),
            input_schema: serde_json::json!({
                "type": "object"
            }),
        };
        let tokens = estimate_single_tool_tokens(&tool);
        assert!(tokens > 10, "Expected > 10 tokens, got {tokens}");
        assert!(tokens < 50, "Expected < 50 tokens, got {tokens}");
    }
}
