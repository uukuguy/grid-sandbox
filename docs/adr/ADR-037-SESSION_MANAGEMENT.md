# ADR-037: Session Management

## Status

Accepted

## Date

2026-03-07

## Context

The system requires session management for:
- Conversation state persistence
- Multi-session support per agent
- Session metadata tracking
- Session lifecycle management

## Decision

Implement session store with SQLite persistence:

### Core Architecture

```rust
// Session store
pub struct SessionStore {
    db: SqlitePool,
    cache: SessionCache,
}

// Session model
pub struct Session {
    pub id: SessionId,
    pub agent_id: AgentId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: SessionStatus,
    pub metadata: SessionMetadata,
}

pub enum SessionStatus {
    Active,
    Paused,
    Completed,
    Failed,
}
```

### Session Operations

| Operation | Description |
|-----------|-------------|
| create | Create new session |
| resume | Resume paused session |
| pause | Pause active session |
| complete | Mark session as completed |
| delete | Delete session and associated data |

### Storage Backends

- **SQLite**: Persistent storage for production
- **InMemory**: Ephemeral for testing

## Consequences

### Positive

- Persistent conversation state
- Support for session resumption
- Session metadata for analytics

### Negative

- Database storage growth
- Cleanup strategy required

## Related

- [ADR-044: Database Layer](ADR-044-DATABASE_LAYER.md)
