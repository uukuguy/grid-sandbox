# ADR-024: HookHandler Event Processing Mechanism

## Status
Completed

## Context

Each hook point needs to support multiple Handlers, executed by priority.

## Decision

Implement `HookHandler` trait:

```rust
#[async_trait]
pub trait HookHandler: Send + Sync {
    async fn handle(&self, ctx: &HookContext) -> Result<HookResult>;
}
```

Handler types:
- **Block**: Stop operation, return error
- **Transform**: Modify input/output
- **Observe**: Observe only, no impact on flow

## References

- Code paths: `src/hooks/handler.rs`
