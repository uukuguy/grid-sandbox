# ADR-029: StateReconstructor State Replay

## Status
Completed

## Context

Need to reconstruct state from event history.

## Decision

Implement StateReconstructor:

```rust
pub struct StateReconstructor {
    event_store: EventStore,
    aggregates: HashMap<String, Aggregate>,
}
```

**Event Limits**:
- Default limit of 1000 events
- Configurable `max_events` parameter

## Consequences

### Positive
- Enables state recovery from events
- Configurable limits prevent memory exhaustion
- Supports aggregate-based state organization

### Negative
- Performance degrades with large event histories
- Requires careful aggregate boundary design

### Neutral
- Trade-off between replay speed and completeness

## References

- Code paths: `src/event/reconstructor.rs`
