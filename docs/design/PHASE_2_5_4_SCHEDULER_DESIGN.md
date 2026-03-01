# Phase 2.5.4 Cron Scheduler 设计文档

**版本**: v1.0
**创建日期**: 2026-03-01
**目标**: 定时任务调度系统

---

## 一、概述与目标

**设计目标**：
- 定时触发 Agent 执行任务
- 支持 Cron 表达式（标准 5 字段）
- 任务持久化到 SQLite，重启后可恢复
- 支持用户级隔离（每个用户只能管理自己的任务）

**核心能力**：
| 能力 | 描述 |
|------|------|
| 定时执行 | 按 Cron 表达式触发 |
| 持久化 | 任务存储在数据库 |
| 手动触发 | 支持立即执行 |
| 状态追踪 | last_run, next_run, result |

---

## 二、架构设计

```
┌─────────────────────────────────────────────────────────────┐
│                      Scheduler                              │
│  ┌─────────────────────────────────────────────────────┐  │
│  │                 Cron Loop                            │  │
│  │  (每分钟检查是否有任务需要执行)                        │  │
│  └─────────────────────┬───────────────────────────────┘  │
│                        │                                    │
│                        ▼                                    │
│  ┌─────────────────────────────────────────────────────┐  │
│  │              Task Executor                           │  │
│  │  - 从 DB 加载任务                                    │  │
│  │  - 创建 Agent Run                                    │  │
│  │  - 记录执行结果                                      │  │
│  └─────────────────────┬───────────────────────────────┘  │
│                        │                                    │
│                        ▼                                    │
│  ┌─────────────────────────────────────────────────────┐  │
│  │              Persistence Layer                       │  │
│  │  - SQLite                                           │  │
│  │  - scheduled_tasks 表                                │  │
│  └─────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

---

## 三、核心数据结构

### 3.1 ScheduledTask

```rust
// crates/octo-engine/src/scheduler/mod.rs

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// 定时任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    /// 任务唯一标识
    pub id: String,

    /// 用户ID（可选，支持多用户隔离）
    pub user_id: Option<String>,

    /// 任务名称
    pub name: String,

    /// Cron 表达式（标准 5 字段：分 时 日 月 周）
    /// 例如: "0 9 * * *" = 每天 9:00
    pub cron: String,

    /// Agent 配置
    pub agent_config: AgentTaskConfig,

    /// 是否启用
    pub enabled: bool,

    /// 上次执行时间
    pub last_run: Option<DateTime<Utc>>,

    /// 下次执行时间
    pub next_run: Option<DateTime<Utc>>,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// Agent 任务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTaskConfig {
    /// System prompt
    pub system_prompt: String,

    /// 初始输入
    pub input: String,

    /// 最大执行轮数
    pub max_rounds: u32,

    /// 超时时间（秒）
    pub timeout_secs: u64,
}

impl Default for AgentTaskConfig {
    fn default() -> Self {
        Self {
            system_prompt: String::new(),
            input: String::new(),
            max_rounds: 50,
            timeout_secs: 300, // 5 分钟
        }
    }
}
```

### 3.2 TaskExecution（执行记录）

```rust
/// 任务执行记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecution {
    pub id: String,

    /// 关联的任务ID
    pub task_id: String,

    /// 执行开始时间
    pub started_at: DateTime<Utc>,

    /// 执行结束时间
    pub finished_at: Option<DateTime<Utc>>,

    /// 执行状态
    pub status: ExecutionStatus,

    /// 执行结果（成功时的输出）
    pub result: Option<String>,

    /// 错误信息（失败时）
    pub error: Option<String>,
}

/// 执行状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    /// 执行中
    Running,
    /// 执行成功
    Success,
    /// 执行失败
    Failed,
    /// 执行超时
    Timeout,
    /// 取消
    Cancelled,
}
```

### 3.3 Scheduler 核心

```rust
/// 调度器
pub struct Scheduler {
    /// 任务存储
    tasks: Arc<RwLock<HashMap<String, ScheduledTask>>>,

    /// 数据库持久化
    storage: Arc<SchedulerStorage>,

    /// Cron 解析器
    cron_parser: CronParser,

    /// Agent 工厂（用于创建任务执行）
    agent_factory: Arc<dyn AgentFactory>,

    /// 是否正在运行
    running: Arc<AtomicBool>,
}

/// Agent 工厂 trait（解耦）
#[async_trait]
pub trait AgentFactory: Send + Sync {
    /// 创建 Agent 执行
    async fn create_run(&self, config: &AgentTaskConfig) -> Result<String>; // 返回 run_id

    /// 获取执行状态
    async fn get_status(&self, run_id: &str) -> Result<ExecutionStatus>;

    /// 获取执行结果
    async fn get_result(&self, run_id: &str) -> Result<String>;
}
```

---

## 四、数据库设计

### 4.1 Migration v5 - scheduled_tasks

```rust
// crates/octo-engine/src/db/migrations.rs

/// Migration v5: Add scheduled_tasks table
pub fn migration_v5() -> Migration {
    Migration::new(
        5,
        "add_scheduled_tasks",
        r#"
        CREATE TABLE IF NOT EXISTS scheduled_tasks (
            -- Primary key
            id TEXT PRIMARY KEY,

            -- User isolation
            user_id TEXT,

            -- Task config
            name TEXT NOT NULL,
            cron TEXT NOT NULL,
            agent_config TEXT NOT NULL,  -- JSON serialized

            -- State
            enabled INTEGER NOT NULL DEFAULT 1,

            -- Timing
            last_run TEXT,  -- ISO8601 timestamp
            next_run TEXT,  -- ISO8601 timestamp

            -- Timestamps
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,

            -- Indexes
            INDEX idx_user_id (user_id),
            INDEX idx_next_run (next_run),
            INDEX idx_enabled (enabled)
        );

        CREATE TABLE IF NOT EXISTS task_executions (
            -- Primary key
            id TEXT PRIMARY KEY,

            -- Foreign key to task
            task_id TEXT NOT NULL,

            -- Timing
            started_at TEXT NOT NULL,
            finished_at TEXT,  -- ISO8601 timestamp

            -- Status
            status TEXT NOT NULL,

            -- Result
            result TEXT,
            error TEXT,

            -- Indexes
            INDEX idx_task_id (task_id),
            INDEX idx_started_at (started_at)
        );
        "#,
    )
}
```

### 4.2 Storage Trait

```rust
/// Scheduler storage trait
#[async_trait]
pub trait SchedulerStorage: Send + Sync {
    /// Save task
    async fn save_task(&self, task: &ScheduledTask) -> Result<()>;

    /// Get task by ID
    async fn get_task(&self, task_id: &str) -> Result<Option<ScheduledTask>>;

    /// List tasks (with optional user filter)
    async fn list_tasks(&self, user_id: Option<&str>) -> Result<Vec<ScheduledTask>>;

    /// Delete task
    async fn delete_task(&self, task_id: &str) -> Result<()>;

    /// Update task timing
    async fn update_timing(
        &self,
        task_id: &str,
        last_run: Option<DateTime<Utc>>,
        next_run: Option<DateTime<Utc>>,
    ) -> Result<()>;

    /// Save execution
    async fn save_execution(&self, execution: &TaskExecution) -> Result<()>;

    /// Get executions for task
    async fn get_executions(&self, task_id: &str, limit: usize) -> Result<Vec<TaskExecution>>;
}
```

---

## 五、REST API 设计

### 5.1 Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | /api/scheduler/tasks | 列出任务（用户隔离） |
| POST | /api/scheduler/tasks | 创建任务 |
| GET | /api/scheduler/tasks/:id | 获取任务详情 |
| PUT | /api/scheduler/tasks/:id | 更新任务 |
| DELETE | /api/scheduler/tasks/:id | 删除任务 |
| POST | /api/scheduler/tasks/:id/run | 手动触发执行 |
| GET | /api/scheduler/tasks/:id/executions | 获取执行历史 |

### 5.2 Request/Response Examples

**POST /api/scheduler/tasks** (创建任务)

```json
// Request
{
    "name": "Daily Report",
    "cron": "0 9 * * *",
    "agent_config": {
        "system_prompt": "你是一个数据分析助手",
        "input": "请生成昨日的报告",
        "max_rounds": 50,
        "timeout_secs": 300
    },
    "enabled": true
}

// Response
{
    "id": "task-xxx",
    "name": "Daily Report",
    "cron": "0 9 * * *",
    "enabled": true,
    "last_run": null,
    "next_run": "2026-03-02T09:00:00Z",
    "created_at": "2026-03-01T14:00:00Z",
    "updated_at": "2026-03-01T14:00:00Z"
}
```

**GET /api/scheduler/tasks** (列表)

```json
{
    "tasks": [
        {
            "id": "task-xxx",
            "name": "Daily Report",
            "cron": "0 9 * * *",
            "enabled": true,
            "next_run": "2026-03-02T09:00:00Z",
            "last_run": "2026-03-01T09:00:00Z"
        }
    ],
    "total": 1
}
```

---

## 六、执行流程

### 6.1 Cron Loop

```
Scheduler 启动
    │
    ▼
┌─────────────────────┐
│  每 60 秒唤醒一次    │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│  查询即将执行的任务   │
│  next_run <= now    │
│  AND enabled = true │
└──────────┬──────────┘
           │
           ▼
    ┌──────┴──────┐
    │ 任务列表     │
    └──────┬──────┘
           │
     ┌─────┴─────┐
     │ 遍历任务   │
     └─────┬─────┘
           │
           ▼
┌─────────────────────┐
│  检查是否已在运行    │
└──────────┬──────────┘
           │
     ┌─────┴─────┐
     │ 否         │──▶ 执行任务
     └─────┬─────┘
           │
          是
           │
           ▼
┌─────────────────────┐
│  跳过（防止重复执行） │
└─────────────────────┘
```

### 6.2 Task Execution

```rust
impl Scheduler {
    /// 执行定时任务
    async fn execute_task(&self, task: &ScheduledTask) -> Result<TaskExecution> {
        let execution_id = Uuid::new_v4().to_string();

        // 1. 创建执行记录
        let mut execution = TaskExecution {
            id: execution_id.clone(),
            task_id: task.id.clone(),
            started_at: Utc::now(),
            finished_at: None,
            status: ExecutionStatus::Running,
            result: None,
            error: None,
        };

        // 2. 执行 Agent（带超时）
        let result = tokio::time::timeout(
            Duration::from_secs(task.agent_config.timeout_secs),
            self.agent_factory.create_run(&task.agent_config),
        ).await;

        // 3. 记录结果
        match result {
            Ok(Ok(run_id)) => {
                execution.status = ExecutionStatus::Success;
                execution.result = Some(run_id);
            }
            Ok(Err(e)) => {
                execution.status = ExecutionStatus::Failed;
                execution.error = Some(e.to_string());
            }
            Err(_) => {
                execution.status = ExecutionStatus::Timeout;
                execution.error = Some("Execution timeout".to_string());
            }
        }

        execution.finished_at = Some(Utc::now());

        // 4. 更新任务 timing
        self.update_next_run(task).await?;

        // 5. 保存执行记录
        self.storage.save_execution(&execution).await?;

        Ok(execution)
    }

    /// 计算下次执行时间
    fn calculate_next_run(&self, cron: &str, from: DateTime<Utc>) -> Result<DateTime<Utc>> {
        // 使用 cron crate 解析
        let schedule = self.cron_parser.parse(cron)?;
        let next = schedule.next_after(from);
        Ok(next)
    }
}
```

---

## 七、配置与集成

### 7.1 Config

```rust
// crates/octo-server/src/config.rs

#[derive(Debug, Clone, Deserialize)]
pub struct SchedulerConfig {
    /// 是否启用调度器
    #[serde(default = "default_false")]
    pub enabled: bool,

    /// Cron 检查间隔（秒）
    #[serde(default = "60")]
    pub check_interval_secs: u64,

    /// 最大并发执行数
    #[serde(default = "5")]
    pub max_concurrent: usize,

    /// 预加载任务数
    #[serde(default = "100")]
    pub preload_limit: usize,
}

fn default_false() -> bool { false }
```

### 7.2 YAML Config

```yaml
# config.yaml
scheduler:
  enabled: true
  check_interval_secs: 60
  max_concurrent: 5

  # 可选：预定义任务（启动时加载）
  tasks:
    - id: "daily-report"
      name: "Daily Report"
      cron: "0 9 * * *"
      agent:
        system_prompt: "你是一个数据分析助手"
        input: "请生成昨日的报告"
        max_rounds: 50
        timeout_secs: 300
      enabled: true
```

### 7.3 Server Integration

```rust
// crates/octo-server/src/lib.rs

pub struct AppState {
    // ... existing fields
    pub scheduler: Arc<Scheduler>,
}

impl AppState {
    pub async fn new(config: Config) -> Result<Self> {
        // ... existing init

        // Init scheduler if enabled
        let scheduler = if config.scheduler.enabled {
            let storage = SchedulerStorageImpl::new(db.clone());
            let agent_factory = AgentFactoryImpl::new(/* ... */);

            Some(Scheduler::new(
                storage,
                agent_factory,
                config.scheduler.clone(),
            ))
        } else {
            None
        };

        Self {
            scheduler,
            // ...
        }
    }
}
```

### 7.4 Router Integration

```rust
// crates/octo-server/src/router.rs

fn create_router(state: AppState) -> Router {
    Router::new()
        // ... existing routes
        .nest("/api/scheduler", create_scheduler_router(state.clone()))
}

fn create_scheduler_router(state: AppState) -> Router {
    Router::new()
        .route("/tasks", get(list_tasks))
        .route("/tasks", post(create_task))
        .route("/tasks/:id", get(get_task))
        .route("/tasks/:id", put(update_task))
        .route("/tasks/:id", delete(delete_task))
        .route("/tasks/:id/run", post(run_task))
        .route("/tasks/:id/executions", get(list_executions))
        .with_state(state)
}
```

---

## 八、错误处理与边界情况

### 8.1 边界情况处理

| 场景 | 处理策略 |
|------|----------|
| 任务执行超时 | 标记 Timeout 状态，继续调度下一个任务 |
| Agent 执行失败 | 记录错误，继续下次调度 |
| 重启后任务恢复 | 从 DB 加载 enabled=true 的任务，重算 next_run |
| 并发执行限制 | 使用 Semaphore，超过则跳过本次 |
| Cron 表达式无效 | 创建时校验，返回错误 |
| 任务被删除时正在执行 | 标记 Cancelled，继续执行但忽略结果 |

### 8.2 错误处理

```rust
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

impl serde::Serialize for SchedulerError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
```

---

## 九、验收标准

### 9.1 功能验收

| 条件 | 验证方法 |
|------|----------|
| 创建定时任务 | POST /api/scheduler/tasks，返回任务详情 |
| Cron 表达式解析 | 创建任务时校验，无效返回 400 |
| 定时执行 | 等待 cron 触发，验证任务执行 |
| 手动触发 | POST /api/scheduler/tasks/:id/run |
| 执行记录 | GET /api/scheduler/tasks/:id/executions |
| 用户隔离 | 用户A创建的任务，用户B无法看到 |

### 9.2 测试场景

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cron_parsing() {
        let parser = CronParser::new();
        let schedule = parser.parse("0 9 * * *").unwrap();

        let now = Utc::now();
        let next = schedule.next_after(now);

        // 应该是当天 9:00（如果当前时间 < 9:00）
        // 应该是次日 9:00（如果当前时间 > 9:00）
        assert_eq!(next.hour(), 9);
        assert_eq!(next.minute(), 0);
    }

    #[test]
    fn test_next_run_calculation() {
        // "0 9 * * *" - 每天 9:00
        // 如果现在是 2026-03-01 10:00，下应该是 2026-03-02 09:00
    }

    #[tokio::test]
    async fn test_task_execution_timeout() {
        // 超时处理测试
    }
}
```

### 9.3 估算工作量

| 模块 | 估算 |
|------|------|
| 数据库 Migration | 50 LOC |
| Scheduler 核心 | 200 LOC |
| Storage 实现 | 100 LOC |
| REST API | 150 LOC |
| 配置集成 | 50 LOC |
| 测试 | 100 LOC |
| **总计** | **~650 LOC** |

---

## 十、依赖项

### 10.1 Rust Crates

| Crate | 用途 | Version |
|-------|------|---------|
| `cron` | Cron 表达式解析 | `0.15` |
| `cron_clock` | Cron schedule 计算 | `0.14` |
| `tokio::time` | 超时控制 | built-in |
| `uuid` | 生成 ID | `1.x` |

**选择**：使用 `cron` + `cron_clock` 组合，前者解析，后者计算下次执行时间。

### 10.2 数据库表

| 表名 | 用途 |
|------|------|
| `scheduled_tasks` | 定时任务存储 |
| `task_executions` | 执行记录 |

---

## 十一、与其他模块的关系

```
Scheduler
    │
    ├── 依赖 UserContext (用户隔离)
    │       └── Phase 2.5.3 已完成
    │
    ├── 依赖 Agent Factory (执行 Agent)
    │       └── 需要实现 trait
    │
    └── 依赖 DB (持久化)
            └── 新增 migration v5
```

---

## 十二、决策记录

| 编号 | 决策 | 内容 |
|------|------|------|
| D-01 | Cron 库 | 使用 cron + cron_clock 组合 |
| D-02 | 存储 | 复用现有 SQLite，不新增存储 |
| D-03 | 并发 | 使用 Semaphore 限制并发 |
| D-04 | 超时 | 使用 tokio::time::timeout |
