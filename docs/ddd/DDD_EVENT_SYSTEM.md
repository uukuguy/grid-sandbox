# DDD Model: Event System Context

**Project**: octo-sandbox
**Bounded Context**: Event System
**Status**: Implemented

---

## Ubiquitous Language

| Term | Definition |
|------|------------|
| EventBus | Pub/sub event distribution |
| EventStore | SQLite persistence for events |
| ProjectionEngine | Builds read models from events |
| StateReconstructor | Reconstructs state from event history |

---

## Aggregates

### EventBus (Aggregate Root)

```rust
pub struct EventBus {
    sender: broadcast::Sender<Event>,
}
```

**Responsibilities**:
- Event distribution to subscribers
- Multi-subscriber support
- Dead letter handling

---

### EventStore (Entity)

```rust
pub struct EventStore {
    conn: SqliteConnection,
}
```

**Responsibilities**:
- Event persistence
- Event retrieval by aggregate_id
- Sequence number management

---

## Value Objects

### Event

```rust
pub struct Event {
    pub id: EventId,
    pub topic: String,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub aggregate_id: String,
    pub sequence: u64,
}
```

---

## Domain Services

### ProjectionEngine

```rust
pub struct ProjectionEngine {
    projections: HashMap<String, Box<dyn Projection>>,
    checkpoint: Arc<RwLock<Checkpoint>>,
}
```

### StateReconstructor

```rust
pub struct StateReconstructor {
    event_store: EventStore,
    aggregates: HashMap<String, Aggregate>,
}
```

---

## Domain Events

| Event | Payload |
|-------|---------|
| AgentCreated | agent_id, manifest |
| ToolExecuted | tool_name, params, result |
| MemoryStored | entry_id, memory_type |
| SessionStarted | session_id |
| SessionEnded | session_id |

---

## Invariants

1. **Event Ordering**: Events for same aggregate must be ordered by sequence
2. **Checkpoint Safety**: Projection checkpoint uses RwLock for thread safety
3. **Event Limit**: StateReconstructor limits replay to 1000 events by default

---

## References

- ADR-026: EventBus Event Bus
- ADR-027: EventStore Event Persistence
- ADR-028: ProjectionEngine Projection Engine
- ADR-029: StateReconstructor State Replay
