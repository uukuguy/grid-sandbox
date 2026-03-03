# AgentRuntime 重构设计文档

> **For Claude:** 本设计指导后续重构实现

## 背景与目标

### 当前问题

1. **main.rs 持有大量不该持有的组件**：memory、sessions、tools、skill_registry、mcp_manager、scheduler 等全部在 server 层创建，再传给 AgentRuntime
2. **Scheduler 绕过 AgentRuntime**：直接 new AgentLoop 执行任务，不经过 AgentRuntime 的生命周期管理
3. **双重执行路径**：WebSocket 走 AgentRuntime → AgentExecutor → AgentLoop，Scheduler 直接走 AgentLoop
4. **AgentRuntime 与 Session 强绑定**：`handles: DashMap<SessionId, AgentExecutorHandle>` 对 workbench 单用户场景过度设计
5. **Provider 构建泄漏到 server 层**：main.rs 直接调 create_provider()

### 重构目标

- AgentRuntime 成为唯一的运行时容器，所有 agent 执行相关逻辑都归它管
- main.rs 只负责加载配置和启动 AgentRuntime
- 依赖方向：main.rs → AgentRuntime，单向干净
- workbench 单用户场景：AgentRuntime 包含一个主 AgentExecutor，不需要多 session 管理

---

## 设计一：AgentRuntime 职责边界

### 重构后的组件归属

| 组件 | 现在 | 重构后 |
|------|------|--------|
| provider / provider_chain | server 构建传入 | AgentRuntime 内部构建（从 Config 读取） |
| memory | server 构建传入 | AgentRuntime 内部构建 |
| sessions | server 构建传入 | AgentRuntime 内部构建 |
| tools | server 构建传入 | AgentRuntime 内部构建 |
| skill_registry | server 构建传入 | AgentRuntime 内部构建 |
| memory_store | server 构建传入 | AgentRuntime 内部构建 |
| recorder | server 构建传入 | AgentRuntime 内部构建 |
| agent_catalog | server 构建传入 | AgentRuntime 内部构建 |
| agent_store | server 构建传入 | AgentRuntime 内部构建 |
| mcp_manager | AppState 持有 | AgentRuntime 内部持有 |
| scheduler | AppState 持有，绕过 AgentRuntime | AgentRuntime 内部持有，只持有 storage |

### main.rs 只传

- `Config`（包含 database.path、provider、scheduler、skills、mcp 等全部配置）
- 其他外部配置（skills 路径等）

### AgentRuntime 内部从 Config 读取

- `config.database.path` → db connection → AgentStore → AgentCatalog
- `config.provider` → 创建 Provider（从 api_key/base_url）
- `config.scheduler` → 创建 Scheduler（传 storage）
- `config.skills` → 加载 skills → SkillRegistry
- `config.mcp` → MCP 配置

---

## 设计二：AgentRuntime 结构简化

### 单主 Executor 模型（workbench 场景）

```rust
pub struct AgentRuntime {
    // 单一主 executor（workbench 单用户场景）
    primary_executor: Option<Arc<AgentExecutor>>,

    // 依赖（全部内部构建）
    catalog: AgentCatalog,
    provider: Arc<dyn Provider>,
    provider_chain: Option<Arc<ProviderChain>>,
    tools: ToolRegistry,
    skill_registry: Option<Arc<SkillRegistry>>,
    memory: Arc<dyn WorkingMemory>,
    sessions: Arc<dyn SessionStore>,
    memory_store: Option<Arc<dyn MemoryStore>>,
    recorder: Option<Arc<ToolExecutionRecorder>>,

    // 管理组件
    scheduler: Option<Scheduler>,
    mcp_manager: McpManager,

    // 配置
    config: RuntimeConfig,
}
```

### 方法变更

- `get_or_spawn(session_id, ...)` → `start_primary(user_id, sandbox_id, initial_history)`
- `get(session_id)` → `executor()` 返回 `&Arc<AgentExecutor>`
- `remove(session_id)` → 只在 shutdown 时使用
- WebSocket 直接通过 `primary_executor.handle` 通信

---

## 设计三：启动流程

```
main.rs
  │
  └── Config::load(config_path, cli_args, env)
          │
          └── AgentRuntime::new(config)
                  │
                  ├── Open db connection
                  ├── Create AgentStore → AgentCatalog
                  ├── Create Provider (from config.provider)
                  ├── Create ProviderChain (if configured)
                  ├── Create Memory / Sessions / Tools
                  ├── Load Skills → SkillRegistry
                  ├── Create Scheduler (pass storage, not provider/tools)
                  ├── Create McpManager
                  ├── If scheduler.enabled: spawn scheduler loop
                  └── Return AgentRuntime

AppState::new(config, agent_runtime)
          │
          └── router::build_router(state)
```

---

## 设计四：Scheduler 重构

### 问题现状

`Scheduler::run_agent_task()` 直接 `new AgentLoop()`，完全绕过 AgentRuntime：
- 持有 provider/tools/memory 只是为了能 new AgentLoop
- 自己从 session_store 创建 session，AgentRuntime 不知道这个执行

### 重构后

Scheduler 只管"什么时候触发"：

```rust
pub struct Scheduler {
    config: SchedulerConfig,
    storage: Arc<dyn SchedulerStorage>,
    cron_parser: CronParser,
    // 不再持有 provider/tools/memory
}

impl Scheduler {
    /// 触发时调 AgentRuntime 执行
    pub async fn trigger(&self, runtime: &AgentRuntime, task: &ScheduledTask) {
        runtime.execute_scheduled_task(task).await;
    }
}
```

AgentRuntime 添加执行入口：

```rust
impl AgentRuntime {
    pub async fn execute_scheduled_task(&self, task: &ScheduledTask) -> Result<String, AgentError> {
        // 创建 session，执行任务，返回结果
        // 复用 primary_executor 或创建临时 executor
    }
}
```

---

## 设计五：MCP 集成

MCP Server 管理移入 AgentRuntime：

```rust
impl AgentRuntime {
    /// 添加 MCP server → 自动注册 tools
    pub async fn add_mcp_server(&self, config: McpServerConfig) -> Result<(), AgentError> {
        let tools = self.mcp_manager.start_server(config).await?;
        for (name, tool) in tools {
            self.tools.register(name, tool);
        }
    }

    /// 移除 MCP server → 自动注销 tools
    pub async fn remove_mcp_server(&self, server_id: &str) -> Result<(), AgentError> {
        let tools = self.mcp_manager.stop_server(server_id).await?;
        for name in tools {
            self.tools.unregister(&name);
        }
    }

    /// 列出运行中的 MCP servers
    pub fn list_mcp_servers(&self) -> Vec<McpServerInfo> {
        self.mcp_manager.list_servers()
    }
}
```

---

## 设计六：错误处理

统一错误类型：

```rust
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("Agent not found: {0}")]
    NotFound(AgentId),

    #[error("Invalid state transition: cannot {action} from {from}")]
    InvalidTransition { from: AgentStatus, action: &'static str },

    #[error("Database error: {0}")]
    DbError(String),

    #[error("Provider error: {0}")]
    ProviderError(String),

    #[error("MCP error: {0}")]
    McpError(String),

    #[error("Scheduler error: {0}")]
    SchedulerError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Execution error: {0}")]
    ExecutionError(String),
}
```

各模块错误统一转换为 AgentError 向上传播。

---

## 设计七：AppState 简化

重构后 AppState 极其简洁：

```rust
pub struct AppState {
    /// 完整配置（包含 database、provider、scheduler 等）
    config: Config,
    /// 唯一的运行时容器
    agent_supervisor: Arc<AgentRuntime>,
}
```

main.rs 简化为：

```rust
#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::load(...)?;
    let agent_runtime = AgentRuntime::new(config).await?;
    let state = AppState::new(config, agent_runtime);
    let app = router::build_router(state);
    // ... 启动 server
}
```

---

## 设计八：迁移路径

建议分阶段独立可测试：

### Phase 1：Provider 构建内化
- 把 create_provider 调用移入 AgentRuntime::new()
- main.rs 只传 Config.provider
- 验证：AgentRuntime 内部创建 provider

### Phase 2：Scheduler 重构
- Scheduler 不再持有 provider/tools/memory
- run_agent_task 改为调用 AgentRuntime.execute_scheduled_task()
- 验证：Scheduled task 通过 AgentRuntime 执行

### Phase 3：单一 Executor
- 去掉 handles: DashMap
- 改为 primary_executor: Option<Arc<AgentExecutor>>
- 简化 get_or_spawn → start_primary
- 验证：单用户场景正常工作

### Phase 4：全部内化
- 把 memory/sessions/tools/mcp_manager 全部移入 AgentRuntime
- main.rs 只传 Config
- 验证：Config 加载正确，各组件初始化成功

---

## 设计九：测试策略

### 单元测试
- AgentCatalog CRUD 测试
- ToolRegistry 注册/注销测试
- Scheduler cron 解析测试

### 集成测试
- Mock Config，测试 AgentRuntime::new() 完整流程
- 测试 provider 创建
- 测试 scheduler 触发逻辑

### API 测试
- HTTP 测试 agent 生命周期（create/start/stop/pause/resume）
- WebSocket 测试消息通信

---

## 备注

- octo-workbench：单用户系统，AgentRuntime 包含一个主 AgentExecutor
- octo-platform：多用户系统，未来扩展（需要 multi-tenant 设计）
- SubAgent 机制：等 platform 需求明确后再设计
