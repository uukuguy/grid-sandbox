# ADR-034: Observability

## Status

Accepted

## Date

2026-03-07

## Context

Production-grade agent system requires complete observability capabilities:
- Performance metrics monitoring
- Request rate and error rate tracking
- Resource usage monitoring
- Distributed tracing

## Decision

Implement Metrics system and Logging system:

### Metrics System Architecture

```rust
// Metrics registry
pub struct MetricsRegistry {
    counters: Arc<RwLock<HashMap<String, Counter>>>,
    gauges: Arc<RwLock<HashMap<String, Gauge>>>,
    histograms: Arc<RwLock<HashMap<String, Histogram>>>,
}

// Metric types
pub struct Counter { pub name: String, pub labels: HashMap<String, String> }
pub struct Gauge { pub name: String, pub labels: HashMap<String, String> }
pub struct Histogram {
    pub name: String,
    pub labels: HashMap<String, String>,
    pub buckets: Vec<f64>,
}
```

### Core Metrics

| Metric Type | Name | Description |
|-------------|------|-------------|
| Counter | agent_executions_total | Total agent executions |
| Counter | tool_calls_total | Total tool calls |
| Gauge | active_sessions | Current active sessions |
| Histogram | request_duration_ms | Request latency distribution |
| Histogram | token_usage_total | Token usage distribution |

### Logging System

```rust
// Logging configuration
pub struct LoggingConfig {
    pub level: LogLevel,
    pub format: LogFormat,
    pub output: LogOutput,
}

pub enum LogFormat {
    Pretty,  // Development
    Json,    // Production
}

pub enum LogOutput {
    Stdout,
    File(String),
    Custom(Box<dyn LogSink>),
}
```

### Log Levels

- **ERROR**: Error messages
- **WARN**: Warning messages
- **INFO**: General information
- **DEBUG**: Debug information

## Consequences

### Positive

- Unified metrics interface
- Multiple output format support
- Low-overhead metrics collection

### Negative

- Risk of metric dimension explosion
- Log volume needs management

## Related

- [ADR-040: Logging System](ADR-040-LOGGING_SYSTEM.md)
