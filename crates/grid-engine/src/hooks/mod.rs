//! Hook Engine -- Lifecycle hook system for octo-engine
//!
//! Provides extensible hook points across the agent lifecycle.
//! Hooks can observe, modify, or abort operations.

pub mod builtin;
pub mod declarative;
pub mod policy;
pub mod wasm;
mod context;
mod handler;
mod registry;

pub use context::HookContext;
pub use handler::{
    BoxHookHandler, HookAction, HookFailureMode, HookHandler, PermissionHookDecision,
};
pub use registry::HookRegistry;

/// Hook points in the agent lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HookPoint {
    /// Before a tool is executed
    PreToolUse,
    /// After a tool completes
    PostToolUse,
    /// Before a task/turn starts
    PreTask,
    /// After a task/turn completes
    PostTask,
    /// Session starts
    SessionStart,
    /// Session ends
    SessionEnd,
    /// Before context compaction runs (audit-only — fires before the summarizer
    /// LLM is called; payload travels via `HookContext::metadata` per
    /// ADR-V2-018 §D1).
    PreCompact,
    /// After context compaction completes (renamed from `ContextDegraded` in
    /// ADR-V2-018 §D2 to reflect actual fire timing — emitted post-rebuild).
    PostCompact,
    /// Loop turn starts
    LoopTurnStart,
    /// Loop turn ends
    LoopTurnEnd,
    /// Agent is being routed
    AgentRoute,
    /// Skills activated for a query
    SkillsActivated,
    /// A skill was deactivated
    SkillDeactivated,
    /// A skill script started execution
    SkillScriptStarted,
    /// A tool constraint was violated
    ToolConstraintViolated,
    /// Agent is stopping (natural end or cancellation)
    Stop,
    /// Sub-agent execution completed
    SubagentStop,
    /// User prompt submitted (before first LLM call in a turn)
    UserPromptSubmit,
}
