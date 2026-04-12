//! ScopedHookHandler — bridges EAASP skill-frontmatter scoped hooks
//! into grid-engine's HookHandler trait.
//!
//! Each instance wraps ONE scoped hook (one command at one hook point).
//! Multiple ScopedHookHandlers are registered per-point during initialize().

use async_trait::async_trait;
use grid_engine::hooks::declarative::{execute_command, HookDecision};
use grid_engine::hooks::{HookAction, HookContext, HookFailureMode, HookHandler};
use tracing::{debug, warn};

/// A HookHandler that wraps a single EAASP skill-frontmatter scoped hook.
///
/// Scoped hooks are shell commands declared in skill YAML frontmatter.
/// They run at PreToolUse, PostToolUse, or Stop hook points and use
/// the same protocol as declarative command hooks:
/// - Exit 0 with JSON `{"decision":"allow"}` → Continue
/// - Exit 2 with stderr reason → Block (deny)
/// - Exit 0 with JSON `{"decision":"deny"}` → Block (deny)
pub struct ScopedHookHandler {
    /// Unique hook ID from skill frontmatter.
    hook_id: String,
    /// Shell command to execute (already variable-substituted).
    command: String,
    /// Tool name pattern to match (from `condition` field). Empty = match all.
    condition: String,
    /// Precedence (lower runs first).
    precedence: i32,
    /// Timeout in seconds.
    timeout_secs: u32,
}

impl ScopedHookHandler {
    /// Create a new ScopedHookHandler.
    ///
    /// - `hook_id`: Unique identifier from skill frontmatter.
    /// - `command`: Shell command to run (already substituted).
    /// - `condition`: Tool name glob pattern (e.g. `"scada_write*"`). Empty matches all.
    /// - `precedence`: Lower runs first; mapped into priority band 200-499.
    pub fn new(hook_id: String, command: String, condition: String, precedence: i32) -> Self {
        Self {
            hook_id,
            command,
            condition,
            precedence,
            timeout_secs: 5,
        }
    }

    /// Check if this hook's condition matches the tool name.
    ///
    /// Supports:
    /// - Empty or `"*"` → match all
    /// - Trailing `*` → prefix match (e.g. `"scada_write*"` matches `"scada_write_temperature"`)
    /// - Exact match otherwise
    fn matches_tool(&self, tool_name: &str) -> bool {
        if self.condition.is_empty() || self.condition == "*" {
            return true;
        }
        // Glob-like: trailing * is prefix match
        if self.condition.ends_with('*') {
            let prefix = &self.condition[..self.condition.len() - 1];
            return tool_name.starts_with(prefix);
        }
        self.condition == tool_name
    }
}

#[async_trait]
impl HookHandler for ScopedHookHandler {
    fn name(&self) -> &str {
        &self.hook_id
    }

    fn priority(&self) -> u32 {
        // Scoped hooks run after builtin (10) and policy (100) but before
        // declarative (500). Map precedence into band 200-499.
        (200 + self.precedence.max(0) as u32).min(499)
    }

    fn failure_mode(&self) -> HookFailureMode {
        // Scoped hooks are fail-open by default (EAASP spec).
        HookFailureMode::FailOpen
    }

    async fn execute(&self, ctx: &HookContext) -> anyhow::Result<HookAction> {
        let tool_name = ctx.tool_name.as_deref().unwrap_or("");

        // Check condition match — skip if tool doesn't match pattern
        if !self.matches_tool(tool_name) {
            return Ok(HookAction::Continue);
        }

        debug!(
            hook_id = %self.hook_id,
            command = %self.command,
            tool = %tool_name,
            "Executing scoped hook"
        );

        match execute_command(&self.command, ctx, self.timeout_secs).await {
            Ok(decision) => self.decision_to_action(decision),
            Err(e) => {
                warn!(
                    hook_id = %self.hook_id,
                    error = %e,
                    "Scoped hook execution failed (fail-open)"
                );
                // Propagate error — HookRegistry respects failure_mode (FailOpen)
                Err(e)
            }
        }
    }
}

impl ScopedHookHandler {
    /// Convert a command_executor HookDecision into a HookAction.
    fn decision_to_action(&self, decision: HookDecision) -> anyhow::Result<HookAction> {
        if decision.is_deny() {
            let reason = decision.reason.unwrap_or_else(|| {
                format!("Denied by scoped hook '{}'", self.hook_id)
            });
            Ok(HookAction::Block(reason))
        } else {
            // allow or ask → continue
            Ok(HookAction::Continue)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_tool_wildcard() {
        let h = ScopedHookHandler::new("t".into(), "echo".into(), "*".into(), 0);
        assert!(h.matches_tool("anything"));
    }

    #[test]
    fn matches_tool_glob_prefix() {
        let h = ScopedHookHandler::new("t".into(), "echo".into(), "scada_write*".into(), 0);
        assert!(h.matches_tool("scada_write"));
        assert!(h.matches_tool("scada_write_temperature"));
        assert!(!h.matches_tool("scada_read_snapshot"));
    }

    #[test]
    fn matches_tool_exact() {
        let h = ScopedHookHandler::new("t".into(), "echo".into(), "scada_write".into(), 0);
        assert!(h.matches_tool("scada_write"));
        assert!(!h.matches_tool("scada_write_temp"));
    }

    #[test]
    fn matches_tool_empty_matches_all() {
        let h = ScopedHookHandler::new("t".into(), "echo".into(), "".into(), 0);
        assert!(h.matches_tool("anything"));
    }

    #[test]
    fn priority_maps_precedence_into_band() {
        let h0 = ScopedHookHandler::new("t".into(), "echo".into(), "".into(), 0);
        assert_eq!(h0.priority(), 200);

        let h10 = ScopedHookHandler::new("t".into(), "echo".into(), "".into(), 10);
        assert_eq!(h10.priority(), 210);

        // Negative precedence is clamped to 0
        let hn = ScopedHookHandler::new("t".into(), "echo".into(), "".into(), -5);
        assert_eq!(hn.priority(), 200);

        // Very large precedence is capped at 499
        let hmax = ScopedHookHandler::new("t".into(), "echo".into(), "".into(), 1000);
        assert_eq!(hmax.priority(), 499);
    }

    #[test]
    fn failure_mode_is_fail_open() {
        let h = ScopedHookHandler::new("t".into(), "echo".into(), "".into(), 0);
        assert_eq!(h.failure_mode(), HookFailureMode::FailOpen);
    }

    #[tokio::test]
    async fn execute_allow_returns_continue() {
        let h = ScopedHookHandler::new(
            "test-allow".into(),
            r#"echo '{"decision":"allow"}'"#.into(),
            "*".into(),
            0,
        );
        let ctx = HookContext::new().with_tool("bash", serde_json::json!({}));
        let action = h.execute(&ctx).await.unwrap();
        assert!(
            matches!(action, HookAction::Continue),
            "expected Continue, got {:?}",
            action
        );
    }

    #[tokio::test]
    async fn execute_deny_exit2_returns_block() {
        let h = ScopedHookHandler::new(
            "test-deny".into(),
            "echo 'blocked' >&2; exit 2".into(),
            "*".into(),
            0,
        );
        let ctx = HookContext::new().with_tool("bash", serde_json::json!({}));
        let action = h.execute(&ctx).await.unwrap();
        assert!(
            matches!(action, HookAction::Block(ref r) if r == "blocked"),
            "expected Block(blocked), got {:?}",
            action
        );
    }

    #[tokio::test]
    async fn condition_mismatch_skips_execution() {
        let h = ScopedHookHandler::new(
            "test-skip".into(),
            "exit 2".into(), // would deny if executed
            "scada_write*".into(),
            0,
        );
        let ctx = HookContext::new().with_tool("scada_read_snapshot", serde_json::json!({}));
        let action = h.execute(&ctx).await.unwrap();
        assert!(
            matches!(action, HookAction::Continue),
            "expected Continue (condition mismatch), got {:?}",
            action
        );
    }

    #[tokio::test]
    async fn execute_deny_json_returns_block() {
        let h = ScopedHookHandler::new(
            "test-deny-json".into(),
            r#"echo '{"decision":"deny","reason":"policy violation"}'"#.into(),
            "*".into(),
            0,
        );
        let ctx = HookContext::new().with_tool("bash", serde_json::json!({}));
        let action = h.execute(&ctx).await.unwrap();
        assert!(
            matches!(action, HookAction::Block(ref r) if r == "policy violation"),
            "expected Block(policy violation), got {:?}",
            action
        );
    }

    #[tokio::test]
    async fn execute_no_tool_name_matches_empty_condition() {
        let h = ScopedHookHandler::new(
            "stop-hook".into(),
            r#"echo '{"decision":"allow"}'"#.into(),
            "".into(),
            0,
        );
        // No tool_name set on context (Stop hooks typically have none)
        let ctx = HookContext::new();
        let action = h.execute(&ctx).await.unwrap();
        assert!(
            matches!(action, HookAction::Continue),
            "expected Continue, got {:?}",
            action
        );
    }
}
