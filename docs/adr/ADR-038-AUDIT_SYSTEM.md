# ADR-038: Audit System

## Status

Accepted

## Date

2026-03-07

## Context

The system requires comprehensive audit logging for:
- Security compliance
- Access pattern analysis
- Incident investigation
- Regulatory requirements

## Decision

Implement audit system with structured logging:

### Core Components

```rust
// Audit event
pub struct AuditEvent {
    pub id: AuditEventId,
    pub timestamp: DateTime<Utc>,
    pub actor: Actor,
    pub action: Action,
    pub resource: Resource,
    pub outcome: Outcome,
    pub metadata: HashMap<String, String>,
}

// Audit storage
pub struct AuditStorage {
    db: SqlitePool,
    buffer: AsyncBuffer<AuditEvent>,
}
```

### Audit Categories

| Category | Description |
|----------|-------------|
| Authentication | Login, logout, token operations |
| Authorization | Permission checks, access denials |
| DataAccess | Read, write, delete operations |
| Configuration | System configuration changes |
| Security | Suspicious activities, breaches |

### Storage Strategy

- **SQLite**: Primary persistent storage
- **Async Buffer**: Batch writes for performance
- **Retention**: Configurable retention period

## Consequences

### Positive

- Complete audit trail
- Compliance support
- Security incident investigation

### Negative

- Storage growth
- Performance overhead for sync operations

## Related

- [ADR-031: Event System](ADR-031-EVENT_SYSTEM.md)
- [ADR-033: Secret Manager](ADR-033-SECRET_MANAGER.md)
