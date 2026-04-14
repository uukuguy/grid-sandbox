//! S2.T5 — Integration tests for the L3 per-turn aggregate spill helper.
//!
//! Covers the three cases called out in the blueprint §6:
//!   1. Under-budget aggregate is a no-op (nothing spilled).
//!   2. Over-budget aggregate spills the *largest* candidate first until the
//!      running total falls back under budget.
//!   3. Identical-size candidates tie-break by ascending index and the
//!      `blob_replacements` output is byte-identical across repeated runs
//!      (determinism guarantee).
//!
//! These tests pin the public contract of
//! `grid_engine::agent::turn_budget::enforce_turn_aggregate_budget`. They do
//! not exercise `run_agent_loop` — unit-scope is preferred so the helper stays
//! testable without the heavy agent-loop fixtures.

use grid_types::ContentBlock;
use tempfile::TempDir;

use grid_engine::agent::turn_budget::enforce_turn_aggregate_budget;
use grid_engine::storage::BlobStore;

/// Build a fresh tempdir-backed BlobStore for test isolation.
fn fresh_store() -> (BlobStore, TempDir) {
    let dir = TempDir::new().expect("tempdir");
    let store = BlobStore::new(dir.path().to_path_buf());
    (store, dir)
}

fn make_tool_result(id: &str, content: String) -> ContentBlock {
    ContentBlock::ToolResult {
        tool_use_id: id.to_string(),
        content,
        is_error: false,
    }
}

fn aggregate_chars(blocks: &[ContentBlock]) -> usize {
    blocks
        .iter()
        .filter_map(|b| match b {
            ContentBlock::ToolResult { content, .. } => Some(content.chars().count()),
            _ => None,
        })
        .sum()
}

/// Under-budget aggregate: helper must be a no-op. Content lengths stay
/// exactly the same and `blob_replacements` gains zero entries.
#[test]
fn test_aggregate_under_budget_noop() {
    let (store, _dir) = fresh_store();

    // [8K, 6K, 5K] = 19K chars. Well under budget.
    let mut tool_results = vec![
        make_tool_result("t0", "a".repeat(8_000)),
        make_tool_result("t1", "b".repeat(6_000)),
        make_tool_result("t2", "c".repeat(5_000)),
    ];
    let mut blob_replacements: Vec<(usize, String)> = Vec::new();

    let before_total = aggregate_chars(&tool_results);
    assert_eq!(before_total, 19_000);

    enforce_turn_aggregate_budget(&mut tool_results, &mut blob_replacements, &store, 200_000);

    assert!(
        blob_replacements.is_empty(),
        "under-budget input must not spill, got {} replacements",
        blob_replacements.len()
    );
    // Every result still inline, lengths untouched.
    for (i, want_len) in [(0usize, 8_000usize), (1, 6_000), (2, 5_000)] {
        match &tool_results[i] {
            ContentBlock::ToolResult { content, .. } => {
                assert_eq!(content.chars().count(), want_len, "idx={i}");
                assert!(
                    BlobStore::parse_blob_ref(content).is_none(),
                    "idx={i} must remain inline, got blob ref"
                );
            }
            _ => panic!("expected ToolResult at idx={i}"),
        }
    }
    assert_eq!(aggregate_chars(&tool_results), before_total);
}

/// Over-budget aggregate: helper must spill the largest entry first, then
/// continue (if needed) until total drops at or below budget. With
/// [120K, 70K, 30K] = 220K and a 200K budget, spilling the 120K entry alone
/// is enough (70K + 30K + ref ≈ 100K remaining).
#[test]
fn test_aggregate_over_budget_spills_largest_first() {
    let (store, _dir) = fresh_store();

    let content_big = "x".repeat(120_000);
    let content_mid = "y".repeat(70_000);
    let content_small = "z".repeat(30_000);

    let mut tool_results = vec![
        make_tool_result("t0", content_big.clone()),
        make_tool_result("t1", content_mid.clone()),
        make_tool_result("t2", content_small.clone()),
    ];
    let mut blob_replacements: Vec<(usize, String)> = Vec::new();

    assert_eq!(aggregate_chars(&tool_results), 220_000);

    enforce_turn_aggregate_budget(&mut tool_results, &mut blob_replacements, &store, 200_000);

    // Exactly one spill — the largest (idx 0, 120K).
    assert_eq!(
        blob_replacements.len(),
        1,
        "expected exactly 1 spill, got {:?}",
        blob_replacements.iter().map(|(i, _)| i).collect::<Vec<_>>()
    );
    assert_eq!(blob_replacements[0].0, 0, "largest-first expected idx=0");

    // Aggregate must now be at or below budget.
    let after_total = aggregate_chars(&tool_results);
    assert!(
        after_total <= 200_000,
        "post-spill aggregate {} must be <= 200_000 budget",
        after_total
    );

    // tool_results[0] is now a blob ref, and the blob store holds the
    // original content retrievable by hash.
    match &tool_results[0] {
        ContentBlock::ToolResult { content, .. } => {
            let hash = BlobStore::parse_blob_ref(content)
                .expect("tool_results[0] must be a blob ref after spill");
            let loaded = store.load(hash).expect("blob retrievable by hash");
            assert_eq!(
                loaded,
                content_big.as_bytes(),
                "blob store must contain the original 120K content"
            );
            assert_eq!(&blob_replacements[0].1, content);
        }
        _ => panic!("expected ToolResult at idx=0"),
    }

    // tool_results[1] and [2] remain inline with original content.
    match &tool_results[1] {
        ContentBlock::ToolResult { content, .. } => {
            assert_eq!(content, &content_mid, "idx=1 should stay inline");
        }
        _ => panic!("expected ToolResult at idx=1"),
    }
    match &tool_results[2] {
        ContentBlock::ToolResult { content, .. } => {
            assert_eq!(content, &content_small, "idx=2 should stay inline");
        }
        _ => panic!("expected ToolResult at idx=2"),
    }
}

/// Tie-break determinism: two results of identical size, both over budget.
/// Sort must tie-break by ascending index, so the spill order is idx=0 then
/// idx=1 (when both fire). And re-running the exact same fixture must
/// produce byte-identical `blob_replacements` every time (no randomness,
/// BlobStore is content-addressed and idempotent).
#[test]
fn test_aggregate_tie_break_deterministic() {
    // Scenario: [110K, 110K] = 220K over a 200K budget. After spilling idx=0
    // (110K), total is ~110K inline + one blob ref (~80 chars) — already
    // under budget — so only idx=0 is spilled in this particular setup. That
    // asymmetric spill is exactly what the tie-break rule guarantees.

    fn run_once() -> (Vec<ContentBlock>, Vec<(usize, String)>) {
        let (store, _dir) = fresh_store();
        let mut tool_results = vec![
            make_tool_result("t0", "A".repeat(110_000)),
            make_tool_result("t1", "B".repeat(110_000)),
        ];
        let mut blob_replacements: Vec<(usize, String)> = Vec::new();
        enforce_turn_aggregate_budget(&mut tool_results, &mut blob_replacements, &store, 200_000);
        // Leak _dir deliberately via the function's scope boundary — the
        // BlobStore content is no longer needed once the helper has
        // recorded the blob ref. We only compare blob_replacements and
        // tool_results shape across invocations.
        (tool_results, blob_replacements)
    }

    let (results_run1, reps_run1) = run_once();

    // Exactly 1 spill, idx=0 first (ascending-index tie-break).
    assert_eq!(
        reps_run1.len(),
        1,
        "110K+110K over 200K budget should need a single spill, got {:?}",
        reps_run1.iter().map(|(i, _)| i).collect::<Vec<_>>()
    );
    assert_eq!(reps_run1[0].0, 0, "tie-break must pick ascending index 0");

    // idx=0 is a blob ref; idx=1 stays inline.
    match &results_run1[0] {
        ContentBlock::ToolResult { content, .. } => {
            assert!(
                BlobStore::parse_blob_ref(content).is_some(),
                "idx=0 should be a blob ref"
            );
        }
        _ => panic!("expected ToolResult at idx=0"),
    }
    match &results_run1[1] {
        ContentBlock::ToolResult { content, .. } => {
            assert_eq!(
                content.chars().count(),
                110_000,
                "idx=1 should remain inline"
            );
        }
        _ => panic!("expected ToolResult at idx=1"),
    }

    // Run the same fixture four more times; blob_replacements must match
    // exactly (deterministic — content-addressed, sort is stable).
    for attempt in 1..5 {
        let (_, reps_n) = run_once();
        assert_eq!(
            reps_n, reps_run1,
            "attempt #{attempt}: blob_replacements diverged from run 1 — helper is non-deterministic"
        );
    }
}
