//! LLM-based conversation compaction pipeline (AP-T6).
//!
//! When the context window fills up and a prompt-too-long error occurs,
//! this pipeline summarizes older messages using an LLM call, then rebuilds
//! essential state (memory zones, active skill, hooks) so the conversation
//! can continue without losing critical context.

use std::sync::Arc;

use anyhow::{anyhow, Result};
use grid_types::skill::SkillDefinition;
use grid_types::{ChatMessage, CompletionRequest, ContentBlock, MessageRole, SandboxId, SessionId, UserId};
use tracing::{debug, info, warn};

use crate::hooks::{HookContext, HookPoint, HookRegistry};
use crate::memory::store_traits::MemoryStore;
use crate::memory::{MemoryInjector, WorkingMemory};
use crate::providers::Provider;

use super::budget::ContextBudgetManager;
use super::compact_prompt;
use crate::agent::harness::is_prompt_too_long;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for the compaction pipeline.
///
/// New fields per ADR-V2-018 (S3.T1):
/// - `proactive_threshold_pct`: usage % at which a proactive compact may run.
/// - `tail_protect_tokens`: token budget (chars/4 estimate) preserved verbatim
///   at the end of `messages`. Replaces hard `keep_recent_messages` as the
///   primary tail rule; the count-based field is kept as a fallback when the
///   token-walk cannot find a sensible boundary.
/// - `summary_ratio`: relative aggressiveness of the summary; proactive uses
///   the configured value (default 0.2), reactive paths pass `0.5` via the
///   `CompactionTrigger::Reactive` override.
/// - `summary_min_tokens`: floor for `max_tokens` of the summarizer call.
/// - `reactive_only`: when `true`, harness skips the proactive threshold
///   check and only runs compaction in response to errors.
#[derive(Debug, Clone)]
pub struct CompactionPipelineConfig {
    /// Model to use for the summary LLM call. `None` reuses the session model.
    pub compact_model: Option<String>,
    /// Maximum output tokens for the summary response.
    pub summary_max_tokens: u32,
    /// Number of most-recent messages to keep uncompacted (legacy fallback).
    pub keep_recent_messages: usize,
    /// Maximum PTL retries when the summary call itself overflows.
    pub max_ptl_retries: u32,
    /// Usage percent (0-100) above which proactive compaction may run.
    pub proactive_threshold_pct: u8,
    /// Token budget (chars/4 estimate) preserved verbatim at the end.
    pub tail_protect_tokens: u64,
    /// Default compaction ratio for proactive triggers.
    pub summary_ratio: f32,
    /// Compaction ratio for reactive triggers (413/context overflow). More
    /// conservative than `summary_ratio` so post-compaction context still holds
    /// the recent narrative. Reviewer M2 fix — no more magic constant.
    pub reactive_summary_ratio: f32,
    /// Floor for the summarizer `max_tokens`.
    pub summary_min_tokens: u32,
    /// Skip proactive checks when `true`; only compact on PTL/overflow.
    pub reactive_only: bool,
}

impl Default for CompactionPipelineConfig {
    fn default() -> Self {
        Self {
            compact_model: None,
            summary_max_tokens: 2000,
            keep_recent_messages: 6,
            max_ptl_retries: 3,
            proactive_threshold_pct: 75,
            tail_protect_tokens: 20_000,
            summary_ratio: 0.2,
            reactive_summary_ratio: 0.5,
            summary_min_tokens: 2_000,
            reactive_only: false,
        }
    }
}

/// Why a `compact()` invocation was started. Influences the summary ratio,
/// the PreCompact hook payload's `trigger` field, and downstream telemetry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactionTrigger {
    /// Threshold-based preemptive compaction (default ratio 0.2).
    Proactive,
    /// Triggered by a 413 / context-window error (default ratio 0.5).
    Reactive,
}

impl CompactionTrigger {
    /// Wire-format string used in the PreCompact hook payload.
    pub fn as_str(&self) -> &'static str {
        match self {
            CompactionTrigger::Proactive => "proactive_threshold",
            CompactionTrigger::Reactive => "reactive_413",
        }
    }
}

// ---------------------------------------------------------------------------
// Result
// ---------------------------------------------------------------------------

/// Output of a successful compaction.
#[derive(Debug)]
pub struct CompactionResult {
    /// Boundary marker indicating where compaction occurred.
    pub boundary_marker: ChatMessage,
    /// LLM-generated summary of the compacted portion.
    pub summary_messages: Vec<ChatMessage>,
    /// Recent messages kept verbatim (not compacted).
    pub kept_messages: Vec<ChatMessage>,
    /// Re-injected state messages (Zone B, skill context).
    pub reinjections: Vec<ChatMessage>,
    /// Text to append to system prompt (cross-session memory, pinned memories).
    /// Kept in system prompt so LLM treats them as background context
    /// and does not repeat them in tool results or responses.
    pub system_prompt_additions: String,
    /// Estimated token count before compaction.
    pub pre_compact_tokens: usize,
    /// Estimated token count after compaction.
    pub post_compact_tokens: usize,
}

// ---------------------------------------------------------------------------
// Context for state rebuild
// ---------------------------------------------------------------------------

/// Everything needed to rebuild agent state after compaction.
pub struct CompactionContext {
    pub memory: Option<Arc<dyn WorkingMemory>>,
    pub memory_store: Option<Arc<dyn MemoryStore>>,
    pub active_skill: Option<SkillDefinition>,
    pub hook_registry: Option<Arc<HookRegistry>>,
    pub session_summary_store: Option<Arc<crate::memory::SessionSummaryStore>>,
    pub user_id: UserId,
    pub sandbox_id: SandboxId,
    /// Session ID — required by T1.E iterative summary reuse to scope
    /// `SessionSummaryStore::get_latest()` to the current conversation.
    pub session_id: SessionId,
    /// Custom instructions from the system prompt (used to guide summarization).
    pub custom_instructions: Option<String>,
    /// Actual context window of the session's provider model, in tokens.
    /// Threaded from `ContextBudgetManager::context_window()` so the PreCompact
    /// hook payload reports correct `usage_pct` regardless of provider.
    /// Defaults to 200_000 when unknown (claude-3.5 family default).
    pub context_window: u64,
}

// ---------------------------------------------------------------------------
// Pipeline
// ---------------------------------------------------------------------------

/// LLM-based compaction pipeline.
///
/// Replaces older conversation messages with a concise LLM-generated summary,
/// then re-injects essential state (working memory, cross-session memory,
/// active skill, hooks) so the agent can continue seamlessly.
#[derive(Debug, Clone)]
pub struct CompactionPipeline {
    config: CompactionPipelineConfig,
}

impl CompactionPipeline {
    pub fn new(config: CompactionPipelineConfig) -> Self {
        Self { config }
    }

    /// Borrow the pipeline's effective config — used by the harness to read
    /// the proactive trigger threshold and `reactive_only` flag without
    /// duplicating storage in `AgentLoopConfig`.
    pub fn config(&self) -> &CompactionPipelineConfig {
        &self.config
    }

    /// Backward-compatible entry — defaults to `CompactionTrigger::Reactive`
    /// (preserves the historic behavior of being invoked from the 413 handler).
    pub async fn compact(
        &self,
        messages: &[ChatMessage],
        provider: &dyn Provider,
        model: &str,
        context: &CompactionContext,
    ) -> Result<CompactionResult> {
        self.compact_with_trigger(
            messages,
            provider,
            model,
            context,
            CompactionTrigger::Reactive,
        )
        .await
    }

    /// Run the full compaction pipeline with an explicit trigger.
    ///
    /// Token-based head/tail protection (T1.B):
    ///   * Head = system message + first user/asst pair (max 3 entries).
    ///   * Tail = trailing window holding `tail_protect_tokens` (chars/4 estimate).
    ///   * Middle = everything else, fed to the summarizer.
    ///
    /// Iterative summary reuse (T1.E):
    ///   * If a `SessionSummaryStore` is wired, the most recent prior summary
    ///     for `context.session_id` is fetched and prepended as a system
    ///     message before summarization, then the resulting new summary is
    ///     persisted (upsert by session_id).
    ///
    /// PreCompact hook (T1.D):
    ///   * Fired BEFORE the summarizer LLM call. Audit-only — the returned
    ///     `HookAction` is ignored. Payload travels through
    ///     `HookContext::metadata` carrying the 8 fields from
    ///     `PreCompactHook` (ADR-V2-018 §D1).
    pub async fn compact_with_trigger(
        &self,
        messages: &[ChatMessage],
        provider: &dyn Provider,
        model: &str,
        context: &CompactionContext,
        trigger: CompactionTrigger,
    ) -> Result<CompactionResult> {
        if messages.len() < 2 {
            return Err(anyhow!(
                "Not enough messages to compact ({} total)",
                messages.len()
            ));
        }

        // T1.B: token-based head + tail protection.
        let head_end = find_head_boundary(messages);
        let tail_start = find_tail_boundary(messages, self.config.tail_protect_tokens);

        // Decide split. Falls back to the legacy count-based split when the
        // token walk produces an empty middle (head and tail collide) or
        // crossed bounds.
        let (head_slice, middle_slice, tail_slice) =
            if head_end < tail_start && tail_start <= messages.len() {
                (
                    &messages[..head_end],
                    &messages[head_end..tail_start],
                    &messages[tail_start..],
                )
            } else {
                let keep_count = self.config.keep_recent_messages.min(messages.len());
                let boundary = messages.len().saturating_sub(keep_count);
                if boundary < 2 {
                    return Err(anyhow!(
                        "Not enough messages to compact ({} total, head_end={}, tail_start={})",
                        messages.len(),
                        head_end,
                        tail_start
                    ));
                }
                (
                    &messages[..0],
                    &messages[..boundary],
                    &messages[boundary..],
                )
            };

        if middle_slice.is_empty() {
            return Err(anyhow!(
                "Nothing to summarize — middle region is empty after head/tail protection"
            ));
        }

        info!(
            total = messages.len(),
            head = head_slice.len(),
            middle = middle_slice.len(),
            tail = tail_slice.len(),
            ?trigger,
            "Starting LLM compaction"
        );

        let pre_tokens = ContextBudgetManager::estimate_messages_tokens(messages) as usize;

        // T1.E: fetch latest prior summary (if any) for this session.
        let prior_summary = match context.session_summary_store {
            Some(ref store) => store.get_latest(context.session_id.as_str()).await.ok().flatten(),
            None => None,
        };
        let reuses_prior_summary = prior_summary.is_some();

        // 1. Preprocess the middle
        let mut preprocessed = Self::preprocess_for_summary(middle_slice);

        // T1.E: prepend the prior summary as a leading system-style hint so
        // the summarizer can build incrementally rather than from scratch.
        if let Some(ref prior) = prior_summary {
            preprocessed.insert(
                0,
                ChatMessage {
                    role: MessageRole::System,
                    content: vec![ContentBlock::Text {
                        text: format!(
                            "[Prior summary from previous compaction]\n{}",
                            prior.summary
                        ),
                    }],
                },
            );
        }

        // 2. Build prompt
        let prompt = match context.custom_instructions.as_deref() {
            Some(instr) => compact_prompt::with_custom_instructions(instr),
            None => compact_prompt::COMPACT_PROMPT.to_string(),
        };

        // T1.D: fire PreCompact hook BEFORE the summarizer LLM call.
        // Audit-only per ADR-V2-018 §D1 — the action is ignored.
        if let Some(ref hooks) = context.hook_registry {
            // Reviewer M1 fix: use session's real context_window (threaded via
            // CompactionContext), not a hardcoded 200K default that would misreport
            // usage_pct for non-Claude sessions to L3/L4 audit consumers.
            let context_window = context.context_window;
            let usage_pct = if context_window == 0 {
                0u32
            } else {
                ((pre_tokens as f64 / context_window as f64) * 100.0)
                    .round()
                    .clamp(0.0, 100.0) as u32
            };
            let prior_count: u32 = if reuses_prior_summary { 1 } else { 0 };
            let mut ctx = HookContext::new()
                .with_session(context.session_id.as_str().to_string());
            ctx.set_metadata("trigger", serde_json::json!(trigger.as_str()));
            ctx.set_metadata("estimated_tokens", serde_json::json!(pre_tokens as u64));
            ctx.set_metadata("context_window", serde_json::json!(context_window));
            ctx.set_metadata("usage_pct", serde_json::json!(usage_pct));
            ctx.set_metadata(
                "messages_to_compact",
                serde_json::json!(middle_slice.len() as u32),
            );
            ctx.set_metadata("messages_total", serde_json::json!(messages.len() as u32));
            ctx.set_metadata(
                "reuses_prior_summary",
                serde_json::json!(reuses_prior_summary),
            );
            ctx.set_metadata("prior_summary_count", serde_json::json!(prior_count));
            let _ = hooks.execute(HookPoint::PreCompact, &ctx).await;
            debug!(?trigger, "Fired PreCompact hook");
        }

        // 3. Determine effective summary max_tokens. Reactive triggers want a
        // larger summary (ratio 0.5) so post-compaction context still has the
        // recent narrative; proactive triggers want aggressive shrinkage
        // (ratio per config). Floor by `summary_min_tokens`, ceil by
        // `summary_max_tokens` from the config.
        let ratio = match trigger {
            CompactionTrigger::Proactive => self.config.summary_ratio,
            CompactionTrigger::Reactive => self.config.reactive_summary_ratio,
        };
        let target_summary_tokens = ((pre_tokens as f32 * ratio) as u32)
            .max(self.config.summary_min_tokens)
            .min(self.config.summary_max_tokens);

        // 4. Generate summary via LLM (with PTL self-retry)
        let compact_model = self.config.compact_model.as_deref().unwrap_or(model);
        let summary_text = self
            .generate_summary_with_max(provider, compact_model, &preprocessed, &prompt, target_summary_tokens)
            .await?;

        // 5. Format
        let formatted = Self::format_summary(&summary_text);

        // T1.E: persist the new summary (upsert per session_id).
        if let Some(ref store) = context.session_summary_store {
            if let Err(e) = store
                .save(
                    context.session_id.as_str(),
                    &formatted,
                    middle_slice.len(),
                    &[],
                    0,
                )
                .await
            {
                warn!(error = %e, "Failed to persist new compaction summary");
            }
        }

        // 6. Rebuild state
        let (reinjections, sys_additions) = Self::rebuild_state(context).await;

        // 7. Assemble result. Head is preserved verbatim (prepended), then
        // boundary marker, then summary, then tail kept verbatim, then state
        // reinjections.
        let boundary_marker = ChatMessage {
            role: MessageRole::User,
            content: vec![ContentBlock::Text {
                text: "[Context compacted: earlier conversation summarized below]".into(),
            }],
        };

        let summary_msg = ChatMessage::assistant(&formatted);
        // T1.B: keep head verbatim by inlining it ahead of the boundary
        // marker via the existing CompactionResult shape. Callers that
        // rebuild messages already concatenate marker + summary + kept +
        // reinjections; placing head into kept_messages preserves that
        // ordering without changing the result struct.
        let mut kept = Vec::with_capacity(head_slice.len() + tail_slice.len());
        kept.extend_from_slice(head_slice);
        kept.extend_from_slice(tail_slice);

        let post_messages: Vec<ChatMessage> = std::iter::once(&boundary_marker)
            .chain(std::iter::once(&summary_msg))
            .chain(kept.iter())
            .chain(reinjections.iter())
            .cloned()
            .collect();
        let post_tokens = ContextBudgetManager::estimate_messages_tokens(&post_messages) as usize;

        info!(
            pre_tokens,
            post_tokens,
            saved = pre_tokens.saturating_sub(post_tokens),
            ?trigger,
            reuses_prior_summary,
            "Compaction complete"
        );

        Ok(CompactionResult {
            boundary_marker,
            summary_messages: vec![summary_msg],
            kept_messages: kept,
            reinjections,
            system_prompt_additions: sys_additions,
            pre_compact_tokens: pre_tokens,
            post_compact_tokens: post_tokens,
        })
    }

    // -----------------------------------------------------------------------
    // Preprocessing
    // -----------------------------------------------------------------------

    /// Replace images with placeholders and truncate oversized tool results
    /// to reduce the token cost of the summary LLM call.
    fn preprocess_for_summary(messages: &[ChatMessage]) -> Vec<ChatMessage> {
        messages
            .iter()
            .map(|m| {
                let content = m
                    .content
                    .iter()
                    .map(|block| match block {
                        // Images → lightweight placeholder
                        ContentBlock::Image { .. } | ContentBlock::Document { .. } => {
                            ContentBlock::Text {
                                text: "[image]".into(),
                            }
                        }
                        // Truncate long tool results
                        ContentBlock::ToolResult {
                            tool_use_id,
                            content,
                            is_error,
                        } if content.len() > 2000 => {
                            let truncated = if content.is_char_boundary(2000) {
                                &content[..2000]
                            } else {
                                // Find the last valid char boundary before 2000
                                let end = content
                                    .char_indices()
                                    .take_while(|(i, _)| *i < 2000)
                                    .last()
                                    .map(|(i, c)| i + c.len_utf8())
                                    .unwrap_or(0);
                                &content[..end]
                            };
                            ContentBlock::ToolResult {
                                tool_use_id: tool_use_id.clone(),
                                content: format!(
                                    "{}... [truncated, {} chars total]",
                                    truncated,
                                    content.len()
                                ),
                                is_error: *is_error,
                            }
                        }
                        other => other.clone(),
                    })
                    .collect();
                ChatMessage {
                    role: m.role.clone(),
                    content,
                }
            })
            .collect()
    }

    // -----------------------------------------------------------------------
    // Summary generation with PTL self-retry
    // -----------------------------------------------------------------------

    async fn generate_summary(
        &self,
        provider: &dyn Provider,
        model: &str,
        messages: &[ChatMessage],
        prompt: &str,
    ) -> Result<String> {
        self.generate_summary_with_max(
            provider,
            model,
            messages,
            prompt,
            self.config.summary_max_tokens,
        )
        .await
    }

    async fn generate_summary_with_max(
        &self,
        provider: &dyn Provider,
        model: &str,
        messages: &[ChatMessage],
        prompt: &str,
        summary_max_tokens: u32,
    ) -> Result<String> {
        let mut to_summarize = messages.to_vec();

        for attempt in 0..self.config.max_ptl_retries {
            let request = CompletionRequest {
                model: model.to_string(),
                system: Some(prompt.to_string()),
                messages: to_summarize.clone(),
                max_tokens: summary_max_tokens,
                tools: vec![],
                stream: false,
                temperature: None,
                tool_choice: None,
            };

            match provider.complete(request).await {
                Ok(response) => {
                    let text: String = response
                        .content
                        .iter()
                        .filter_map(|b| match b {
                            ContentBlock::Text { text } => Some(text.as_str()),
                            _ => None,
                        })
                        .collect();
                    if text.is_empty() {
                        return Err(anyhow!("LLM returned empty summary"));
                    }
                    return Ok(text);
                }
                Err(e) if is_prompt_too_long(&e) => {
                    let drop_count = (to_summarize.len() / 5).max(1);
                    warn!(
                        attempt,
                        drop_count,
                        remaining = to_summarize.len() - drop_count,
                        "Summary LLM hit PTL, dropping oldest messages"
                    );
                    to_summarize = to_summarize[drop_count..].to_vec();
                    if to_summarize.len() < 2 {
                        return Err(anyhow!("Not enough messages left after PTL retry"));
                    }
                }
                Err(e) => return Err(e),
            }
        }

        Err(anyhow!(
            "Compact summary failed after {} PTL retries",
            self.config.max_ptl_retries
        ))
    }

    // -----------------------------------------------------------------------
    // Summary formatting
    // -----------------------------------------------------------------------

    /// Strip the `<analysis>` scratchpad and extract `<summary>` content.
    pub fn format_summary(raw: &str) -> String {
        let mut result = raw.to_string();

        // Strip <analysis>...</analysis> block
        if let (Some(start), Some(end)) = (result.find("<analysis>"), result.find("</analysis>")) {
            if end > start {
                result = format!(
                    "{}{}",
                    &result[..start],
                    &result[end + "</analysis>".len()..]
                );
            }
        }

        // Extract <summary>...</summary> content
        if let (Some(start), Some(end)) = (result.find("<summary>"), result.find("</summary>")) {
            if end > start {
                let inner = &result[start + "<summary>".len()..end];
                result = inner.trim().to_string();
            }
        }

        format!(
            "This session is being continued from a previous conversation that hit the \
             context limit. The summary below captures the key points.\n\n{}",
            result.trim()
        )
    }

    // -----------------------------------------------------------------------
    // State rebuild
    // -----------------------------------------------------------------------

    /// Re-inject Zone B (working memory), Zone B++ (session summaries),
    /// active skill context, and fire SessionStart hooks.
    /// Returns (reinjection_messages, system_prompt_additions).
    async fn rebuild_state(ctx: &CompactionContext) -> (Vec<ChatMessage>, String) {
        let mut reinjections = Vec::new();

        // Zone B: working memory
        if let Some(ref memory) = ctx.memory {
            if let Ok(xml) = memory.compile(&ctx.user_id, &ctx.sandbox_id).await {
                if !xml.is_empty() {
                    reinjections.push(ChatMessage {
                        role: MessageRole::User,
                        content: vec![ContentBlock::Text {
                            text: format!("<working_memory>\n{}\n</working_memory>", xml),
                        }],
                    });
                    debug!("Reinjected Zone B working memory");
                }
            }
        }

        // Zone B+: cross-session memory + pinned memories → system prompt additions
        // (NOT user messages, so LLM treats them as background context)
        let mut sys_additions = String::new();
        if let Some(ref store) = ctx.memory_store {
            let injector = MemoryInjector::with_defaults();
            let cross = injector
                .build_memory_context(store.as_ref(), ctx.user_id.as_str(), "")
                .await;
            if !cross.is_empty() {
                sys_additions.push_str(&cross);
                debug!("System prompt addition: cross-session memory {} chars", cross.len());
            }

            // Phase AS: Importance-based pinned memories (safety net for high-importance entries)
            let pinned = injector
                .build_pinned_memories(store.as_ref(), ctx.user_id.as_str(), 0.8, 5, &[])
                .await;
            if !pinned.is_empty() {
                sys_additions.push_str(&pinned);
                debug!("System prompt addition: pinned memories {} chars", pinned.len());
            }
        }

        // Zone B++: session summaries
        if let Some(ref summary_store) = ctx.session_summary_store {
            if let Ok(summaries) = summary_store.recent(5).await {
                if !summaries.is_empty() {
                    let text = summaries
                        .iter()
                        .map(|s| {
                            format!(
                                "<session_summary session_id=\"{}\">\n{}\n</session_summary>",
                                s.session_id, s.summary
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    reinjections.push(ChatMessage {
                        role: MessageRole::User,
                        content: vec![ContentBlock::Text { text }],
                    });
                    debug!(count = summaries.len(), "Reinjected Zone B++ session summaries");
                }
            }
        }

        // Active skill context
        if let Some(ref skill) = ctx.active_skill {
            if !skill.body.is_empty() {
                reinjections.push(ChatMessage {
                    role: MessageRole::User,
                    content: vec![ContentBlock::Text {
                        text: format!("[Active skill: {}]\n{}", skill.name, skill.body),
                    }],
                });
                debug!(skill = %skill.name, "Reinjected active skill context");
            }
        }

        // Fire SessionStart hooks (non-blocking, best-effort)
        if let Some(ref hooks) = ctx.hook_registry {
            let hook_ctx = HookContext::new();
            let _ = hooks.execute(HookPoint::SessionStart, &hook_ctx).await;
            debug!("Fired SessionStart hooks for state rebuild");
        }

        (reinjections, sys_additions)
    }
}

// ---------------------------------------------------------------------------
// Head/Tail boundary helpers (T1.B, ADR-V2-018 §D6)
// ---------------------------------------------------------------------------

/// Identify the index just past the protected "head" of the conversation.
///
/// Per ADR-V2-018 §D6 the head is `system messages + first user/assistant pair`.
/// Returns the index of the first message that is fair game for summarization.
fn find_head_boundary(messages: &[ChatMessage]) -> usize {
    let mut idx = 0;
    while idx < messages.len() && messages[idx].role == MessageRole::System {
        idx += 1;
    }
    if idx < messages.len() && messages[idx].role == MessageRole::User {
        idx += 1;
        if idx < messages.len() && messages[idx].role == MessageRole::Assistant {
            idx += 1;
        }
    }
    idx
}

/// Walk backwards over `messages` accumulating estimated tokens until the
/// running total reaches `tail_budget_tokens`. Returns the start index of
/// the protected tail window. Returns `messages.len()` when the budget is 0.
fn find_tail_boundary(messages: &[ChatMessage], tail_budget_tokens: u64) -> usize {
    if tail_budget_tokens == 0 || messages.is_empty() {
        return messages.len();
    }
    let mut acc: u64 = 0;
    let mut idx = messages.len();
    while idx > 0 {
        let next = idx - 1;
        let msg_tokens =
            ContextBudgetManager::estimate_messages_tokens(std::slice::from_ref(&messages[next]));
        if acc.saturating_add(msg_tokens) > tail_budget_tokens && next < messages.len() - 1 {
            // Including this message would exceed the tail budget — stop here.
            // The `next < len - 1` guard ensures we always keep at least the
            // most recent message so the conversation has something to anchor.
            break;
        }
        acc = acc.saturating_add(msg_tokens);
        idx = next;
    }
    idx
}

// ---------------------------------------------------------------------------
// Auto-Snip (AV-T3)
// ---------------------------------------------------------------------------

impl CompactionPipeline {
    /// Auto-snip: remove messages older than `boundary` index via simple truncation.
    ///
    /// Unlike `compact()`, this does NOT call an LLM for summarization — it just
    /// truncates. Intended for automatic budget pressure relief.
    ///
    /// Returns the number of messages removed, or 0 if no action was taken.
    pub fn auto_snip(
        messages: &mut Vec<ChatMessage>,
        keep_recent: usize,
    ) -> usize {
        let min_messages = 8;
        if messages.len() <= min_messages || keep_recent >= messages.len() {
            return 0;
        }
        let boundary = messages.len().saturating_sub(keep_recent);
        if boundary < 2 {
            return 0;
        }
        info!(boundary, total = messages.len(), keep_recent, "Auto-snip triggered");
        // Insert a marker at position 0 summarizing what was removed
        let summary_marker = ChatMessage {
            role: MessageRole::User,
            content: vec![ContentBlock::Text {
                text: format!(
                    "[Auto-snip: {} older messages removed to free context space]",
                    boundary
                ),
            }],
        };
        messages.drain(..boundary);
        messages.insert(0, summary_marker);
        info!(removed = boundary, remaining = messages.len(), "Auto-snip complete");
        boundary
    }
}

// ---------------------------------------------------------------------------
// Snip Compact (AP-T10)
// ---------------------------------------------------------------------------

/// Marker that users can insert to request context truncation at that point.
pub const SNIP_MARKER: &str = "[SNIP]";

impl CompactionPipeline {
    /// Detect and process a snip marker in the message history.
    ///
    /// If `[SNIP]` is found, messages before (and including) the marker are
    /// either summarized (if a provider is available) or simply truncated.
    ///
    /// Returns the number of messages removed.
    pub async fn snip_compact(
        messages: &mut Vec<ChatMessage>,
        provider: Option<&dyn Provider>,
        model: &str,
        pipeline: Option<&CompactionPipeline>,
        context: Option<&CompactionContext>,
    ) -> Result<usize> {
        // Find the last SNIP marker
        let pos = messages.iter().rposition(|m| {
            m.content.iter().any(|b| {
                if let ContentBlock::Text { text } = b {
                    text.contains(SNIP_MARKER)
                } else {
                    false
                }
            })
        });

        let pos = match pos {
            Some(p) => p,
            None => return Ok(0),
        };

        info!(snip_position = pos, total = messages.len(), "Snip marker found");

        // If we have a pipeline + provider, try to summarize before truncating
        if let (Some(pipeline), Some(provider), Some(ctx)) = (pipeline, provider, context) {
            let to_summarize = &messages[..pos];
            if to_summarize.len() >= 2 {
                match pipeline.compact(to_summarize, provider, model, ctx).await {
                    Ok(result) => {
                        let removed = pos + 1;
                        messages.drain(..=pos);
                        // Insert boundary marker + summary at the front
                        messages.insert(0, result.boundary_marker);
                        for (i, msg) in result.summary_messages.into_iter().enumerate() {
                            messages.insert(1 + i, msg);
                        }
                        info!(removed, "Snip compact with summary");
                        return Ok(removed);
                    }
                    Err(e) => {
                        warn!(error = %e, "Snip summary failed, falling back to truncation");
                        // Fall through to simple truncation
                    }
                }
            }
        }

        // Simple truncation: remove everything up to and including the snip marker
        let removed = pos + 1;
        messages.drain(..=pos);
        info!(removed, "Snip compact (truncation only)");
        Ok(removed)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preprocess_replaces_images() {
        let msgs = vec![ChatMessage {
            role: MessageRole::User,
            content: vec![ContentBlock::Image {
                source_type: grid_types::ImageSourceType::Base64,
                media_type: "image/png".into(),
                data: "huge-base64-data".into(),
            }],
        }];
        let result = CompactionPipeline::preprocess_for_summary(&msgs);
        assert_eq!(result.len(), 1);
        match &result[0].content[0] {
            ContentBlock::Text { text } => assert_eq!(text, "[image]"),
            other => panic!("Expected Text, got {:?}", other),
        }
    }

    #[test]
    fn test_preprocess_truncates_long_tool_results() {
        let long_content = "x".repeat(5000);
        let msgs = vec![ChatMessage {
            role: MessageRole::User,
            content: vec![ContentBlock::ToolResult {
                tool_use_id: "t1".into(),
                content: long_content.clone(),
                is_error: false,
            }],
        }];
        let result = CompactionPipeline::preprocess_for_summary(&msgs);
        match &result[0].content[0] {
            ContentBlock::ToolResult { content, .. } => {
                assert!(content.len() < long_content.len());
                assert!(content.contains("[truncated, 5000 chars total]"));
            }
            other => panic!("Expected ToolResult, got {:?}", other),
        }
    }

    #[test]
    fn test_preprocess_keeps_short_tool_results() {
        let short = "ok";
        let msgs = vec![ChatMessage {
            role: MessageRole::User,
            content: vec![ContentBlock::ToolResult {
                tool_use_id: "t1".into(),
                content: short.into(),
                is_error: false,
            }],
        }];
        let result = CompactionPipeline::preprocess_for_summary(&msgs);
        match &result[0].content[0] {
            ContentBlock::ToolResult { content, .. } => assert_eq!(content, short),
            other => panic!("Expected ToolResult, got {:?}", other),
        }
    }

    #[test]
    fn test_format_summary_strips_analysis() {
        let raw = "<analysis>thinking...</analysis>\n<summary>\n1. Intent: foo\n</summary>";
        let result = CompactionPipeline::format_summary(raw);
        assert!(!result.contains("<analysis>"));
        assert!(!result.contains("thinking..."));
        assert!(result.contains("1. Intent: foo"));
        assert!(result.contains("continued from a previous conversation"));
    }

    #[test]
    fn test_format_summary_no_tags() {
        let raw = "Just plain summary text";
        let result = CompactionPipeline::format_summary(raw);
        assert!(result.contains("Just plain summary text"));
        assert!(result.contains("continued from a previous conversation"));
    }

    #[test]
    fn test_format_summary_analysis_only() {
        let raw = "<analysis>deep thoughts</analysis>\nSome remaining text";
        let result = CompactionPipeline::format_summary(raw);
        assert!(!result.contains("deep thoughts"));
        assert!(result.contains("Some remaining text"));
    }

    #[tokio::test]
    async fn test_snip_compact_truncation() {
        let mut messages = vec![
            ChatMessage {
                role: MessageRole::User,
                content: vec![ContentBlock::Text {
                    text: "old stuff".into(),
                }],
            },
            ChatMessage {
                role: MessageRole::Assistant,
                content: vec![ContentBlock::Text {
                    text: "old response".into(),
                }],
            },
            ChatMessage {
                role: MessageRole::User,
                content: vec![ContentBlock::Text {
                    text: "[SNIP]".into(),
                }],
            },
            ChatMessage {
                role: MessageRole::User,
                content: vec![ContentBlock::Text {
                    text: "new stuff".into(),
                }],
            },
        ];

        let removed = CompactionPipeline::snip_compact(&mut messages, None, "test", None, None)
            .await
            .unwrap();
        assert_eq!(removed, 3, "Should remove everything up to and including SNIP");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].text_content(), "new stuff");
    }

    #[tokio::test]
    async fn test_snip_compact_no_marker() {
        let mut messages = vec![ChatMessage {
            role: MessageRole::User,
            content: vec![ContentBlock::Text {
                text: "no snip here".into(),
            }],
        }];

        let removed = CompactionPipeline::snip_compact(&mut messages, None, "test", None, None)
            .await
            .unwrap();
        assert_eq!(removed, 0);
        assert_eq!(messages.len(), 1);
    }

    #[test]
    fn test_preprocess_replaces_documents() {
        let msgs = vec![ChatMessage {
            role: MessageRole::User,
            content: vec![ContentBlock::Document {
                source_type: "base64".into(),
                media_type: "application/pdf".into(),
                data: "pdf-data".into(),
            }],
        }];
        let result = CompactionPipeline::preprocess_for_summary(&msgs);
        match &result[0].content[0] {
            ContentBlock::Text { text } => assert_eq!(text, "[image]"),
            other => panic!("Expected Text, got {:?}", other),
        }
    }
}
