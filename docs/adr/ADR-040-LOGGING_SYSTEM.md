# ADR-040: Logging System

## Status

Accepted

## Date

2026-03-07

## Context

The system requires structured logging for:
- Debugging and troubleshooting
- Production monitoring
- Log aggregation
- Compliance requirements

## Decision

Implement structured logging system:

### Core Components

```rust
// Logger configuration
pub struct Logger {
    level: LogLevel,
    format: LogFormat,
    output: LogSink,
    subscriber: Option<TracingSubscriber>,
}

pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

pub enum LogFormat {
    Pretty,
    Json,
    Compact,
}
```

### Output Sinks

| Sink | Description | Use Case |
|------|-------------|----------|
| Stdout | Console output | Development |
| File | Rotating file | Production |
| Custom | External system | Integration |

### Structured Logging

```rust
// Example usage
logger.info("agent_execution {
    agent_id = %s,
    duration_ms = %d,
    status = %s
}", agent_id, duration, status);
```

## Consequences

### Positive

- Structured format for parsing
- Multiple output options
- Performance optimized

### Negative

- JSON verbosity in development
- Storage management needed

## Related

- [ADR-034: Observability](ADR-034-OBSERVABILITY.md)
