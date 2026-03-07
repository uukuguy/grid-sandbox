# ADR-007: Hook Engine Introduction

## Status
Proposed

## Context

RuView practice shows that 8 lifecycle Hook points (PreToolUse, PostToolUse, UserPromptSubmit, SessionStart, SessionEnd, Stop, PreCompact, SubagentStart) are the foundation data collection. Current for multi-agent collaboration octo-engine's Extension trait only has 3 Hook points (on_agent_start/end/tool_call), and requires writing Rust code, cannot be driven by configuration.

## Decision

Add new `hooks/` module in `octo-engine`:

```rust
pub enum HookPoint {
    PreToolUse,      // Before tool call (security validation, parameter enhancement)
    PostToolUse,     // After tool call (result recording, pattern learning)
    PreTask,         // Before Agent task starts (context preparation, constraint injection)
    PostTask,        // After Agent task completes (reward calculation, pattern storage)
    SessionStart,    // Session start (state recovery, memory loading)
    SessionEnd,      // Session end (state persistence, memory sync)
    ContextDegraded, // Context budget insufficient (save critical information)
    LoopTurnStart,   // Conversation turn start
    LoopTurnEnd,     // Conversation turn end
}

#[async_trait]
pub trait HookHandler: Send + Sync {
    fn name(&self) -> &str;
    fn matches(&self, point: HookPoint, context: &HookContext) -> bool;
    async fn execute(&self, context: &mut HookContext) -> Result<HookAction>;
}

pub struct HookRegistry {
    handlers: HashMap<HookPoint, Vec<Arc<dyn HookHandler>>>,
}
```

**Integration points**:
- `AgentLoop::run()` inserts Hook calls between Zone A/B/C
- `ToolRegistry::execute()` triggers Hook before/after tool calls
- `SessionStore` triggers Hook at session start/end

## Consequences

- Extension trait preserved (backward compatible), but recommended migration to HookRegistry
- Workbench can register few hooks (e.g., audit logging)
- Platform can register full Hook chain (routing, learning, constraint injection)

## References

- Related: ADR-006 (Three-Tier Architecture) — Hook engine belongs to engine layer
- RuView `.claude/settings.json` hooks configuration — design reference
