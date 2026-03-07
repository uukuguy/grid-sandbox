# ADR-015: AgentRouter Routing Decision

## Status
Completed

## Context

System needs to support multiple types of Agents (coder, reviewer, tester, etc.), automatically select appropriate Agent based on task characteristics.

## Decision

Implement `AgentRouter` providing routing capability based on task description:

```rust
pub trait AgentRouter: Send + Sync {
    fn route(&self, task: &str) -> Result<RouteDecision>;
}
```

Routing decision contains:
- `agent_type`: Recommended Agent type
- `confidence`: Confidence (0.0-1.0)
- `fallback`: Backup Agent type

## References

- Code paths: `src/agent/router.rs`
