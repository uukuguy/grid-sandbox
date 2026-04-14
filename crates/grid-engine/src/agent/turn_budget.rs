//! S2.T5 — Per-turn aggregate budget enforcement for tool results (L3 defense).
//!
//! The agent loop's tool-result pipeline has three layers of protection:
//!
//! * **L1** (inside tools / `postprocess_tool_output`) — per-call hard caps.
//! * **L2** (`harness.rs` blob-spill loop) — per-result spill when a single
//!   result exceeds `BLOB_THRESHOLD_BYTES`.
//! * **L3** (this module) — *aggregate* spill when the sum of all inline
//!   `ToolResult` contents in a single assistant turn exceeds
//!   `TOOL_RESULT_TURN_BUDGET` (see `harness.rs`).
//!
//! L3 runs after L2 has fully populated `blob_replacements` and
//! `tool_results`, but before the `ChatMessage` is pushed onto the history.
//! That ordering ensures the LLM sees the trimmed view this turn *and* the
//! blob refs are persisted consistently by the drain step that follows.
//!
//! The helper is exposed as a plain `pub fn` (not a method on harness
//! internals) so integration tests can exercise it without touching
//! `run_agent_loop`.

use grid_types::ContentBlock;
use tracing::debug;

use crate::storage::BlobStore;

/// Enforce a per-turn aggregate budget on `tool_results`.
///
/// * Sums char-count (not byte length) across every
///   `ContentBlock::ToolResult` content field. Char counting matches
///   `soft_trim_tool_result` accounting in `harness.rs` and avoids the
///   UTF-8 multi-byte miscount that `.len()` would introduce.
/// * If the total is `<= budget`, returns early (no-op).
/// * Otherwise collects `(idx, char_len)` for every `ToolResult` whose
///   content does **not** already parse as a blob ref (via
///   [`BlobStore::parse_blob_ref`]). This idempotency guard is essential:
///   L2 may have already spilled some entries, and re-spilling the compact
///   ref string itself would corrupt the output and grow the blob store.
/// * Sorts candidates by char_len descending (spill largest first to reach
///   budget with the fewest spill ops) with original-index ascending as a
///   deterministic tie-break.
/// * Walks the list, storing each selected content into `blob_store`,
///   replacing `tool_results[idx]` with the returned blob ref, and
///   appending `(idx, ref_str)` to `blob_replacements` so the downstream
///   drain step persists the compact ref in the message history.
/// * Stops as soon as the running total falls back under `budget`.
///
/// The function is infallible with respect to the caller: if
/// `blob_store.store()` fails for a particular entry we simply skip that
/// entry and keep trying others. This mirrors the L2 behaviour in
/// `harness.rs` (a warn-and-keep-inline fallback).
pub fn enforce_turn_aggregate_budget(
    tool_results: &mut Vec<ContentBlock>,
    blob_replacements: &mut Vec<(usize, String)>,
    blob_store: &BlobStore,
    budget: usize,
) {
    // Step 1: compute running total across ToolResult entries (chars, not bytes).
    let mut total: usize = tool_results
        .iter()
        .filter_map(|b| match b {
            ContentBlock::ToolResult { content, .. } => Some(content.chars().count()),
            _ => None,
        })
        .sum();

    if total <= budget {
        return;
    }

    // Step 2: gather spill candidates — only results that are NOT already blob refs.
    let mut candidates: Vec<(usize, usize)> = tool_results
        .iter()
        .enumerate()
        .filter_map(|(idx, b)| match b {
            ContentBlock::ToolResult { content, .. } => {
                if BlobStore::parse_blob_ref(content).is_some() {
                    None // L2 already spilled this one; skip to keep idempotent.
                } else {
                    Some((idx, content.chars().count()))
                }
            }
            _ => None,
        })
        .collect();

    // Step 3: sort by char_len DESC with idx ASC as deterministic tie-break.
    candidates.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    let mut freed_bytes: usize = 0;
    let mut spilled_count: usize = 0;

    // Step 4: spill one at a time until under budget.
    for (idx, char_len) in candidates {
        if total <= budget {
            break;
        }

        // Extract the content to spill (avoid double-borrow of tool_results).
        let Some(ContentBlock::ToolResult { content, .. }) = tool_results.get_mut(idx) else {
            continue;
        };

        let hash = match blob_store.store(content.as_bytes()) {
            Ok(h) => h,
            Err(e) => {
                tracing::warn!(idx, error = %e, "L3 aggregate spill failed, keeping inline");
                continue;
            }
        };
        let ref_str = BlobStore::format_blob_ref(&hash);

        *content = ref_str.clone();
        blob_replacements.push((idx, ref_str));

        // char_len of the spilled content is removed from the running total;
        // the blob ref (~80 chars) is negligible compared to ≥4K content but
        // we subtract precisely so the early-exit is exact.
        let ref_chars = blob_replacements
            .last()
            .map(|(_, r)| r.chars().count())
            .unwrap_or(0);
        total = total.saturating_sub(char_len).saturating_add(ref_chars);
        freed_bytes = freed_bytes.saturating_add(char_len);
        spilled_count += 1;
    }

    if spilled_count > 0 {
        debug!(
            freed_bytes,
            spilled_count,
            remaining_total = total,
            "L3 per-turn aggregate spill triggered"
        );
    }
}
