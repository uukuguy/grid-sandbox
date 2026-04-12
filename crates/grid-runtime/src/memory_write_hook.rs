//! MemoryWriteHook — PostToolUse hook that writes tool execution
//! evidence to the L2 Memory Engine.
//!
//! This hook is fire-and-forget (FailOpen + is_async=true) so that
//! L2 memory writes never block or abort agent execution.

use async_trait::async_trait;
use grid_engine::hooks::{HookAction, HookContext, HookFailureMode, HookHandler};
use tracing::{debug, warn};

use crate::l2_memory_client::{L2MemoryClient, WriteAnchorRequest};

/// PostToolUse hook that writes tool execution evidence to L2.
pub struct MemoryWriteHook {
    client: L2MemoryClient,
    session_id: String,
}

impl MemoryWriteHook {
    pub fn new(client: L2MemoryClient, session_id: String) -> Self {
        Self { client, session_id }
    }
}

#[async_trait]
impl HookHandler for MemoryWriteHook {
    fn name(&self) -> &str {
        "eaasp-memory-write"
    }

    fn priority(&self) -> u32 {
        900 // Runs after all other hooks
    }

    fn failure_mode(&self) -> HookFailureMode {
        HookFailureMode::FailOpen // Memory write failure must NOT block agent
    }

    fn is_async(&self) -> bool {
        true // Fire-and-forget: don't block tool execution pipeline
    }

    async fn execute(&self, ctx: &HookContext) -> anyhow::Result<HookAction> {
        let tool_name = ctx.tool_name.as_deref().unwrap_or("unknown");

        // Only write anchors for successful tool calls
        if !ctx.success.unwrap_or(false) {
            debug!(
                tool = %tool_name,
                "Skipping L2 anchor write for unsuccessful tool call"
            );
            return Ok(HookAction::Continue);
        }

        let event_id = format!(
            "tool-{}-{}",
            tool_name,
            chrono::Utc::now().timestamp_millis()
        );

        // Truncate data_ref to avoid oversized payloads
        let data_ref = ctx.tool_result.as_ref().map(|v| {
            let s = v.to_string();
            if s.len() > 500 {
                format!("{}...[truncated]", &s[..500])
            } else {
                s
            }
        });

        let req = WriteAnchorRequest {
            event_id,
            session_id: self.session_id.clone(),
            anchor_type: "tool_execution".to_string(),
            data_ref,
            snapshot_hash: None,
            source_system: Some("grid-runtime".to_string()),
        };

        debug!(
            tool = %tool_name,
            session = %self.session_id,
            "Writing tool execution evidence to L2"
        );

        if let Err(e) = self.client.write_anchor(&req).await {
            warn!(error = %e, tool = %tool_name, "Failed to write evidence anchor to L2 (non-fatal)");
        }

        Ok(HookAction::Continue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_hook() -> MemoryWriteHook {
        let client = L2MemoryClient::new("http://127.0.0.1:18085");
        MemoryWriteHook::new(client, "test-session".into())
    }

    #[test]
    fn hook_name() {
        assert_eq!(make_hook().name(), "eaasp-memory-write");
    }

    #[test]
    fn hook_priority_is_900() {
        assert_eq!(make_hook().priority(), 900);
    }

    #[test]
    fn hook_failure_mode_is_fail_open() {
        assert_eq!(make_hook().failure_mode(), HookFailureMode::FailOpen);
    }

    #[test]
    fn hook_is_async() {
        assert!(make_hook().is_async());
    }

    #[tokio::test]
    async fn skips_unsuccessful_tool_calls() {
        let hook = make_hook();
        let ctx = HookContext::new()
            .with_session("s1")
            .with_tool("bash", serde_json::json!({"command": "ls"}))
            .with_result(false, 100);
        let action = hook.execute(&ctx).await.unwrap();
        assert!(matches!(action, HookAction::Continue));
    }
}
