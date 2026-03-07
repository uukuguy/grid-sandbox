# ADR-025: HookContext Context Propagation

## Status
Completed

## Context

Handlers need access to runtime context information.

## Decision

Implement `HookContext` carrying necessary information:

```rust
pub struct HookContext {
    pub hook_point: HookPoint,
    pub session_id: Option<SessionId>,
    pub sandbox_id: Option<SandboxId>,
    pub user_id: Option<UserId>,
    pub metadata: HashMap<String, String>,
}
```

## References

- Code paths: `src/hooks/context.rs
