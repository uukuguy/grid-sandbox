# DDD Model: Hook System Context

**Project**: octo-sandbox
**Bounded Context**: Hook System
**Status**: Implemented

---

## Ubiquitous Language

| Term | Definition |
|------|------------|
| HookRegistry | Global hook registration and lookup |
| HookHandler | Event processing for hooks |
| HookContext | Context propagation between hooks |
| HookEvent | Events that trigger hooks |

---

## Aggregates

### HookRegistry (Aggregate Root)

```rust
pub struct HookRegistry {
    hooks: HashMap<HookEvent, Vec<Hook>>,
}
```

**Responsibilities**:
- Hook registration by event type
- Hook ordering and priority management

---

## Value Objects

### Hook

```rust
pub struct Hook {
    pub id: HookId,
    pub name: String,
    pub event: HookEvent,
    pub handler: Arc<dyn HookHandler>,
    pub priority: i32,
    pub enabled: bool,
}
```

### HookEvent

```rust
pub enum HookEvent {
    PreAgentRun,
    PostAgentRun,
    PreToolExecute,
    PostToolExecute,
    PrePromptBuild,
    PostPromptBuild,
    Error,
}
```

### HookContext

```rust
pub struct HookContext {
    pub session_id: Option<SessionId>,
    pub agent_id: Option<AgentId>,
    pub data: HashMap<String, Value>,
}
```

---

## Domain Services

### HookHandler

```rust
pub trait HookHandler {
    async fn handle(&self, context: &HookContext) -> Result<()>;
}
```

---

## Invariants

1. **Hook Ordering**: Hooks execute in priority order (higher priority first)
2. **Error Isolation**: Hook errors don't break main execution flow
3. **Context Propagation**: HookContext passed through chain

---

## Dependencies

- **Agent Context**: Hooks can intercept agent execution
- **Tool Context**: Hooks can intercept tool execution
- **Event Context**: Uses EventBus for event triggering

---

## References

- ADR-007: Hook Engine Introduction
- ADR-023: HookRegistry Global Hook Registration
- ADR-024: HookHandler Event Processing Mechanism
- ADR-025: HookContext Context Propagation
