# Phase 2.5.4 Cron Scheduler Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 实现定时任务调度系统，支持 Cron 表达式、任务持久化、用户隔离

**Architecture:**
- Scheduler 作为独立模块，包含 Cron 循环和任务执行器
- 使用 SQLite 存储任务和执行记录
- 通过 REST API 提供 CRUD 和手动触发功能
- 复用现有 UserContext 实现用户隔离

**Tech Stack:** Rust, tokio, cron, SQLite

---

## Task 1: Database Migration v5

**Files:**
- Modify: `crates/octo-engine/src/db/migrations.rs`
- Modify: `crates/octo-engine/src/db/mod.rs`

**Step 1: Add migration v5 function**

Add to `crates/octo-engine/src/db/migrations.rs`:

```rust
/// Migration v5: Add scheduled_tasks and task_executions tables
pub fn migration_v5() -> Migration {
    Migration::new(
        5,
        "add_scheduled_tasks",
        r#"
        CREATE TABLE IF NOT EXISTS scheduled_tasks (
            id TEXT PRIMARY KEY,
            user_id TEXT,
            name TEXT NOT NULL,
            cron TEXT NOT NULL,
            agent_config TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            last_run TEXT,
            next_run TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_scheduled_tasks_user_id ON scheduled_tasks(user_id);
        CREATE INDEX IF NOT EXISTS idx_scheduled_tasks_next_run ON scheduled_tasks(next_run);
        CREATE INDEX IF NOT EXISTS idx_scheduled_tasks_enabled ON scheduled_tasks(enabled);

        CREATE TABLE IF NOT EXISTS task_executions (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL,
            started_at TEXT NOT NULL,
            finished_at TEXT,
            status TEXT NOT NULL,
            result TEXT,
            error TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_task_executions_task_id ON task_executions(task_id);
        CREATE INDEX IF NOT EXISTS idx_task_executions_started_at ON task_executions(started_at);
        "#,
    )
}
```

**Step 2: Register migration in mod.rs**

Add to `crates/octo-engine/src/db/mod.rs`:

```rust
pub mod migrations;

pub fn get_migrations() -> Vec<Migration> {
    vec![
        migrations::migration_v1(),
        migrations::migration_v2(),
        migrations::migration_v3(),
        migrations::migration_v4(),
        migrations::migration_v5(), // Add this
    ]
}
```

**Step 3: Run cargo check to verify**

Run: `cargo check -p octo-engine`
Expected: No errors

**Step 4: Commit**

```bash
git add crates/octo-engine/src/db/migrations.rs crates/octo-engine/src/db/mod.rs
git commit -m "feat(scheduler): add migration v5 for scheduled_tasks"
```

---

## Task 2: Scheduler Module - Data Structures

**Files:**
- Create: `crates/octo-engine/src/scheduler/mod.rs`

**Step 1: Create scheduler module with data structures**

Create `crates/octo-engine/src/scheduler/mod.rs`:

```rust
//! Scheduler module for periodic task execution

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Scheduled task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub id: String,
    pub user_id: Option<String>,
    pub name: String,
    pub cron: String,
    pub agent_config: AgentTaskConfig,
    pub enabled: bool,
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Agent task configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTaskConfig {
    pub system_prompt: String,
    pub input: String,
    pub max_rounds: u32,
    pub timeout_secs: u64,
}

impl Default for AgentTaskConfig {
    fn default() -> Self {
        Self {
            system_prompt: String::new(),
            input: String::new(),
            max_rounds: 50,
            timeout_secs: 300,
        }
    }
}

/// Task execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecution {
    pub id: String,
    pub task_id: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub status: ExecutionStatus,
    pub result: Option<String>,
    pub error: Option<String>,
}

/// Execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Running,
    Success,
    Failed,
    Timeout,
    Cancelled,
}

/// Scheduler errors
#[derive(Debug, thiserror::Error)]
pub enum SchedulerError {
    #[error("Task not found: {0}")]
    TaskNotFound(String),
    #[error("Invalid cron expression: {0}")]
    InvalidCron(String),
    #[error("Task already running: {0}")]
    TaskAlreadyRunning(String),
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Configuration error: {0}")]
    Config(String),
}

impl Serialize for SchedulerError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
```

**Step 2: Run cargo check**

Run: `cargo check -p octo-engine`
Expected: No errors

**Step 3: Add scheduler module to engine lib.rs**

Modify `crates/octo-engine/src/lib.rs`:

```rust
pub mod scheduler;
```

**Step 4: Commit**

```bash
git add crates/octo-engine/src/scheduler/mod.rs crates/octo-engine/src/lib.rs
git commit -m "feat(scheduler): add scheduler module with data structures"
```

---

## Task 3: Scheduler Storage Trait

**Files:**
- Modify: `crates/octo-engine/src/scheduler/mod.rs`

**Step 1: Add storage trait**

Add to `crates/octo-engine/src/scheduler/mod.rs`:

```rust
use async_trait::async_trait;

/// Scheduler storage trait
#[async_trait]
pub trait SchedulerStorage: Send + Sync {
    async fn save_task(&self, task: &ScheduledTask) -> Result<(), SchedulerError>;
    async fn get_task(&self, task_id: &str) -> Result<Option<ScheduledTask>, SchedulerError>;
    async fn list_tasks(&self, user_id: Option<&str>) -> Result<Vec<ScheduledTask>, SchedulerError>;
    async fn delete_task(&self, task_id: &str) -> Result<(), SchedulerError>;
    async fn update_timing(
        &self,
        task_id: &str,
        last_run: Option<DateTime<Utc>>,
        next_run: Option<DateTime<Utc>>,
    ) -> Result<(), SchedulerError>;
    async fn save_execution(&self, execution: &TaskExecution) -> Result<(), SchedulerError>;
    async fn get_executions(&self, task_id: &str, limit: usize) -> Result<Vec<TaskExecution>, SchedulerError>;
    async fn get_due_tasks(&self) -> Result<Vec<ScheduledTask>, SchedulerError>;
}
```

**Step 2: Run cargo check**

Run: `cargo check -p octo-engine`
Expected: No errors (async_trait already in dependencies)

**Step 3: Commit**

```bash
git add crates/octo-engine/src/scheduler/mod.rs
git commit -m "feat(scheduler): add SchedulerStorage trait"
```

---

## Task 4: Scheduler Storage Implementation

**Files:**
- Create: `crates/octo-engine/src/scheduler/storage.rs`

**Step 1: Create storage implementation**

Create `crates/octo-engine/src/scheduler/storage.rs`:

```rust
//! SQLite-based scheduler storage implementation

use super::*;
use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;

/// SQLite-based scheduler storage
pub struct SqliteSchedulerStorage {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteSchedulerStorage {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }
}

#[async_trait]
impl SchedulerStorage for SqliteSchedulerStorage {
    async fn save_task(&self, task: &ScheduledTask) -> Result<(), SchedulerError> {
        let conn = self.conn.lock().await;
        let agent_config_json = serde_json::to_string(&task.agent_config)
            .map_err(|e| SchedulerError::Storage(e.to_string()))?;

        conn.execute(
            r#"INSERT OR REPLACE INTO scheduled_tasks
               (id, user_id, name, cron, agent_config, enabled, last_run, next_run, created_at, updated_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"#,
            (
                &task.id,
                &task.user_id,
                &task.name,
                &task.cron,
                agent_config_json,
                task.enabled as i32,
                task.last_run.map(|d| d.to_rfc3339()),
                task.next_run.map(|d| d.to_rfc3339()),
                task.created_at.to_rfc3339(),
                task.updated_at.to_rfc3339(),
            ),
        ).map_err(|e| SchedulerError::Storage(e.to_string()))?;

        Ok(())
    }

    async fn get_task(&self, task_id: &str) -> Result<Option<ScheduledTask>, SchedulerError> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, user_id, name, cron, agent_config, enabled, last_run, next_run, created_at, updated_at
             FROM scheduled_tasks WHERE id = ?1"
        ).map_err(|e| SchedulerError::Storage(e.to_string()))?;

        let task = stmt.query_row([task_id], |row| {
            Ok(row_to_task(row))
        }).ok();

        Ok(task)
    }

    async fn list_tasks(&self, user_id: Option<&str>) -> Result<Vec<ScheduledTask>, SchedulerError> {
        let conn = self.conn.lock().await;
        let query = match user_id {
            Some(_) => "SELECT id, user_id, name, cron, agent_config, enabled, last_run, next_run, created_at, updated_at FROM scheduled_tasks WHERE user_id = ?1",
            None => "SELECT id, user_id, name, cron, agent_config, enabled, last_run, next_run, created_at, updated_at FROM scheduled_tasks",
        };

        let mut stmt = conn.prepare(query).map_err(|e| SchedulerError::Storage(e.to_string()))?;

        let tasks = if let Some(uid) = user_id {
            stmt.query_map([uid], |row| Ok(row_to_task(row)))
        } else {
            stmt.query_map([], |row| Ok(row_to_task(row)))
        }.map_err(|e| SchedulerError::Storage(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

        Ok(tasks)
    }

    async fn delete_task(&self, task_id: &str) -> Result<(), SchedulerError> {
        let conn = self.conn.lock().await;
        conn.execute("DELETE FROM scheduled_tasks WHERE id = ?1", [task_id])
            .map_err(|e| SchedulerError::Storage(e.to_string()))?;
        Ok(())
    }

    async fn update_timing(
        &self,
        task_id: &str,
        last_run: Option<DateTime<Utc>>,
        next_run: Option<DateTime<Utc>>,
    ) -> Result<(), SchedulerError> {
        let conn = self.conn.lock().await;
        conn.execute(
            "UPDATE scheduled_tasks SET last_run = ?1, next_run = ?2, updated_at = ?3 WHERE id = ?4",
            (
                last_run.map(|d| d.to_rfc3339()),
                next_run.map(|d| d.to_rfc3339()),
                Utc::now().to_rfc3339(),
                task_id,
            ),
        ).map_err(|e| SchedulerError::Storage(e.to_string()))?;
        Ok(())
    }

    async fn save_execution(&self, execution: &TaskExecution) -> Result<(), SchedulerError> {
        let conn = self.conn.lock().await;
        conn.execute(
            r#"INSERT OR REPLACE INTO task_executions
               (id, task_id, started_at, finished_at, status, result, error)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"#,
            (
                &execution.id,
                &execution.task_id,
                execution.started_at.to_rfc3339(),
                execution.finished_at.map(|d| d.to_rfc3339()),
                serde_json::to_string(&execution.status).unwrap_or_default(),
                &execution.result,
                &execution.error,
            ),
        ).map_err(|e| SchedulerError::Storage(e.to_string()))?;
        Ok(())
    }

    async fn get_executions(&self, task_id: &str, limit: usize) -> Result<Vec<TaskExecution>, SchedulerError> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, task_id, started_at, finished_at, status, result, error
             FROM task_executions WHERE task_id = ?1 ORDER BY started_at DESC LIMIT ?2"
        ).map_err(|e| SchedulerError::Storage(e.to_string()))?;

        let executions = stmt.query_map([task_id, &limit.to_string()], |row| {
            Ok(row_to_execution(row))
        }).map_err(|e| SchedulerError::Storage(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

        Ok(executions)
    }

    async fn get_due_tasks(&self) -> Result<Vec<ScheduledTask>, SchedulerError> {
        let conn = self.conn.lock().await;
        let now = Utc::now().to_rfc3339();
        let mut stmt = conn.prepare(
            "SELECT id, user_id, name, cron, agent_config, enabled, last_run, next_run, created_at, updated_at
             FROM scheduled_tasks WHERE enabled = 1 AND next_run IS NOT NULL AND next_run <= ?1"
        ).map_err(|e| SchedulerError::Storage(e.to_string()))?;

        let tasks = stmt.query_map([&now], |row| {
            Ok(row_to_task(row))
        }).map_err(|e| SchedulerError::Storage(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

        Ok(tasks)
    }
}

fn row_to_task(row: &rusqlite::Row) -> ScheduledTask {
    let agent_config_json: String = row.get(4).unwrap_or_default();
    let agent_config: AgentTaskConfig = serde_json::from_str(&agent_config_json).unwrap_or_default();

    ScheduledTask {
        id: row.get(0).unwrap_or_default(),
        user_id: row.get(1).ok(),
        name: row.get(2).unwrap_or_default(),
        cron: row.get(3).unwrap_or_default(),
        agent_config,
        enabled: row.get::<_, i32>(5).unwrap_or(1) == 1,
        last_run: row.get::<_, Option<String>>(6).ok().flatten().map(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc))),
        next_run: row.get::<_, Option<String>>(7).ok().flatten().map(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc))),
        created_at: row.get::<_, String>(8).map(|s| DateTime::parse_from_rfc3339(&s).map(|d| d.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now())).unwrap_or_else(|_| Utc::now()),
        updated_at: row.get::<_, String>(9).map(|s| DateTime::parse_from_rfc3339(&s).map(|d| d.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now())).unwrap_or_else(|_| Utc::now()),
    }
}

fn row_to_execution(row: &rusqlite::Row) -> TaskExecution {
    let status_str: String = row.get(4).unwrap_or_default();
    let status: ExecutionStatus = serde_json::from_str(&status_str).unwrap_or(ExecutionStatus::Failed);

    TaskExecution {
        id: row.get(0).unwrap_or_default(),
        task_id: row.get(1).unwrap_or_default(),
        started_at: row.get::<_, String>(2).map(|s| DateTime::parse_from_rfc3339(&s).map(|d| d.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now())).unwrap_or_else(|_| Utc::now()),
        finished_at: row.get::<_, Option<String>>(3).ok().flatten().map(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc))),
        status,
        result: row.get(5).ok(),
        error: row.get(6).ok(),
    }
}
```

**Step 2: Add storage module to mod.rs**

Modify `crates/octo-engine/src/scheduler/mod.rs`:

```rust
pub mod storage;
pub use storage::SqliteSchedulerStorage;
```

**Step 3: Run cargo check**

Run: `cargo check -p octo-engine`
Expected: No errors

**Step 4: Commit**

```bash
git add crates/octo-engine/src/scheduler/storage.rs crates/octo-engine/src/scheduler/mod.rs
git commit -m "feat(scheduler): add SqliteSchedulerStorage implementation"
```

---

## Task 5: Scheduler Core - Cron Parser & Next Run Calculation

**Files:**
- Modify: `crates/octo-engine/src/scheduler/mod.rs`

**Step 1: Add cron parser helper**

Add to `crates/octo-engine/src/scheduler/mod.rs`:

```rust
use cron::Schedule;
use std::str::FromStr;

/// Cron parser helper
pub struct CronParser;

impl CronParser {
    pub fn new() -> Self {
        Self
    }

    /// Parse cron expression and calculate next run time
    pub fn parse_next(&self, cron_expr: &str, from: DateTime<Utc>) -> Result<DateTime<Utc>, SchedulerError> {
        // Cron expression uses standard 5-field format: minute hour day month weekday
        let schedule = Schedule::from_str(cron_expr)
            .map_err(|e| SchedulerError::InvalidCron(e.to_string()))?;

        let next = schedule
            .next_after(from)
            .with_timezone(&Utc);

        Ok(next)
    }

    /// Validate cron expression
    pub fn validate(&self, cron_expr: &str) -> Result<(), SchedulerError> {
        Schedule::from_str(cron_expr)
            .map_err(|e| SchedulerError::InvalidCron(e.to_string()))?;
        Ok(())
    }
}
```

**Step 2: Add dependency to Cargo.toml**

Modify `crates/octo-engine/Cargo.toml`:

```toml
[dependencies]
cron = "0.15"
```

**Step 3: Run cargo check**

Run: `cargo check -p octo-engine`
Expected: No errors

**Step 4: Commit**

```bash
git add crates/octo-engine/Cargo.toml crates/octo-engine/src/scheduler/mod.rs
git commit -m "feat(scheduler): add CronParser for cron expression parsing"
```

---

## Task 6: Scheduler Core - Scheduler Struct

**Files:**
- Modify: `crates/octo-engine/src/scheduler/mod.rs`

**Step 1: Add Scheduler struct**

Add to `crates/octo-engine/src/scheduler/mod.rs`:

```rust
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use uuid::Uuid;

/// Scheduler configuration
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    pub enabled: bool,
    pub check_interval_secs: u64,
    pub max_concurrent: usize,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            check_interval_secs: 60,
            max_concurrent: 5,
        }
    }
}

/// Scheduler core
pub struct Scheduler {
    config: SchedulerConfig,
    storage: Arc<dyn SchedulerStorage>,
    cron_parser: CronParser,
    running: Arc<AtomicBool>,
    semaphore: Arc<Semaphore>,
}

impl Scheduler {
    pub fn new(config: SchedulerConfig, storage: Arc<dyn SchedulerStorage>) -> Self {
        Self {
            config,
            storage,
            cron_parser: CronParser::new(),
            running: Arc::new(AtomicBool::new(false)),
            semaphore: Arc::new(Semaphore::new(5)),
        }
    }

    /// Start the scheduler loop
    pub async fn start(&self) {
        if !self.config.enabled {
            return;
        }

        self.running.store(true, Ordering::SeqCst);
        tracing::info!("Scheduler started with {}s interval", self.config.check_interval_secs);

        while self.running.load(Ordering::SeqCst) {
            self.tick().await;
            tokio::time::sleep(tokio::time::Duration::from_secs(self.config.check_interval_secs)).await;
        }
    }

    /// Stop the scheduler
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Execute one tick - check and run due tasks
    async fn tick(&self) {
        let tasks = match self.storage.get_due_tasks().await {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("Failed to get due tasks: {}", e);
                return;
            }
        };

        for task in tasks {
            if let Err(e) = self.execute_task(&task).await {
                tracing::error!("Task {} execution failed: {}", task.id, e);
            }
        }
    }

    /// Execute a single task
    async fn execute_task(&self, task: &ScheduledTask) -> Result<(), SchedulerError> {
        // Check concurrency limit
        let _permit = self.semaphore.acquire().await.map_err(|e| SchedulerError::ExecutionFailed(e.to_string()))?;

        let execution_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        // Create execution record
        let mut execution = TaskExecution {
            id: execution_id.clone(),
            task_id: task.id.clone(),
            started_at: now,
            finished_at: None,
            status: ExecutionStatus::Running,
            result: None,
            error: None,
        };

        // TODO: Execute agent (placeholder for now - integrate with agent later)
        // For now, simulate execution
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        execution.status = ExecutionStatus::Success;
        execution.result = Some("Task executed (placeholder)".to_string());
        execution.finished_at = Some(Utc::now());

        // Calculate next run
        let next_run = self.cron_parser.parse_next(&task.cron, Utc::now()).ok();

        // Update task timing
        self.storage.update_timing(&task.id, Some(now), next_run).await?;

        // Save execution
        self.storage.save_execution(&execution).await?;

        tracing::info!("Task {} executed, next run: {:?}", task.id, next_run);

        Ok(())
    }

    // === Public API ===

    /// Create a new task
    pub async fn create_task(
        &self,
        user_id: Option<String>,
        name: String,
        cron: String,
        agent_config: AgentTaskConfig,
        enabled: bool,
    ) -> Result<ScheduledTask, SchedulerError> {
        // Validate cron
        self.cron_parser.validate(&cron)?;

        let now = Utc::now();
        let next_run = self.cron_parser.parse_next(&cron, now)?;

        let task = ScheduledTask {
            id: Uuid::new_v4().to_string(),
            user_id,
            name,
            cron,
            agent_config,
            enabled,
            last_run: None,
            next_run: Some(next_run),
            created_at: now,
            updated_at: now,
        };

        self.storage.save_task(&task).await?;

        Ok(task)
    }

    /// List tasks
    pub async fn list_tasks(&self, user_id: Option<&str>) -> Result<Vec<ScheduledTask>, SchedulerError> {
        self.storage.list_tasks(user_id).await
    }

    /// Get task by ID
    pub async fn get_task(&self, task_id: &str) -> Result<Option<ScheduledTask>, SchedulerError> {
        self.storage.get_task(task_id).await
    }

    /// Delete task
    pub async fn delete_task(&self, task_id: &str) -> Result<(), SchedulerError> {
        self.storage.delete_task(task_id).await
    }

    /// Update task
    pub async fn update_task(
        &self,
        task_id: &str,
        name: Option<String>,
        cron: Option<String>,
        agent_config: Option<AgentTaskConfig>,
        enabled: Option<bool>,
    ) -> Result<ScheduledTask, SchedulerError> {
        let mut task = self.storage.get_task(task_id)
            .await?
            .ok_or_else(|| SchedulerError::TaskNotFound(task_id.to_string()))?;

        if let Some(n) = name {
            task.name = n;
        }
        if let Some(c) = cron {
            self.cron_parser.validate(&c)?;
            task.cron = c;
            task.next_run = self.cron_parser.parse_next(&task.cron, Utc::now()).ok();
        }
        if let Some(ac) = agent_config {
            task.agent_config = ac;
        }
        if let Some(e) = enabled {
            task.enabled = e;
            if e {
                task.next_run = self.cron_parser.parse_next(&task.cron, Utc::now()).ok();
            }
        }

        task.updated_at = Utc::now();
        self.storage.save_task(&task).await?;

        Ok(task)
    }

    /// Run task immediately (manual trigger)
    pub async fn run_now(&self, task_id: &str, user_id: Option<&str>) -> Result<TaskExecution, SchedulerError> {
        let task = self.storage.get_task(task_id).await?
            .ok_or_else(|| SchedulerError::TaskNotFound(task_id.to_string()))?;

        // Check user ownership
        if let (Some(req_user), Some(task_user)) = (user_id, &task.user_id) {
            if req_user != task_user {
                return Err(SchedulerError::TaskNotFound(task_id.to_string()));
            }
        }

        let now = Utc::now();
        let execution = TaskExecution {
            id: Uuid::new_v4().to_string(),
            task_id: task_id.to_string(),
            started_at: now,
            finished_at: Some(now),
            status: ExecutionStatus::Success,
            result: Some("Manually triggered".to_string()),
            error: None,
        };

        self.storage.save_execution(&execution).await?;

        // Update last_run
        self.storage.update_timing(task_id, Some(now), task.next_run).await?;

        Ok(execution)
    }

    /// Get task executions
    pub async fn get_executions(&self, task_id: &str, limit: usize) -> Result<Vec<TaskExecution>, SchedulerError> {
        self.storage.get_executions(task_id, limit).await
    }
}
```

**Step 2: Run cargo check**

Run: `cargo check -p octo-engine`
Expected: No errors

**Step 3: Commit**

```bash
git add crates/octo-engine/src/scheduler/mod.rs
git commit -m "feat(scheduler): add Scheduler core implementation"
```

---

## Task 7: Server Config - SchedulerConfig

**Files:**
- Modify: `crates/octo-server/src/config.rs`

**Step 1: Add SchedulerConfig**

Add to `crates/octo-server/src/config.rs`:

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct SchedulerConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default = "default_check_interval")]
    pub check_interval_secs: u64,

    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: usize,
}

fn default_check_interval() -> u64 { 60 }
fn default_max_concurrent() -> usize { 5 }

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            check_interval_secs: 60,
            max_concurrent: 5,
        }
    }
}
```

**Step 2: Add scheduler to Config**

Modify the `Config` struct in `crates/octo-server/src/config.rs`:

```rust
pub struct Config {
    // ... existing fields
    pub scheduler: SchedulerConfig,
}
```

**Step 3: Run cargo check**

Run: `cargo check -p octo-server`
Expected: No errors

**Step 4: Commit**

```bash
git add crates/octo-server/src/config.rs
git commit -m "feat(scheduler): add SchedulerConfig to server config"
```

---

## Task 8: REST API Handlers

**Files:**
- Create: `crates/octo-server/src/api/scheduler.rs`

**Step 1: Create scheduler API handlers**

Create `crates/octo-server/src/api/scheduler.rs`:

```rust
use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};

pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/tasks", get(list_tasks))
        .route("/tasks", post(create_task))
        .route("/tasks/:id", get(get_task))
        .route("/tasks/:id", put(update_task))
        .route("/tasks/:id", delete(delete_task))
        .route("/tasks/:id/run", post(run_task))
        .route("/tasks/:id/executions", get(list_executions))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskRequest {
    pub name: String,
    pub cron: String,
    pub agent_config: octo_engine::scheduler::AgentTaskConfig,
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskRequest {
    pub name: Option<String>,
    pub cron: Option<String>,
    pub agent_config: Option<octo_engine::scheduler::AgentTaskConfig>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskResponse {
    pub id: String,
    pub user_id: Option<String>,
    pub name: String,
    pub cron: String,
    pub agent_config: octo_engine::scheduler::AgentTaskConfig,
    pub enabled: bool,
    pub last_run: Option<String>,
    pub next_run: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<octo_engine::scheduler::ScheduledTask> for TaskResponse {
    fn from(t: octo_engine::scheduler::ScheduledTask) -> Self {
        Self {
            id: t.id,
            user_id: t.user_id,
            name: t.name,
            cron: t.cron,
            agent_config: t.agent_config,
            enabled: t.enabled,
            last_run: t.last_run.map(|d| d.to_rfc3339()),
            next_run: t.next_run.map(|d| d.to_rfc3339()),
            created_at: t.created_at.to_rfc3339(),
            updated_at: t.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TaskListResponse {
    pub tasks: Vec<TaskResponse>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionResponse {
    pub id: String,
    pub task_id: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub status: String,
    pub result: Option<String>,
    pub error: Option<String>,
}

impl From<octo_engine::scheduler::TaskExecution> for ExecutionResponse {
    fn from(e: octo_engine::scheduler::TaskExecution) -> Self {
        Self {
            id: e.id,
            task_id: e.task_id,
            started_at: e.started_at.to_rfc3339(),
            finished_at: e.finished_at.map(|d| d.to_rfc3339()),
            status: serde_json::to_string(&e.status).unwrap_or_default().trim_matches('"').to_string(),
            result: e.result,
            error: e.error,
        }
    }
}

async fn list_tasks(
    State(state): State<AppState>,
    Extension(user_ctx): Extension<octo_engine::auth::UserContext>,
) -> Result<Json<TaskListResponse>, StatusCode> {
    let user_id = user_ctx.user_id.as_deref();
    let tasks = state.scheduler
        .as_ref()
        .ok_or(StatusCode::NOT_FOUND)?
        .list_tasks(user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let total = tasks.len();
    Ok(Json(TaskListResponse {
        tasks: tasks.into_iter().map(|t| t.into()).collect(),
        total,
    }))
}

async fn create_task(
    State(state): State<AppState>,
    Extension(user_ctx): Extension<octo_engine::auth::UserContext>,
    Json(payload): Json<CreateTaskRequest>,
) -> Result<Json<TaskResponse>, StatusCode> {
    let scheduler = state.scheduler.as_ref().ok_or(StatusCode::NOT_FOUND)?;

    let task = scheduler
        .create_task(
            user_ctx.user_id,
            payload.name,
            payload.cron,
            payload.agent_config,
            payload.enabled.unwrap_or(true),
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to create task: {}", e);
            StatusCode::BAD_REQUEST
        })?;

    Ok(Json(task.into()))
}

async fn get_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
    Extension(user_ctx): Extension<octo_engine::auth::UserContext>,
) -> Result<Json<TaskResponse>, StatusCode> {
    let scheduler = state.scheduler.as_ref().ok_or(StatusCode::NOT_FOUND)?;

    let task = scheduler
        .get_task(&task_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Check ownership
    if let (Some(req_user), Some(task_user)) = (&user_ctx.user_id, &task.user_id) {
        if req_user != task_user {
            return Err(StatusCode::NOT_FOUND);
        }
    }

    Ok(Json(task.into()))
}

async fn update_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
    Extension(user_ctx): Extension<octo_engine::auth::UserContext>,
    Json(payload): Json<UpdateTaskRequest>,
) -> Result<Json<TaskResponse>, StatusCode> {
    let scheduler = state.scheduler.as_ref().ok_or(StatusCode::NOT_FOUND)?;

    let task = scheduler
        .get_task(&task_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Check ownership
    if let (Some(req_user), Some(task_user)) = (&user_ctx.user_id, &task.user_id) {
        if req_user != task_user {
            return Err(StatusCode::NOT_FOUND);
        }
    }

    let updated = scheduler
        .update_task(&task_id, payload.name, payload.cron, payload.agent_config, payload.enabled)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update task: {}", e);
            StatusCode::BAD_REQUEST
        })?;

    Ok(Json(updated.into()))
}

async fn delete_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
    Extension(user_ctx): Extension<octo_engine::auth::UserContext>,
) -> Result<StatusCode, StatusCode> {
    let scheduler = state.scheduler.as_ref().ok_or(StatusCode::NOT_FOUND)?;

    let task = scheduler
        .get_task(&task_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Check ownership
    if let (Some(req_user), Some(task_user)) = (&user_ctx.user_id, &task.user_id) {
        if req_user != task_user {
            return Err(StatusCode::NOT_FOUND);
        }
    }

    scheduler
        .delete_task(&task_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

async fn run_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
    Extension(user_ctx): Extension<octo_engine::auth::UserContext>,
) -> Result<Json<ExecutionResponse>, StatusCode> {
    let scheduler = state.scheduler.as_ref().ok_or(StatusCode::NOT_FOUND)?;

    let execution = scheduler
        .run_now(&task_id, user_ctx.user_id.as_deref())
        .await
        .map_err(|e| {
            tracing::error!("Failed to run task: {}", e);
            StatusCode::BAD_REQUEST
        })?;

    Ok(Json(execution.into()))
}

async fn list_executions(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
    Query(params): Query<ExecutionQuery>,
    Extension(user_ctx): Extension<octo_engine::auth::UserContext>,
) -> Result<Json<Vec<ExecutionResponse>>, StatusCode> {
    let scheduler = state.scheduler.as_ref().ok_or(StatusCode::NOT_FOUND)?;

    let task = scheduler
        .get_task(&task_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Check ownership
    if let (Some(req_user), Some(task_user)) = (&user_ctx.user_id, &task.user_id) {
        if req_user != task_user {
            return Err(StatusCode::NOT_FOUND);
        }
    }

    let executions = scheduler
        .get_executions(&task_id, params.limit.unwrap_or(10))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(executions.into_iter().map(|e| e.into()).collect()))
}

#[derive(Debug, Deserialize)]
pub struct ExecutionQuery {
    pub limit: Option<usize>,
}

use axum::{
    extract::Extension,
    http::StatusCode,
};
```

**Step 2: Add to router**

Modify `crates/octo-server/src/router.rs`:

```rust
pub mod scheduler;
```

And add to router creation:

```rust
Router::new()
    // ... existing routes
    .nest("/api/scheduler", scheduler::create_router())
```

**Step 3: Run cargo check**

Run: `cargo check -p octo-server`
Expected: No errors

**Step 4: Commit**

```bash
git add crates/octo-server/src/api/scheduler.rs crates/octo-server/src/router.rs
git commit -m "feat(scheduler): add REST API handlers"
```

---

## Task 8: Integration - Wire Scheduler in AppState

**Files:**
- Modify: `crates/octo-server/src/state.rs`

**Step 1: Add scheduler to AppState**

Modify `crates/octo-server/src/state.rs`:

```rust
use octo_engine::scheduler::{Scheduler, SchedulerConfig, SqliteSchedulerStorage};

pub struct AppState {
    // ... existing fields
    pub scheduler: Option<Arc<Scheduler>>,
}
```

**Step 2: Initialize scheduler in state**

Modify where AppState is created (likely in `lib.rs` or `main.rs`):

```rust
let scheduler = if config.scheduler.enabled {
    let storage = SqliteSchedulerStorage::new(db.clone());
    Some(Arc::new(Scheduler::new(
        config.scheduler.clone(),
        storage,
    )))
} else {
    None
};

AppState {
    scheduler,
    // ... other fields
}
```

**Step 3: Run cargo check**

Run: `cargo check -p octo-server`
Expected: No errors

**Step 4: Commit**

```bash
git add crates/octo-server/src/state.rs
git commit -m "feat(scheduler): integrate scheduler in AppState"
```

---

## Task 9: Add Tests

**Files:**
- Create: `crates/octo-engine/src/scheduler/tests.rs`

**Step 1: Add scheduler tests**

Create `crates/octo-engine/src/scheduler/tests.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cron_parser_validate() {
        let parser = CronParser::new();

        // Valid expressions
        assert!(parser.validate("0 9 * * *").is_ok());     // Daily at 9am
        assert!(parser.validate("*/5 * * * *").is_ok());   // Every 5 minutes
        assert!(parser.validate("0 0 1 * *").is_ok());    // Monthly

        // Invalid expressions
        assert!(parser.validate("invalid").is_err());
        assert!(parser.validate("60 * * * *").is_err());   // Invalid minute
    }

    #[test]
    fn test_cron_parser_next_run() {
        let parser = CronParser::new();
        let now = Utc::now();

        // Every minute
        let next = parser.parse_next("* * * * *", now).unwrap();
        assert!(next > now);

        // Daily at 9am
        let next = parser.parse_next("0 9 * * *", now).unwrap();
        assert_eq!(next.hour(), 9);
        assert_eq!(next.minute(), 0);
    }
}
```

**Step 2: Run tests**

Run: `cargo test -p octo-engine scheduler`
Expected: Tests pass

**Step 3: Commit**

```bash
git add crates/octo-engine/src/scheduler/tests.rs
git commit -m "test(scheduler): add scheduler tests"
```

---

## Task 10: Build & Verify

**Step 1: Full build**

Run: `cargo check --workspace`
Expected: No errors

**Step 2: Run all tests**

Run: `cargo test -p octo-engine`
Expected: All tests pass

**Step 3: Commit**

```bash
git add -A
git commit -m "feat(scheduler): complete Phase 2.5.4 - Cron Scheduler"
```

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | DB Migration v5 | 2 files |
| 2 | Data Structures | 2 files |
| 3 | Storage Trait | 1 file |
| 4 | Storage Impl | 2 files |
| 5 | Cron Parser | 2 files |
| 6 | Scheduler Core | 1 file |
| 7 | Config | 1 file |
| 8 | REST API | 2 files |
| 9 | Integration | 1 file |
| 10 | Tests & Verify | 1 file |
