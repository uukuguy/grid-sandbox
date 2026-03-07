# ADR-030: Hooks System

## Status

Accepted

## Date

2026-03-07

## Context

octo-sandbox requires an extensible hook system that allows inserting custom logic at critical points in the agent execution lifecycle. This is essential for:
- Workflow automation triggers
- Auditing and monitoring
- Custom business logic injection
- External system integration

## Decision

Implement a registry-based hooks system supporting the following core capabilities:

### Architecture Design

```rust
// Core type definitions
pub trait HookHandler: Send + Sync {
    fn handle(&self, context: &HookContext) -> Result<HookResult>;
}

pub struct HookRegistry {
    handlers: Arc<RwLock<HashMap<String, Vec<Box<dyn HookHandler>>>>>,
}

pub enum HookTrigger {
    PreAgentExecution,
    PostAgentExecution,
    PreToolExecution,
    PostToolExecution,
    PreProviderCall,
    PostProviderCall,
    SessionStart,
    SessionEnd,
}
```

### Key Features

1. **Multi-trigger Support**: Triggers at agent execution, tool execution, provider calls
2. **Context Propagation**: HookContext carries execution context information
3. **Async Processing**: Support for async hook handling
4. **Priority Mechanism**: Support for priority-based execution ordering
5. **Error Handling Strategies**: Fail-stop, Continue, Fallback three strategies

### Alternatives Considered

- **Option A (Chosen)**: In-memory registry + runtime registration - Flexible but requires lifecycle management
- **Option B**: Static compile-time hooks - Performant but inflexible
- **Option C**: External message queue - Suitable for distributed but adds complexity

## Consequences

### Positive

- Decouples core logic from extension features
- Supports runtime dynamic registration
- Unified hook interface easy to use
- Supports sync/async processing

### Negative

- Introduces additional execution overhead
- Requires hook lifecycle management
- Error handling complexity increases

### Limitations

- Currently does not support hook chain termination signal propagation
- Async hook timeout handling needs improvement

## Related

- [ADR-023: Hook Registry](ADR-023_HOOK_REGISTRY.md)
- [ADR-024: Hook Handler](ADR-024_HOOK_HANDLER.md)
- [ADR-025: Hook Context](ADR-025_HOOK_CONTEXT.md)
