# ADR-028: ProjectionEngine Projection Engine

## Status
Completed

## Context

Need to build read models from event streams to support different views.

## Decision

Implement ProjectionEngine:

```rust
pub struct ProjectionEngine {
    projections: HashMap<String, Box<dyn Projection>>,
    checkpoint: Checkpoint,
}
```

**Checkpoint Thread Safety**:
- Use Arc<RwLock<Checkpoint>>
- Periodic persistence to SQLite

## Consequences

### Positive
- Flexible read model generation
- Checkpoint enables resume from failure
- Support for multiple concurrent projections

### Negative
- Complexity in managing multiple projections
- Checkpoint consistency challenges under high load

### Neutral
- Requires careful projection design

## References

- Code paths: `src/event/projection.rs`
