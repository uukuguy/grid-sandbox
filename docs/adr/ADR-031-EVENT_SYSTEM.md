# ADR-031: Event System

## Status

Accepted

## Date

2026-03-07

## Context

The agent system requires an event-driven architecture for loose-coupled communication between components, supporting:
- Real-time state change notifications
- Event sourcing
- Cross-component async communication
- Observability and debugging

## Decision

Implement a publish-subscribe based event system:

### Core Components

```rust
// Event Bus - Publish-Subscribe
pub struct EventBus {
    sender: broadcast::Sender<SystemEvent>,
    subscriptions: Arc<RwLock<HashMap<String, broadcast::Receiver<SystemEvent>>>>,
}

// Event Structure
pub struct SystemEvent {
    pub event_type: EventType,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub source: ModuleSource,
    pub correlation_id: Option<String>,
}

// Event Sourcing Store
pub struct EventStore {
    db: SqlitePool,
}
```

### Event Types

| Event Type | Description | Payload |
|------------|-------------|---------|
| AgentCreated | Agent creation | AgentId, Config |
| AgentStateChanged | Agent state change | AgentId, OldState, NewState |
| ToolExecuted | Tool execution | ToolName, Input, Output, Duration |
| SessionCreated | Session creation | SessionId, AgentId |
| MemoryIndexed | Memory indexing | MemoryId, Vector |

### Event Sourcing Support

- **EventStore**: Persist events to SQLite
- **ProjectionEngine**: Materialized view construction
- **StateReconstructor**: Reconstruct state from event replay

## Consequences

### Positive

- Complete decoupling between components
- Support multiple subscribers
- Built-in event filtering
- Support event replay and state reconstruction

### Negative

- Event routing adds complexity
- Slow subscribers may cause message backlog
- Requires careful topic design

## References

- Code paths: `crates/octo-engine/src/event/`
