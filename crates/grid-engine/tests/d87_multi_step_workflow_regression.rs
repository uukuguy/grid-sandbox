//! D87 regression test — grid-engine agent loop 多步工作流过早终止 bug.
//!
//! **Discovered by Phase 1 E2E verification (2026-04-14)**:
//! threshold-calibration skill defines 6-step workflow (scada_read → memory_search →
//! memory_read → memory_write_anchor → memory_write_file → final JSON).
//!
//! - grid-runtime (grid-engine): only executed step 1, then exited with text
//! - claude-code-runtime (Anthropic SDK): autonomously executed all 4-6 steps
//!
//! **Root cause**: `crates/grid-engine/src/agent/harness.rs:1169` condition
//! `stop_reason != ToolUse || tool_uses.is_empty()` terminates the loop on any
//! non-tool-use text, even when the agent should continue a multi-step workflow.
//!
//! **Expected fix (Phase 2)**: After tool execution, append tool_result as a
//! user message and re-invoke the LLM (Anthropic SDK pattern). Only exit when
//! stop_reason=EndTurn AND the LLM explicitly signals completion.
//!
//! This test is **expected to FAIL** on current grid-engine until D87 is fixed.
//! The `#[ignore]` attribute prevents it from blocking CI; remove when fixing D87.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use futures_util::stream::{self, StreamExt};
use serde_json::json;

use grid_engine::agent::{run_agent_loop, AgentConfig, AgentEvent, AgentLoopConfig};
use grid_engine::providers::{CompletionStream, Provider};
use grid_engine::tools::{Tool, ToolRegistry};
use grid_types::{
    ChatMessage, CompletionRequest, CompletionResponse, StopReason, StreamEvent,
    ToolContext, ToolOutput, ToolSource, TokenUsage,
};

// ---------------------------------------------------------------------------
// Multi-step MockProvider — simulates a 3-tool workflow skill
// ---------------------------------------------------------------------------

/// Provider that reproduces the D87 scenario: agent loop exits on text-only
/// response mid-workflow, even though the workflow is not yet complete.
///
/// **Real-world E2E scenario** (grid-runtime + threshold-calibration skill,
/// 2026-04-14): LLM calls `scada_read_snapshot` (step 1), receives tool result,
/// then emits text asking user "是否需要我：1/2/3?" with stop_reason=EndTurn
/// WITHOUT emitting another tool_use block. Agent loop terminates. User sees
/// only 1 of 6 skill steps executed.
///
/// Mock sequence:
///   Call 0: tool_use (read_data)      — stop_reason=ToolUse  → loop continues
///   Call 1: text_only (asks user)     — stop_reason=EndTurn  ← LOOP EXITS HERE (broken)
///           (expected: loop should re-prompt LLM with "continue the workflow"
///            context until workflow completes, per Anthropic SDK pattern)
///
/// This test represents what we WANT grid-engine to do after D87 is fixed:
/// when the LLM emits a text-only response mid-workflow without explicit
/// "task complete" signal, the loop should nudge it to continue (or at least
/// not silently drop steps that the skill contract requires).
///
/// **Note**: This specific assertion may need Phase 2 design discussion —
/// one possible fix is to retry with a continuation hint; another is to
/// always execute a minimum N tool calls when the skill prose specifies them.
/// This test encodes the MINIMUM behavior expected.
struct MultiStepProvider {
    call_count: AtomicU32,
}

impl MultiStepProvider {
    fn new() -> Self {
        Self {
            call_count: AtomicU32::new(0),
        }
    }

    fn tool_use_stream(tool_name: &str, tool_id: &str, input: serde_json::Value) -> CompletionStream {
        let events: Vec<Result<StreamEvent>> = vec![
            Ok(StreamEvent::MessageStart {
                id: format!("msg_{}", tool_id),
            }),
            Ok(StreamEvent::TextDelta {
                text: format!("Now calling {}.", tool_name),
            }),
            Ok(StreamEvent::ToolUseComplete {
                index: 0,
                id: tool_id.to_string(),
                name: tool_name.to_string(),
                input,
            }),
            Ok(StreamEvent::MessageStop {
                stop_reason: StopReason::ToolUse,
                usage: TokenUsage {
                    input_tokens: 100,
                    output_tokens: 50,
                },
            }),
        ];
        Box::pin(stream::iter(events))
    }

    /// Mimics the grid-runtime 2026-04-14 E2E observation: LLM emits text
    /// asking user for guidance with stop_reason=EndTurn (no tool_use block).
    /// This is what triggers D87 — the harness treats it as "task complete".
    fn ask_user_text_stream(text: &str) -> CompletionStream {
        let events: Vec<Result<StreamEvent>> = vec![
            Ok(StreamEvent::MessageStart {
                id: "msg_ask_user".into(),
            }),
            Ok(StreamEvent::TextDelta {
                text: text.to_string(),
            }),
            Ok(StreamEvent::MessageStop {
                stop_reason: StopReason::EndTurn, // ← the D87 trigger
                usage: TokenUsage {
                    input_tokens: 150,
                    output_tokens: 40,
                },
            }),
        ];
        Box::pin(stream::iter(events))
    }

    fn final_text_stream() -> CompletionStream {
        let events: Vec<Result<StreamEvent>> = vec![
            Ok(StreamEvent::MessageStart {
                id: "msg_final".into(),
            }),
            Ok(StreamEvent::TextDelta {
                text: "Workflow complete. All steps executed.".into(),
            }),
            Ok(StreamEvent::MessageStop {
                stop_reason: StopReason::EndTurn,
                usage: TokenUsage {
                    input_tokens: 200,
                    output_tokens: 100,
                },
            }),
        ];
        Box::pin(stream::iter(events))
    }
}

#[async_trait]
impl Provider for MultiStepProvider {
    fn id(&self) -> &str {
        "multi-step-mock"
    }

    async fn complete(&self, _request: CompletionRequest) -> Result<CompletionResponse> {
        unimplemented!("streaming only")
    }

    async fn stream(&self, _request: CompletionRequest) -> Result<CompletionStream> {
        let n = self.call_count.fetch_add(1, Ordering::SeqCst);
        match n {
            // Call 0: LLM calls read_data. Normal tool_use path. Loop continues.
            0 => Ok(Self::tool_use_stream(
                "read_data",
                "toolu_step1",
                json!({"device_id": "T-001"}),
            )),
            // Call 1: LLM returns TEXT ONLY with stop_reason=EndTurn, asking
            // the user for guidance mid-workflow. This is the D87 trigger —
            // real grid-runtime exits here, skipping the remaining skill steps.
            //
            // Expected D87 fix: loop should inject a continuation prompt ("the
            // skill requires you to also call search_history and write_result
            // — continue") OR the skill prose should be re-injected to force
            // the LLM to continue. After the fix, Call 2 and Call 3 should
            // be invoked.
            1 => Ok(Self::ask_user_text_stream(
                "I have read the data. Do you want me to: \
                 1) search history?  2) write result?  3) skip?",
            )),
            // Call 2 (only reached after D87 fix): LLM continues to search.
            2 => Ok(Self::tool_use_stream(
                "search_history",
                "toolu_step2",
                json!({"query": "T-001 baseline"}),
            )),
            // Call 3 (only reached after D87 fix): LLM writes result.
            3 => Ok(Self::tool_use_stream(
                "write_result",
                "toolu_step3",
                json!({"memory_id": "mem_T-001_v1"}),
            )),
            // Call 4+: final summary.
            _ => Ok(Self::final_text_stream()),
        }
    }
}

// ---------------------------------------------------------------------------
// Stub tools — match the tool names produced by MultiStepProvider
// ---------------------------------------------------------------------------

macro_rules! stub_tool {
    ($struct_name:ident, $name:expr, $desc:expr) => {
        struct $struct_name;

        #[async_trait]
        impl Tool for $struct_name {
            fn name(&self) -> &str {
                $name
            }

            fn description(&self) -> &str {
                $desc
            }

            fn parameters(&self) -> serde_json::Value {
                json!({"type": "object", "properties": {}})
            }

            async fn execute(
                &self,
                _params: serde_json::Value,
                _ctx: &ToolContext,
            ) -> Result<ToolOutput> {
                Ok(ToolOutput::success(format!("{} ok", $name)))
            }

            fn source(&self) -> ToolSource {
                ToolSource::BuiltIn
            }
        }
    };
}

stub_tool!(ReadDataTool, "read_data", "Read data");
stub_tool!(SearchHistoryTool, "search_history", "Search history");
stub_tool!(WriteResultTool, "write_result", "Write result");

// ---------------------------------------------------------------------------
// Test — EXPECTED TO FAIL until D87 is fixed
// ---------------------------------------------------------------------------

/// D87 regression: agent loop must execute all tool calls in a multi-step
/// workflow, not exit after the first non-tool-use text.
///
/// Expected behavior (after D87 fix):
/// - Loop executes 3 tool calls (read_data, search_history, write_result)
/// - Loop exits only on EndTurn with no tool use (call 3: final text)
///
/// Current broken behavior (pre-D87 fix):
/// - Depending on exact harness logic, may exit after round 0 or fewer than 3 tools
///
/// This test is `#[ignore]`'d so CI doesn't fail. Remove `#[ignore]` when
/// starting D87 fix work to lock in the regression.
#[tokio::test]
#[ignore = "D87 pending fix — grid-engine harness.rs:1169 terminates loop on text-only response"]
async fn test_d87_multi_step_workflow_no_early_exit() {
    let provider = Arc::new(MultiStepProvider::new());
    let mut registry = ToolRegistry::new();
    registry.register(ReadDataTool);
    registry.register(SearchHistoryTool);
    registry.register(WriteResultTool);
    let registry = Arc::new(registry);

    let config = AgentLoopConfig::builder()
        .provider(provider)
        .tools(registry)
        .model("mock-model".into())
        .max_tokens(1024)
        .max_iterations(10)
        .force_text_at_last(false)
        .agent_config(AgentConfig {
            enable_typing_signal: false,
            enable_parallel: false,
            ..AgentConfig::default()
        })
        .build();

    let messages = vec![ChatMessage::user(
        "Run the full workflow: read data, search history, write result.",
    )];

    let events: Vec<AgentEvent> = run_agent_loop(config, messages).collect().await;

    // Count executed tool starts.
    let tool_starts: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            AgentEvent::ToolStart { tool_name, .. } => Some(tool_name.as_str()),
            _ => None,
        })
        .collect();

    let completed_rounds = events.iter().find_map(|e| match e {
        AgentEvent::Completed(r) => Some(r.rounds),
        _ => None,
    });

    // Assertion 1: ALL three expected tools were called
    assert_eq!(
        tool_starts.len(),
        3,
        "D87: Expected 3 tool calls (read_data, search_history, write_result), \
         got {}. Tool calls seen: {:?}. \
         This proves the agent loop terminated early after a text-only response \
         instead of continuing the multi-step workflow. \
         Fix harness.rs:1169 per ADR or Phase 2 plan.",
        tool_starts.len(),
        tool_starts
    );

    assert!(tool_starts.contains(&"read_data"), "read_data missing");
    assert!(tool_starts.contains(&"search_history"), "search_history missing");
    assert!(tool_starts.contains(&"write_result"), "write_result missing");

    // Assertion 2: Loop ran enough rounds to cover all tools + final summary
    assert!(
        completed_rounds.unwrap_or(0) >= 4,
        "D87: Expected ≥4 rounds (3 tool calls + 1 final text), got {:?}",
        completed_rounds
    );
}

/// D87 companion test: single-tool workflow should still work correctly
/// (no regression on existing behavior).
///
/// This one is NOT ignored — it's the baseline that the D87 fix must preserve.
#[tokio::test]
async fn test_d87_single_tool_workflow_still_works() {
    // Provider that emits exactly 1 tool call then final text.
    struct OneTool {
        n: AtomicU32,
    }

    #[async_trait]
    impl Provider for OneTool {
        fn id(&self) -> &str {
            "one-tool"
        }
        async fn complete(&self, _r: CompletionRequest) -> Result<CompletionResponse> {
            unimplemented!()
        }
        async fn stream(&self, _r: CompletionRequest) -> Result<CompletionStream> {
            let n = self.n.fetch_add(1, Ordering::SeqCst);
            if n == 0 {
                Ok(MultiStepProvider::tool_use_stream(
                    "read_data",
                    "toolu_only",
                    json!({}),
                ))
            } else {
                Ok(MultiStepProvider::final_text_stream())
            }
        }
    }

    let provider = Arc::new(OneTool {
        n: AtomicU32::new(0),
    });
    let mut registry = ToolRegistry::new();
    registry.register(ReadDataTool);
    let registry = Arc::new(registry);

    let config = AgentLoopConfig::builder()
        .provider(provider)
        .tools(registry)
        .model("mock-model".into())
        .max_tokens(1024)
        .max_iterations(10)
        .force_text_at_last(true)
        .agent_config(AgentConfig {
            enable_typing_signal: false,
            enable_parallel: false,
            ..AgentConfig::default()
        })
        .build();

    let messages = vec![ChatMessage::user("Read data")];
    let events: Vec<AgentEvent> = run_agent_loop(config, messages).collect().await;

    let tool_calls = events
        .iter()
        .filter(|e| matches!(e, AgentEvent::ToolStart { .. }))
        .count();

    assert_eq!(tool_calls, 1, "Single-tool workflow should execute 1 tool");

    // Make sure we saw Completed
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AgentEvent::Completed { .. })),
        "Loop should complete normally"
    );
}
