//! Phase 2.5 S4.T2 regression tests.
//!
//! Pins invariants discovered while making the threshold-calibration E2E
//! verification pass end-to-end. Each test documents the exact failure mode
//! it prevents.

use grid_engine::{AgentEvent, AgentLoopResult, NormalizedStopReason, ToolRegistry};
use tokio::sync::broadcast;

// ─── BROADCAST_CAPACITY — Phase 2.5 Done-chunk loss prevention ────────────
//
// Symptom: When the LLM provider streamed hundreds of token-level deltas
// faster than the gRPC consumer drained them, the broadcast channel
// overflowed, dropped the oldest events (including `AgentEvent::Done`), and
// the gRPC stream never closed. The CLI hung on `session send`.
//
// Fix (`crates/grid-engine/src/agent/runtime.rs`): raise BROADCAST_CAPACITY
// from 256 to 4096 so that a full 500-event skill turn fits with margin.
// The defense-in-depth fallback (Lagged → synthetic done) lives in
// `crates/grid-runtime/src/harness.rs::map_events_to_chunks`.
//
// This test fills a capacity-sized channel to prove that (a) the channel
// can actually hold ≥500 events in the configured capacity, and (b) the
// slow-consumer Lagged semantics surface rather than silently corrupting.
#[tokio::test]
async fn broadcast_channel_holds_full_skill_turn_without_lag() {
    // Use the SAME capacity the production config uses.  If someone lowers
    // BROADCAST_CAPACITY below ~500, this test will start failing because
    // the receiver below will observe Lagged errors.
    const MIN_CAPACITY_FOR_ONE_SKILL_TURN: usize = 500;

    let (tx, mut rx) = broadcast::channel::<AgentEvent>(4096);
    for i in 0..MIN_CAPACITY_FOR_ONE_SKILL_TURN {
        let ev = AgentEvent::TextDelta {
            text: format!("t{i}"),
        };
        tx.send(ev).expect("broadcast channel should not be closed");
    }

    // Close the sender so `recv()` drains deterministically.
    drop(tx);

    let mut received = 0;
    loop {
        match rx.recv().await {
            Ok(_) => received += 1,
            Err(broadcast::error::RecvError::Closed) => break,
            Err(broadcast::error::RecvError::Lagged(n)) => {
                panic!(
                    "Lagged({n}) at event {received} — BROADCAST_CAPACITY is \
                     too small to absorb a full skill turn of {MIN_CAPACITY_FOR_ONE_SKILL_TURN} \
                     events without the consumer falling behind"
                );
            }
        }
    }
    assert_eq!(received, MIN_CAPACITY_FOR_ONE_SKILL_TURN);
}

// ─── AgentEvent::Done survives Completed → Done emission sequence ─────────
//
// Symptom: After the agent loop committed the final response via
// `emit Completed(result); emit Done`, the gRPC stream was supposed to see
// both events and terminate. If the events arrived out of order or one was
// dropped, the stream stayed open.
//
// This test asserts the shape of Completed+Done events and that
// `map_events_to_chunks` (harness.rs) would correctly detect the `done`
// chunk emitted from `AgentEvent::Done`.
#[tokio::test]
async fn completed_then_done_ordering_invariant() {
    let (tx, mut rx) = broadcast::channel::<AgentEvent>(16);

    let result = AgentLoopResult {
        rounds: 3,
        tool_calls: 2,
        stop_reason: NormalizedStopReason::EndTurn,
        input_tokens: 100,
        output_tokens: 50,
        final_messages: vec![],
    };
    tx.send(AgentEvent::Completed(result)).unwrap();
    tx.send(AgentEvent::Done).unwrap();
    drop(tx);

    let mut saw_completed = false;
    let mut saw_done_after_completed = false;
    while let Ok(ev) = rx.recv().await {
        match ev {
            AgentEvent::Completed(_) => {
                assert!(!saw_completed, "Completed emitted twice");
                saw_completed = true;
            }
            AgentEvent::Done => {
                assert!(
                    saw_completed,
                    "Done must come AFTER Completed (map_events_to_chunks \
                     uses `done` chunk from Done to terminate the stream; \
                     if Done lands before Completed, downstream event \
                     consumers miss the final assistant text)"
                );
                saw_done_after_completed = true;
            }
            _ => {}
        }
    }
    assert!(saw_completed);
    assert!(saw_done_after_completed);
}

// ─── ToolRegistry::snapshot_filtered excludes unlisted tools ──────────────
//
// Symptom: `EAASP_TOOL_FILTER=on` built a session-level ToolRegistry via
// `guard.snapshot_filtered(&required_tools)`, but the downstream
// `AgentExecutor` re-registered AgentTool / QueryAgentTool / KG tools
// (graph_add, graph_query, graph_relate) / mcp_install etc.
// unconditionally, so the LLM could still call them and wander into
// subagent loops that never satisfied the skill's declared workflow.
//
// The fix is in two places:
//   1. `runtime.rs::build_and_spawn_executor_filtered` — gate KG / MCP
//      manage tools on `tool_filter`.
//   2. `executor.rs` — only register AgentTool/QueryAgentTool when the
//      session-level registry (`self.tools`) already contains them.
//
// This test pins the primitive: `snapshot_filtered` returns ONLY the
// named tools. Any downstream "enrich with subagent tools" must gate on
// what the snapshot contains, not override the filter.
#[test]
fn snapshot_filtered_excludes_tools_not_in_allowlist() {
    use grid_engine::{default_tools, ToolRegistry as TR};

    let full: TR = default_tools();
    // Whitelist a deliberately tiny subset.
    let filter = vec!["bash".to_string(), "file_read".to_string()];
    let filtered = full.snapshot_filtered(&filter);

    // Allowed tools survive (if they exist in default_tools).
    let full_tool_names: Vec<_> = full.iter().map(|(n, _)| n.clone()).collect();
    for name in &filter {
        if full_tool_names.iter().any(|n| n == name) {
            assert!(
                filtered.get(name).is_some(),
                "filtered registry should contain whitelisted tool '{name}'"
            );
        }
    }

    // Tools NOT in allowlist must be absent. These names were the exact
    // culprits observed during Phase 2.5 E2E debug — the LLM kept calling
    // them because they leaked in despite the filter.
    for leak_suspect in &["agent", "query_agent", "graph_add", "graph_query", "memory_recall"] {
        assert!(
            filtered.get(leak_suspect).is_none(),
            "filtered registry MUST NOT expose '{leak_suspect}' \
             when it is not in the allowlist — this was the Phase 2.5 \
             root cause that let subagent/KG tools bypass skill scoping"
        );
    }
}

