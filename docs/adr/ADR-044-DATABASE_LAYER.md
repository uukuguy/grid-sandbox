# ADR-044: Database Layer

## Status

Accepted

## Date

2026-03-07

## Context

The system requires database persistence for:
- Session storage
- Agent state
- Event logs
- Configuration

## Decision

Implement SQLite database layer:

### Core Components

```rust
// Database wrapper
pub struct Database {
    pool: SqlitePool,
    migrations: MigrationRunner,
}

// Migration runner
pub struct MigrationRunner {
    migrations: Vec<Migration>,
}

// Connection pool
pub struct SqlitePool {
    connections: Vec<Mutex<SqliteConnection>>,
}
```

### Schema Overview

| Table | Description |
|-------|-------------|
| sessions | Session metadata |
| agents | Agent configurations |
| events | Event store |
| audit | Audit logs |
| tools_executions | Tool execution records |

### Migrations

- **Versioned**: Each migration has version number
- **Idempotent**: Safe to run multiple times
- **Rollback**: Support migration rollback

### Async Support

- **tokio-rusqlite**: Async database operations
- **Connection Pool**: Reuse connections
- **Prepared Statements**: Performance optimization

## Consequences

### Positive

- Simple deployment
- ACID compliance
- Single file storage

### Negative

- Limited scalability
- Single-node only

## References

- Code path: `crates/octo-engine/src/db/`
