# AgentRuntime 架构深度分析：合理性评估与改进路径

> 分析日期：2026-03-04
> 基准：octo-workbench `octo-workbench` 分支，commit `520a1bc`
> 对标框架：Goose (block/goose)、OpenHands (All-Hands-AI/OpenHands)、pi_agent_rust (本地)

---

## 一、架构主线数据流

```
用户消息 (WebSocket)
    ↓
ws.rs: handle_socket()
    ↓ AgentMessage::UserMessage
AgentExecutorHandle.tx  (mpsc channel)
    ↓
AgentExecutor::run()     [持久化主循环]
    ↓ 每轮构建新 AgentLoop
AgentLoop::run()
    ├── Zone A: SystemPromptBuilder / manifest.system_prompt
    ├── Zone B: WorkingMemory.compile() → <context> XML
    ├── Provider.stream() → SSE 流
    ├── ToolRegistry.get() → Tool.execute()
    └── LoopGuard.check() → 防死循环
    ↓ AgentEvent broadcast
ws.rx → WebSocket 客户端
```

---

## 二、AgentRuntime 字段审计

```rust
pub struct AgentRuntime {
    // 单一主 executor（单用户场景）
    primary_handle: Mutex<Option<AgentExecutorHandle>>,  // ⚠️ 设计局限
    agent_handles: DashMap<AgentId, CancellationToken>,   // ✅ 合理
    catalog: Arc<AgentCatalog>,                           // ✅ 合理
    provider: Arc<dyn Provider>,                          // ✅ 合理
    tools: Arc<StdMutex<ToolRegistry>>,                  // ⚠️ 动态更新问题
    skill_registry: Option<Arc<SkillRegistry>>,           // ✅ 合理
    memory: Arc<dyn WorkingMemory>,                       // ⚠️ 无 session 隔离
    memory_store: Arc<dyn MemoryStore>,                   // ✅ 合理
    session_store: Arc<dyn SessionStore>,                 // ✅ 合理
    default_model: String,                                // ✅ 合理
    event_bus: Option<Arc<EventBus>>,                     // ✅ 合理
    recorder: Arc<ToolExecutionRecorder>,                 // ✅ 合理
    provider_chain: Option<Arc<ProviderChain>>,           // ✅ 合理
    mcp_manager: Option<Arc<Mutex<McpManager>>>,         // ⚠️ Option 无必要
    working_dir: PathBuf,                                 // ✅ 合理
}
```

---

## 三、已确认的问题（精确代码定位）

### P0：MCP 动态添加对运行中 Agent 无效

**代码路径**：
```
add_mcp_server()
  → self.tools.lock() → register(bridge)  ← 更新共享 ToolRegistry

start_primary()
  → build_tool_registry()  ← 生成新的独立快照 Arc<ToolRegistry>
  → AgentExecutor::new(tools=快照)  ← Executor 持有快照，不跟踪原始
```

`build_tool_registry()` 每次都 `Arc::new(新 ToolRegistry)`，`AgentExecutor` 持有的是快照，与 `self.tools` 完全解耦。运行中添加 MCP 工具，现有 session 无法感知。

**对标 Goose**：`ExtensionManager` 使用版本化缓存（`tools_cache_version`），添加/移除扩展时 `invalidate_tools_cache_and_bump_version()`，AgentLoop 每 round 前调用 `get_all_tools_cached()` 获取最新工具列表。

### P0：stop_primary 不真正终止 Executor

```rust
// executor.rs:196-198
AgentMessage::Cancel => {
    self.cancel_flag.store(true, Ordering::Relaxed);
    // while loop 继续！Executor 仍在等待下一条消息
}

// stop_primary 发送 Cancel 后 handle 被 take
// 但 AgentExecutor 的 while let Some(msg) = self.rx.recv().await 会永远等待
// 正确做法：drop sender，让 recv() 返回 None
```

### P0：Scheduler run_now 是假执行

```rust
// scheduler/mod.rs:525-562
pub async fn run_now(&self, ...) -> Result<TaskExecution, SchedulerError> {
    let execution = TaskExecution {
        status: ExecutionStatus::Success,
        result: Some("Manually triggered".to_string()),  // ← 硬编码，未实际执行
    };
    self.storage.save_execution(&execution).await?;
    Ok(execution)  // ← run_agent_task() 从未被调用
}
```

### P1：WorkingMemory 无 Session 隔离

```rust
// runtime.rs:124-128
let memory: Arc<dyn WorkingMemory> = Arc::new(SqliteWorkingMemory::new(conn.clone()).await?);
// start_primary 时：
self.memory.clone()  // 所有 AgentExecutor 共享同一个实例
```

`InMemoryWorkingMemory` 使用全局 `RwLock<HashMap>`，所有 session 的 MemoryBlock 混在一起。

### P1：ws.rs 单 session 固化

```rust
// ws.rs:128-129
let handle = &state.agent_handle;  // AppState 唯一 handle
// 所有 WebSocket 连接竞争同一个 Agent
```

无法支持多并发 session。

### P1：AgentConfig::enable_parallel 配置无效

```rust
// config.rs
pub enable_parallel: bool,  // 默认 false，从未生效

// loop_.rs 工具执行是串行 for loop，无并行分支
```

### P2：LoopTurnStarted 事件未发布

`loop_.rs` 的 `for round in 0..max_rounds` 循环开始处没有发布 `LoopTurnStarted` 事件，导致 `MetricsRegistry` 中 `octo.sessions.turns.total` 计数器永远为 0。

### P2：Option\<McpManager\> 无必要

`runtime.rs:199` 中 `new()` 总是 `Some(Arc::new(Mutex::new(McpManager::new())))`，`Option` 包装只产生 `ok_or(AgentError::McpNotInitialized)` 噪声。

### P2：build_tool_registry 每次重建（性能）

即使工具集未变化，每次 `start_primary` 都重建 `Arc<ToolRegistry>`，遍历并 clone 所有工具。应增加版本号检测。

---

## 四、对标顶级框架的差距

### 4.1 Goose (block/goose) — Rust 最相关对标

| 维度 | octo-workbench | Goose |
|------|---------------|-------|
| 工具注册 | 启动时快照，运行中不更新 | 版本化缓存，实时感知 |
| MCP 工具 | 添加后需重启 Executor 生效 | 添加后下一 round 即可用 |
| 工具移除 | 重建整个 Registry | 缓存失效后重新 fetch |
| 工具数量管理 | 全量注入（上下文膨胀风险） | 懒加载，按需获取 |
| 并行工具 | 配置项存在但未实现 | 原生支持 |

### 4.2 OpenHands — 多 Agent 架构

OpenHands 每个 session 有独立 Runtime（sandbox 隔离），Agent 间通过 EventStream 通信，支持 `AgentDelegateAction` 委托模式。octo 当前是单 session 共享 Runtime，无 Agent-to-Agent 通信机制。

### 4.3 pi_agent_rust (本地) — 高级特性

| 特性 | pi_agent_rust | octo-workbench |
|------|--------------|----------------|
| SteeringMode | one-at-a-time / batch | 无 |
| 会话分支（SessionTree） | ✅ JSONL 树结构 | ❌ 线性列表 |
| thinkingLevel | off / auto / max | 无 |
| 扩展 OAuth | ✅ | ❌ |
| autoCompaction | ✅ | 手动触发 |

---

## 五、具体优化方案

### 优化1：ToolRegistry 动态更新

**方案**：将 `Arc<StdMutex<ToolRegistry>>` 直接传给 `AgentExecutor`，AgentLoop 每 round 前 lock 获取最新工具。

```rust
// 改动点1：AgentExecutor 持有共享引用而非快照
pub struct AgentExecutor {
    tools: Arc<StdMutex<ToolRegistry>>,  // 共享引用
}

// 改动点2：AgentLoop 每 round 前生成只读快照
let tools_snapshot = {
    let guard = executor.tools.lock().unwrap();
    Arc::new(guard.snapshot())  // snapshot() 返回只读视图
};
let mut agent_loop = AgentLoop::new(provider, tools_snapshot, memory);
```

或更简洁：添加版本计数器，仅在工具变化时重建快照。

### 优化2：stop_primary 真正终止

```rust
pub async fn stop_primary(&self) {
    // take() drop AgentExecutorHandle → tx 被 drop
    // → AgentExecutor.rx.recv() 返回 None → while loop 退出
    let _handle = {
        let mut guard = self.primary_handle.lock().await;
        guard.take()
    };
    // _handle drop 时 tx 自动 drop，Executor 自然退出
}
```

### 优化3：WorkingMemory per-session 隔离

```rust
// 方案A：AgentExecutor 各自持有独立实例
let memory = Arc::new(InMemoryWorkingMemory::new());
// （简单，适合单用户）

// 方案B：SQLite 按 session_id 过滤（适合平台化）
impl WorkingMemory for SqliteWorkingMemory {
    async fn get_blocks(&self, user_id: &UserId, sandbox_id: &SandboxId) -> Result<Vec<MemoryBlock>> {
        sqlx::query!("SELECT * FROM memory_blocks WHERE sandbox_id = ?", sandbox_id.as_str())
            .fetch_all(&self.pool).await
    }
}
```

### 优化4：实现 parallel tool 执行

```rust
// loop_.rs 中
if self.config.enable_parallel && tool_calls.len() > 1 {
    let semaphore = Arc::new(Semaphore::new(self.config.max_parallel_tools as usize));
    let futs = tool_calls.iter().map(|tc| {
        let sem = semaphore.clone();
        async move {
            let _permit = sem.acquire().await.unwrap();
            self.execute_tool(tc).await
        }
    });
    let results = futures::future::join_all(futs).await;
} else {
    // 现有串行逻辑
}
```

---

## 六、octo-workbench → octo-platform 演进路径

### Phase 1：单 Agent 质量提升（近期）

| 任务 | 优先级 |
|------|--------|
| 修复 ToolRegistry 动态更新 | 🔴 P0 |
| 修复 stop_primary 真正终止 | 🔴 P0 |
| 修复 Scheduler run_now | 🔴 P0 |
| WorkingMemory session 隔离 | 🟡 P1 |
| 实现 parallel tool 执行 | 🟡 P1 |
| 发布 LoopTurnStarted 事件 | 🟢 P2 |
| 移除 Option\<McpManager\> | 🟢 P2 |

### Phase 2：多 Session 支持

```
当前：AppState → agent_handle (固定)
目标：AppState → AgentRuntime
               → SessionRegistry: HashMap<SessionId, AgentExecutorHandle>
```

- ws.rs 每个 WebSocket 连接创建独立 session
- WorkingMemory 强制 per-session 隔离
- REST API 支持 session CRUD

### Phase 3：Multi-Agent 编排

- `AgentMessage::Delegate { to: AgentId, task: String }`
- EventBus 支持 Agent-to-Agent 消息路由
- Supervisor + Worker Agent 模式
- Scheduler 支持任务图（DAG）

### Phase 4：octo-platform 多用户

```
octo-platform
├── TenantManager            # 租户隔离（per-tenant AgentRuntime）
├── UserSessionManager       # 用户会话
├── AuthMiddleware           # JWT/OAuth
├── QuotaManager             # 资源限额
└── ObservabilityStack
    ├── MetricsRegistry（完善指标）
    ├── DistributedTracing
    └── AuditLog
```

### 架构演进总图

```
octo-workbench (今天)      octo-workbench (成熟)      octo-platform
───────────────────────────────────────────────────────────────────
单 session 静态工具          多 session 动态工具          多租户多 Agent

AgentRuntime(1)             AgentRuntime(1)             AgentRuntime × N
  └ primary_handle(固定)      └ SessionRegistry              (per tenant)
                               ├ session_A → Executor        └ UserMgr
tools: 启动时快照              ├ session_B → Executor            ├ quota
                               └ session_C → Executor            └ rbac
WorkingMemory: 全局共享        WorkingMemory: per-session

MCP: 动态注册不生效            MCP: 实时感知(版本化缓存)    MCP: per-tenant 配置

Scheduler: run_now 假执行      Scheduler: 真实执行          Scheduler: 分布式调度

parallel_tool: 配置无效        parallel_tool: 真实并行      parallel_tool: 资源限制
```
