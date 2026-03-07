# ADR-032: Scheduler System

## Status

Accepted

## Date

2026-03-07

## Context

The agent system requires scheduled task execution capabilities, supporting:
- Periodic agent execution
- Delayed task execution
- Cron expression configuration
- Persistent task state

## Decision

Implement a Cron-based task scheduler:

### Core Architecture

```rust
// Scheduler main structure
pub struct TaskScheduler {
    runtime: Runtime,
    jobs: Arc<RwLock<HashMap<JobId, ScheduledJob>>>,
    store: SqliteJobStore,
}

// Job definition
pub struct ScheduledJob {
    pub id: JobId,
    pub name: String,
    pub schedule: CronSchedule,
    pub task: TaskDefinition,
    pub enabled: bool,
    pub next_run: Option<DateTime<Utc>>,
}

// Task types
pub enum TaskDefinition {
    AgentExecution(AgentExecutionTask),
    Webhook(WebhookTask),
    Cleanup(CleanupTask),
}
```

### Storage Backend

- **SQLite**: Default persistent storage
- Support CRUD operations for tasks
- Auto-track next execution time

### Scheduling Strategy

1. **Precise Scheduling**: Use Cron parsing library to calculate exact execution times
2. **Persistence**: Store task state in database
3. **Fault Tolerance**: Auto skip or retry missed executions

## Consequences

### Positive

- Support complex Cron expressions
- Task persistence not lost
- Easy to manage and monitor

### Negative

- Single-node scheduling without high availability
- Cluster environment requires external scheduler

## Related

- [ADR-037: Session Management](ADR-037-SESSION_MANAGEMENT.md)
