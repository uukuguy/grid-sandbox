//! Unified ContextManager — facade over SystemPromptBuilder, ContextBudgetManager, and ContextPruner.
//!
//! Provides a single entry point for context engineering: token counting,
//! budget snapshots, and pruning decisions.

use octo_types::message::ChatMessage;

// ---------------------------------------------------------------------------
// TokenCounter trait
// ---------------------------------------------------------------------------

/// Trait for counting tokens in text and message sequences.
pub trait TokenCounter: Send + Sync {
    /// Count estimated tokens for a plain text string.
    fn count(&self, text: &str) -> usize;

    /// Count estimated tokens for a slice of `ChatMessage`s.
    ///
    /// Implementations should account for per-message framing overhead.
    fn count_messages(&self, messages: &[ChatMessage]) -> usize;
}

// ---------------------------------------------------------------------------
// EstimateCounter
// ---------------------------------------------------------------------------

/// A lightweight, dependency-free token estimator.
///
/// Heuristic: ASCII characters are counted as `chars / 4`, CJK / non-ASCII
/// characters as `chars / 1.5`.  Each message carries an additional 4-token
/// overhead (role tag, delimiters, etc.).
#[derive(Debug, Clone, Default)]
pub struct EstimateCounter;

impl TokenCounter for EstimateCounter {
    fn count(&self, text: &str) -> usize {
        let mut total: f64 = 0.0;
        for ch in text.chars() {
            if ch.is_ascii() {
                total += 0.25;
            } else {
                total += 0.67; // ~1/1.5
            }
        }
        total.ceil() as usize
    }

    fn count_messages(&self, messages: &[ChatMessage]) -> usize {
        messages
            .iter()
            .map(|m| self.count(&m.text_content()) + 4)
            .sum()
    }
}

// ---------------------------------------------------------------------------
// ContextBudgetSnapshot
// ---------------------------------------------------------------------------

/// A point-in-time snapshot of how the context token budget is allocated.
#[derive(Debug, Clone)]
pub struct ContextBudgetSnapshot {
    /// Total token budget for the model context window.
    pub total_budget: usize,
    /// Tokens consumed by the system prompt.
    pub system_tokens: usize,
    /// Tokens consumed by conversation messages.
    pub message_tokens: usize,
    /// Tokens reserved for tool definitions (fixed reserve ratio).
    pub tool_tokens: usize,
    /// Remaining tokens available for new content.
    pub remaining: usize,
    /// Budget utilisation as a fraction in `[0.0, 1.0]`.
    pub usage_pct: f32,
}

// ---------------------------------------------------------------------------
// ContextManager
// ---------------------------------------------------------------------------

/// Unified context manager that combines token counting, budget tracking,
/// and pruning decisions behind a single API.
pub struct ContextManager {
    counter: Box<dyn TokenCounter>,
    max_context_tokens: usize,
    /// Fraction of the total budget reserved for the system prompt (default 0.15).
    system_reserve_pct: f32,
}

impl ContextManager {
    /// Create a new `ContextManager`.
    ///
    /// * `counter` – token counting implementation
    /// * `max_context_tokens` – model context window size in tokens
    pub fn new(counter: Box<dyn TokenCounter>, max_context_tokens: usize) -> Self {
        Self {
            counter,
            max_context_tokens,
            system_reserve_pct: 0.15,
        }
    }

    /// Override the default system-prompt reserve percentage.
    pub fn with_system_reserve_pct(mut self, pct: f32) -> Self {
        self.system_reserve_pct = pct;
        self
    }

    /// Produce a snapshot describing current budget allocation.
    pub fn budget_snapshot(
        &self,
        system_prompt: &str,
        messages: &[ChatMessage],
    ) -> ContextBudgetSnapshot {
        let system_tokens = self.counter.count(system_prompt);
        let message_tokens = self.counter.count_messages(messages);
        let tool_tokens =
            (self.max_context_tokens as f32 * self.system_reserve_pct).ceil() as usize;

        let used = system_tokens + message_tokens + tool_tokens;
        let remaining = self.max_context_tokens.saturating_sub(used);
        let usage_pct = if self.max_context_tokens > 0 {
            used as f32 / self.max_context_tokens as f32
        } else {
            0.0
        };

        ContextBudgetSnapshot {
            total_budget: self.max_context_tokens,
            system_tokens,
            message_tokens,
            tool_tokens,
            remaining,
            usage_pct,
        }
    }

    /// Returns `true` when the budget utilisation exceeds 85 %, indicating
    /// that older messages should be pruned.
    pub fn needs_pruning(&self, snapshot: &ContextBudgetSnapshot) -> bool {
        snapshot.usage_pct > 0.85
    }

    /// Convenience accessor: tokens still available for new content.
    pub fn available_tokens(&self, snapshot: &ContextBudgetSnapshot) -> usize {
        snapshot.remaining
    }
}
