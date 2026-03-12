use octo_types::{ChatMessage, ContentBlock, MessageRole};
use tracing::{debug, info, warn};

use super::budget::DegradationLevel;

// ---------------------------------------------------------------------------
// Compaction strategies (W7-T2)
// ---------------------------------------------------------------------------

/// Strategy for compacting old messages when the context window fills up.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum CompactionStrategy {
    /// Simply truncate/remove old messages (current behavior, default).
    #[default]
    Truncate,
    /// Summarize old messages using an LLM (caller must perform the actual call).
    Summarize,
    /// Move old messages to workspace/session memory for later recall.
    MoveToWorkspace,
}

/// Configuration for compaction behavior.
#[derive(Debug, Clone)]
pub struct CompactionConfig {
    pub strategy: CompactionStrategy,
    /// Maximum tokens for the summary (only relevant for Summarize strategy).
    pub summary_max_tokens: usize,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            strategy: CompactionStrategy::Truncate,
            summary_max_tokens: 500,
        }
    }
}

/// Action returned by `plan_compaction()`. The caller is responsible for
/// executing async operations (LLM summarization, workspace storage).
#[derive(Debug, Clone, PartialEq)]
pub enum CompactionAction {
    /// Messages were truncated in-place (synchronous, already done).
    Truncated { removed_count: usize },
    /// Caller should summarize these messages and insert the summary at `insert_position`.
    NeedsSummarize {
        messages_to_summarize: Vec<ChatMessage>,
        insert_position: usize,
    },
    /// Caller should save these messages to workspace and insert a placeholder.
    NeedsWorkspaceSave {
        messages_to_save: Vec<ChatMessage>,
        insert_position: usize,
    },
    /// No compaction needed (too few messages).
    NoAction,
}

/// Marker embedded in messages containing skill content with `always: true`.
/// Messages containing this marker are exempt from pruning/compaction.
pub const SKILL_PROTECTED_MARKER: &str = "[SKILL:ALWAYS]";

const SOFT_TRIM_HEAD: usize = 1_500;
const SOFT_TRIM_TAIL: usize = 500;

/// 工具结果截断上限（ToolResultTruncation 阶段）
const TOOL_RESULT_TRUNCATION_CHARS: usize = 8_000;

/// AutoCompaction 阶段保留的最近消息数量
const AUTO_COMPACTION_KEEP: usize = 10;

/// OverflowCompaction 阶段保留的最近消息数量
const OVERFLOW_COMPACTION_KEEP: usize = 4;

/// Prunes conversation history based on degradation level.
/// Does NOT modify the most recent `protect_recent_rounds` rounds.
#[derive(Clone)]
pub struct ContextPruner {
    /// Number of recent agent rounds to protect from pruning.
    protect_recent_rounds: usize,
    /// Optional compaction configuration (W7-T2).
    compaction_config: Option<CompactionConfig>,
}

impl ContextPruner {
    pub fn new() -> Self {
        Self {
            protect_recent_rounds: 2,
            compaction_config: None,
        }
    }

    /// Set compaction configuration (builder pattern).
    pub fn with_compaction_config(mut self, config: CompactionConfig) -> Self {
        self.compaction_config = Some(config);
        self
    }

    /// Plan a compaction action based on the configured strategy.
    ///
    /// Returns a `CompactionAction` describing what the caller should do.
    /// The pruner does NOT perform async operations itself.
    ///
    /// Minimum threshold: at least 8 messages required before compaction activates.
    /// Keeps the most recent 50% of messages (minimum 4).
    #[allow(clippy::ptr_arg)]
    pub fn plan_compaction(&self, messages: &[ChatMessage]) -> CompactionAction {
        let config = match &self.compaction_config {
            Some(c) => c,
            None => return CompactionAction::NoAction,
        };

        const MIN_MESSAGES_FOR_COMPACTION: usize = 8;
        const MIN_KEEP: usize = 4;

        if messages.len() < MIN_MESSAGES_FOR_COMPACTION {
            return CompactionAction::NoAction;
        }

        // Keep at least 50% of messages, minimum MIN_KEEP
        let keep_count = (messages.len() / 2).max(MIN_KEEP);
        let boundary = messages.len().saturating_sub(keep_count);

        if boundary == 0 {
            return CompactionAction::NoAction;
        }

        // Collect messages to compact, skipping skill-protected ones
        let messages_to_compact: Vec<ChatMessage> = messages[..boundary]
            .iter()
            .filter(|m| !Self::is_skill_protected(m))
            .cloned()
            .collect();

        if messages_to_compact.is_empty() {
            return CompactionAction::NoAction;
        }

        match config.strategy {
            CompactionStrategy::Truncate => CompactionAction::Truncated {
                removed_count: messages_to_compact.len(),
            },
            CompactionStrategy::Summarize => CompactionAction::NeedsSummarize {
                messages_to_summarize: messages_to_compact,
                insert_position: 0,
            },
            CompactionStrategy::MoveToWorkspace => CompactionAction::NeedsWorkspaceSave {
                messages_to_save: messages_to_compact,
                insert_position: 0,
            },
        }
    }

    /// Apply degradation to messages in-place.
    /// Returns the number of content blocks modified.
    ///
    /// 4+1 阶段：
    ///
    /// 1. None — 无操作
    /// 2. SoftTrim — 对 2 轮前的工具结果做头/尾裁剪
    /// 3. AutoCompaction — 保留最近 10 条消息，其余工具结果替换为占位符
    /// 4. OverflowCompaction — 保留最近 4 条消息（drain 旧消息）
    /// 5. ToolResultTruncation — 截断最后一条工具结果至 8000 chars
    /// 6. FinalError — 不修改（由调用方处理为错误）
    #[allow(clippy::ptr_arg)]
    pub fn apply(&self, messages: &mut Vec<ChatMessage>, level: DegradationLevel) -> usize {
        match level {
            DegradationLevel::None => 0,
            DegradationLevel::SoftTrim => self.soft_trim(messages),
            DegradationLevel::AutoCompaction => self.auto_compaction(messages),
            DegradationLevel::OverflowCompaction => self.overflow_compaction(messages),
            DegradationLevel::ToolResultTruncation => self.tool_result_truncation(messages),
            DegradationLevel::FinalError => {
                // 不修改消息，由调用方返回结构化错误
                0
            }
        }
    }

    /// 阶段 1: Soft-trim —— 对 2 轮前的工具结果做头尾裁剪（保留 head + tail）
    #[allow(clippy::ptr_arg)]
    fn soft_trim(&self, messages: &mut Vec<ChatMessage>) -> usize {
        let boundary = self.find_protection_boundary(messages);
        let mut modified = 0;

        for msg in messages[..boundary].iter_mut() {
            if Self::is_skill_protected(msg) {
                continue;
            }
            for block in msg.content.iter_mut() {
                if let ContentBlock::ToolResult { content, .. } = block {
                    if content.len() > (SOFT_TRIM_HEAD + SOFT_TRIM_TAIL + 100) {
                        let (head, tail) =
                            Self::head_tail_utf8(content, SOFT_TRIM_HEAD, SOFT_TRIM_TAIL);
                        let omitted = content.len() - SOFT_TRIM_HEAD - SOFT_TRIM_TAIL;
                        *content = format!(
                            "{}\n\n[... omitted {} chars ...]\n\n{}",
                            head, omitted, tail
                        );
                        modified += 1;
                    }
                }
            }
        }

        if modified > 0 {
            debug!(modified, "SoftTrim: trimmed tool results head/tail");
        }
        modified
    }

    /// 阶段 2: AutoCompaction —— 保留最近 10 条消息，其余工具结果替换为占位符
    #[allow(clippy::ptr_arg)]
    fn auto_compaction(&self, messages: &mut Vec<ChatMessage>) -> usize {
        let keep = AUTO_COMPACTION_KEEP;
        let boundary = if messages.len() > keep {
            messages.len() - keep
        } else {
            0
        };

        let mut modified = 0;

        for msg in messages[..boundary].iter_mut() {
            if Self::is_skill_protected(msg) {
                continue;
            }
            for block in msg.content.iter_mut() {
                if let ContentBlock::ToolResult {
                    content,
                    tool_use_id,
                    ..
                } = block
                {
                    if content.len() > 100 {
                        *content = format!(
                            "[Tool result omitted (AutoCompaction), tool_use_id={}]",
                            tool_use_id
                        );
                        modified += 1;
                    }
                }
            }
        }

        if modified > 0 {
            info!(
                modified,
                boundary, "AutoCompaction: replaced old tool results with placeholders"
            );
        }
        modified
    }

    /// 阶段 3: OverflowCompaction —— 保留最近 4 条消息，drain 旧消息
    /// Skill-protected messages are never drained.
    fn overflow_compaction(&self, messages: &mut Vec<ChatMessage>) -> usize {
        let keep = OVERFLOW_COMPACTION_KEEP;
        if messages.len() <= keep {
            return 0;
        }

        let drain_end = messages.len() - keep;
        // Collect indices of skill-protected messages in the drain range
        let protected_msgs: Vec<ChatMessage> = messages[..drain_end]
            .iter()
            .filter(|msg| Self::is_skill_protected(msg))
            .cloned()
            .collect();

        let drain_count = drain_end - protected_msgs.len();
        // Remove non-protected messages from the drain range
        messages.drain(..drain_end);
        // Re-insert protected messages at the beginning
        for (i, msg) in protected_msgs.into_iter().enumerate() {
            messages.insert(i, msg);
        }

        warn!(
            drain_count,
            "OverflowCompaction: drained old messages, kept last {}", keep
        );
        drain_count
    }

    /// 阶段 +1: ToolResultTruncation —— 截断最后一条工具结果至 8000 chars
    #[allow(clippy::ptr_arg)]
    fn tool_result_truncation(&self, messages: &mut Vec<ChatMessage>) -> usize {
        // 从末尾向前找最后一条包含 ToolResult 的消息（跳过 skill-protected）
        for msg in messages.iter_mut().rev() {
            if Self::is_skill_protected(msg) {
                continue;
            }
            for block in msg.content.iter_mut() {
                if let ContentBlock::ToolResult { content, .. } = block {
                    if content.len() > TOOL_RESULT_TRUNCATION_CHARS {
                        let truncated = Self::truncate_utf8(content, TOOL_RESULT_TRUNCATION_CHARS);
                        let omitted = content.len() - TOOL_RESULT_TRUNCATION_CHARS;
                        *content = format!(
                            "{}\n\n[... truncated {} chars (ToolResultTruncation) ...]",
                            truncated, omitted
                        );
                        warn!(
                            original_len = content.len(),
                            "ToolResultTruncation: truncated last tool result to {} chars",
                            TOOL_RESULT_TRUNCATION_CHARS
                        );
                        return 1;
                    }
                }
            }
        }
        0
    }

    /// Check if a message contains the skill-protected marker.
    /// Messages with this marker must never be pruned or compacted.
    fn is_skill_protected(msg: &ChatMessage) -> bool {
        msg.content.iter().any(|block| match block {
            ContentBlock::Text { text } => text.contains(SKILL_PROTECTED_MARKER),
            ContentBlock::ToolResult { content, .. } => content.contains(SKILL_PROTECTED_MARKER),
            ContentBlock::ToolUse { input, .. } => {
                input.to_string().contains(SKILL_PROTECTED_MARKER)
            }
            ContentBlock::Image { .. } | ContentBlock::Document { .. } => false,
        })
    }

    /// Find the message index before which we can prune.
    /// Protects the last N "rounds" (user+assistant pairs).
    fn find_protection_boundary(&self, messages: &[ChatMessage]) -> usize {
        if messages.is_empty() {
            return 0;
        }

        let mut rounds_found = 0;
        let mut boundary = messages.len();

        for (i, msg) in messages.iter().enumerate().rev() {
            if msg.role == MessageRole::User {
                let is_tool_result_msg = msg
                    .content
                    .iter()
                    .all(|b| matches!(b, ContentBlock::ToolResult { .. }));
                if !is_tool_result_msg {
                    rounds_found += 1;
                    if rounds_found > self.protect_recent_rounds {
                        boundary = i;
                        break;
                    }
                }
            }
        }

        if rounds_found <= self.protect_recent_rounds {
            return 0;
        }

        boundary
    }

    /// Find safe compaction boundary (not in the middle of a tool chain).
    /// Returns the index at which to split: messages[..index] will be summarized.
    pub fn find_compaction_boundary(messages: &[ChatMessage], min_keep_chars: usize) -> usize {
        if messages.is_empty() {
            return 0;
        }

        let mut kept_chars: usize = 0;
        let mut candidate_boundary = 0;

        for (i, msg) in messages.iter().enumerate().rev() {
            let msg_chars: usize = msg
                .content
                .iter()
                .map(|b| match b {
                    ContentBlock::Text { text } => text.len(),
                    ContentBlock::ToolUse { input, .. } => input.to_string().len(),
                    ContentBlock::ToolResult { content, .. } => content.len(),
                    ContentBlock::Image { data, .. } => data.len(),
                    ContentBlock::Document { data, .. } => data.len(),
                })
                .sum();

            kept_chars += msg_chars;

            if kept_chars >= min_keep_chars {
                candidate_boundary = i;
                break;
            }
        }

        // Walk forward from candidate to find a safe boundary:
        // Safe = right after an Assistant message that contains Text (not just ToolUse).
        for i in candidate_boundary..messages.len() {
            if messages[i].role == MessageRole::Assistant {
                let has_text = messages[i]
                    .content
                    .iter()
                    .any(|b| matches!(b, ContentBlock::Text { .. }));
                let has_only_tool_use = messages[i]
                    .content
                    .iter()
                    .all(|b| matches!(b, ContentBlock::ToolUse { .. }));
                if has_text && !has_only_tool_use {
                    return (i + 1).min(messages.len());
                }
            }
        }

        candidate_boundary
    }

    /// UTF-8 safe head+tail extraction.
    fn head_tail_utf8(s: &str, head_chars: usize, tail_chars: usize) -> (String, String) {
        let head_end = s
            .char_indices()
            .nth(head_chars)
            .map(|(idx, _)| idx)
            .unwrap_or(s.len());

        let char_count = s.chars().count();
        let tail_start_char = char_count.saturating_sub(tail_chars);
        let tail_start = s
            .char_indices()
            .nth(tail_start_char)
            .map(|(idx, _)| idx)
            .unwrap_or(s.len());

        (s[..head_end].to_string(), s[tail_start..].to_string())
    }

    /// UTF-8 safe truncation to max_chars.
    fn truncate_utf8(s: &str, max_chars: usize) -> String {
        let end = s
            .char_indices()
            .nth(max_chars)
            .map(|(idx, _)| idx)
            .unwrap_or(s.len());
        s[..end].to_string()
    }
}

impl Default for ContextPruner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod compaction_tests {
    use super::*;

    fn make_messages(count: usize) -> Vec<ChatMessage> {
        (0..count)
            .map(|i| {
                if i % 2 == 0 {
                    ChatMessage::user(format!("User message {}", i))
                } else {
                    ChatMessage::assistant(format!("Assistant message {}", i))
                }
            })
            .collect()
    }

    #[test]
    fn test_no_config_returns_no_action() {
        let pruner = ContextPruner::new();
        let msgs = make_messages(20);
        assert_eq!(pruner.plan_compaction(&msgs), CompactionAction::NoAction);
    }

    #[test]
    fn test_few_messages_returns_no_action() {
        let pruner = ContextPruner::new()
            .with_compaction_config(CompactionConfig::default());
        let msgs = make_messages(4);
        assert_eq!(pruner.plan_compaction(&msgs), CompactionAction::NoAction);
    }

    #[test]
    fn test_truncate_strategy() {
        let pruner = ContextPruner::new()
            .with_compaction_config(CompactionConfig {
                strategy: CompactionStrategy::Truncate,
                ..Default::default()
            });
        let msgs = make_messages(10);
        match pruner.plan_compaction(&msgs) {
            CompactionAction::Truncated { removed_count } => {
                assert!(removed_count > 0);
            }
            other => panic!("Expected Truncated, got {:?}", other),
        }
    }

    #[test]
    fn test_summarize_strategy() {
        let pruner = ContextPruner::new()
            .with_compaction_config(CompactionConfig {
                strategy: CompactionStrategy::Summarize,
                summary_max_tokens: 300,
            });
        let msgs = make_messages(10);
        match pruner.plan_compaction(&msgs) {
            CompactionAction::NeedsSummarize { messages_to_summarize, insert_position } => {
                assert!(!messages_to_summarize.is_empty());
                assert_eq!(insert_position, 0);
            }
            other => panic!("Expected NeedsSummarize, got {:?}", other),
        }
    }

    #[test]
    fn test_move_to_workspace_strategy() {
        let pruner = ContextPruner::new()
            .with_compaction_config(CompactionConfig {
                strategy: CompactionStrategy::MoveToWorkspace,
                ..Default::default()
            });
        let msgs = make_messages(10);
        match pruner.plan_compaction(&msgs) {
            CompactionAction::NeedsWorkspaceSave { messages_to_save, insert_position } => {
                assert!(!messages_to_save.is_empty());
                assert_eq!(insert_position, 0);
            }
            other => panic!("Expected NeedsWorkspaceSave, got {:?}", other),
        }
    }

    #[test]
    fn test_keeps_recent_messages() {
        let pruner = ContextPruner::new()
            .with_compaction_config(CompactionConfig {
                strategy: CompactionStrategy::Summarize,
                ..Default::default()
            });
        let msgs = make_messages(10);
        if let CompactionAction::NeedsSummarize { messages_to_summarize, .. } = pruner.plan_compaction(&msgs) {
            // 10 messages, keep 50% = 5, compact 5
            assert_eq!(messages_to_summarize.len(), 5);
        } else {
            panic!("Expected NeedsSummarize");
        }
    }

    #[test]
    fn test_with_compaction_config_sets_config() {
        let config = CompactionConfig {
            strategy: CompactionStrategy::Summarize,
            summary_max_tokens: 1000,
        };
        let pruner = ContextPruner::new().with_compaction_config(config);
        assert!(pruner.compaction_config.is_some());
        assert_eq!(
            pruner.compaction_config.unwrap().strategy,
            CompactionStrategy::Summarize
        );
    }

    #[test]
    fn test_skill_protected_messages_excluded() {
        let pruner = ContextPruner::new()
            .with_compaction_config(CompactionConfig {
                strategy: CompactionStrategy::Summarize,
                ..Default::default()
            });
        let mut msgs = make_messages(10);
        // Mark first message as skill-protected
        msgs[0] = ChatMessage::user(format!("{} important skill", SKILL_PROTECTED_MARKER));

        if let CompactionAction::NeedsSummarize { messages_to_summarize, .. } = pruner.plan_compaction(&msgs) {
            // Protected message should be excluded
            assert!(messages_to_summarize.iter().all(|m| {
                !m.content.iter().any(|b| match b {
                    ContentBlock::Text { text } => text.contains(SKILL_PROTECTED_MARKER),
                    _ => false,
                })
            }));
        }
    }

    #[test]
    fn test_default_compaction_config() {
        let config = CompactionConfig::default();
        assert_eq!(config.strategy, CompactionStrategy::Truncate);
        assert_eq!(config.summary_max_tokens, 500);
    }
}
