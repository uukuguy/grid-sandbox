// Phase AS: Old SystemPromptBuilder and ContextBuilder removed (dead code).
// Active builder lives in system_prompt.rs (re-exported as NewSystemPromptBuilder).
// Only estimate_messages_tokens remains here.

/// Estimate total tokens used by messages (chars / 4 approximation)
pub fn estimate_messages_tokens(
    messages: &[octo_types::ChatMessage],
    tools: &[octo_types::ToolSpec],
) -> u32 {
    let msg_chars: usize = messages
        .iter()
        .map(|m| {
            m.content
                .iter()
                .map(|b| match b {
                    octo_types::ContentBlock::Text { text } => text.len(),
                    octo_types::ContentBlock::ToolUse { input, .. } => input.to_string().len(),
                    octo_types::ContentBlock::ToolResult { content, .. } => content.len(),
                    octo_types::ContentBlock::Image { data, .. } => data.len(),
                    octo_types::ContentBlock::Document { data, .. } => data.len(),
                })
                .sum::<usize>()
        })
        .sum();

    let tool_chars: usize = tools
        .iter()
        .map(|t| t.name.len() + t.description.len() + t.input_schema.to_string().len())
        .sum();

    ((msg_chars + tool_chars) / 4) as u32
}
