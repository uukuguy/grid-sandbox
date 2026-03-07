# ADR-022: ContextInjector Zone B Dynamic Context

## Status
Completed

## Context

LLM message construction needs dynamic context injection, including current time and memory blocks.

## Decision

Implement `ContextInjector` compiling Zone B dynamic context:

```rust
pub struct ContextInjector;

impl ContextInjector {
    pub fn compile(blocks: &[MemoryBlock]) -> String {
        // Output <context> XML block
    }
}
```

Output format:
```xml
<context>
<datetime>2026-03-07 14:30 CST</datetime>
<user_profile priority="128">User Profile content</user_profile>
<task_context priority="200">Task Context content</task_context>
</context>
```

## References

- Code paths: `src/memory/injector.rs`
