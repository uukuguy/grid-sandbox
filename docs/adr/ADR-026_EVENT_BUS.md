# ADR-026: EventBus Event Bus

## Status
Completed

## Context

System components need loose coupling communication mechanism.

## Decision

Implement EventBus based on broadcast channel:

```rust
pub struct EventBus {
    sender: broadcast::Sender<Event>,
}

pub struct Event {
    pub topic: String,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub source: String,
}
```

Supports:
- Multiple subscribers
- Event filtering
- Dead letter handling

## Consequences

### Positive
- Loose coupling between components
- Support for multiple subscribers
- Built-in event filtering

### Negative
- Additional complexity for event routing
- Potential for message loss if subscribers are slow

### Neutral
- Requires careful topic design

## References

- Code paths: `src/event/bus.rs`
