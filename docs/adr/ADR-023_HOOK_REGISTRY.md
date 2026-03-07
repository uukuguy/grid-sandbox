# ADR-023: HookRegistry Global Hook Registration

## Status
Completed

## Context

System needs to support extensions at key points, allowing external logic injection.

## Decision

Implement `HookRegistry` managing 11 hook points:

| Hook Point | Trigger Timing | Purpose |
|-----------|----------------|---------|
| PreToolUse | Before tool call | Permission check, parameter validation |
| PostToolUse | After tool call | Result recording, cleanup |
| PreTask | Before task start | Initialization, preparation |
| PostTask | After task completion | Summary, cleanup |
| SessionStart | Session start | Load session state |
| SessionEnd | Session end | Save session state |
| ContextDegraded | Context degradation | Trigger memory extraction |
| LoopTurnStart | Agent loop start | Per-turn initialization |
| LoopTurnEnd | Agent loop end | Per-turn summary |
| AgentRoute | Agent routing decision | Custom routing |
| Notify | Notification event | Event subscription |

```rust
pub struct HookRegistry {
    hooks: HashMap<HookPoint, Vec<Arc<dyn HookHandler>>>,
}
```

## References

- Code paths: `src/hooks/registry.rs`, `src/hooks/mod.rs`
