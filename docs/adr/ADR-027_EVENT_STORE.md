# ADR-027: EventStore Event Persistence

## Status
Completed

## Context

Events need persistence for audit and replay support.

## Decision

Implement SQLite EventStore:

```rust
pub struct EventStore {
    conn: SqliteConnection,
}

impl EventStore {
    pub async fn append(&self, event: &Event) -> Result<EventId>;
    pub async fn get_events(&self, aggregate_id: &str) -> Result<Vec<Event>>;
}
```

## Consequences

### Positive
- Persistent event history for auditing
- Enables event replay for state reconstruction
- SQLite provides ACID guarantees

### Negative
- Storage overhead for event data
- Performance implications for large event streams

### Neutral
- Requires migration for schema changes

## References

- Code paths: `src/event/store.rs`
