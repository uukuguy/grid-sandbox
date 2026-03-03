# D6: AgentSupervisor 统一管理 Agent 相关依赖实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将所有 agent 运行时依赖（tools、memory、memory_store、sessions、recorder、provider_chain、catalog）统一归 AgentSupervisor 管理，AppState 只保留 server 基础设施字段，REST API 通过 Supervisor 的 getter 方法访问这些资源。

**Architecture:** AgentSupervisor 新增公开 getter 方法暴露内部依赖（只读引用），AgentSupervisor.catalog 从 `pub` 改为私有并通过方法访问，AppState 删除冗余字段（tools、memory、memory_store、sessions、model、recorder、skill_registry、provider_chain），main.rs 构建路径简化，REST API handler 改为从 `state.agent_supervisor.xxx()` 获取依赖。

**Tech Stack:** Rust, Tokio, Axum, SQLite

---

## 目标架构对照

### AppState 字段变化

| 字段 | 当前 | 变化后 | 原因 |
|------|------|--------|------|
| `provider_chain` | AppState | 移入 Supervisor | agent 依赖 |
| `tools` | AppState | 移入 Supervisor（已有） | 重复持有 |
| `memory` | AppState | 移入 Supervisor（已有） | 重复持有 |
| `memory_store` | AppState | 移入 Supervisor（已有） | 重复持有 |
| `sessions` | AppState | 移入 Supervisor（已有） | 重复持有 |
| `model` | AppState | 删除（Supervisor 内有 `default_model`） | 重复持有 |
| `recorder` | AppState | 移入 Supervisor（已有） | 重复持有 |
| `skill_registry` | AppState | 删除（Supervisor 内有） | 标注了 `#[allow(dead_code)]` |
| `catalog` | AppState（来自 Supervisor.pub catalog） | 通过 Supervisor getter 访问 | agent 系统内部 |
| `db_path` | AppState | **保留** | server 基础设施 |
| `mcp_manager` | AppState | **保留** | 暂不动 |
| `config` | AppState | **保留** | server 配置 |
| `auth_config` | AppState | **保留** | 请求认证 |
| `metrics_registry` | AppState | **保留** | 可观测性 |
| `agent_supervisor` | AppState | **保留** | 创建/启停 agent |
| `agent_handle` | AppState | **保留** | ws.rs channel 接入点 |
| `scheduler` | AppState | **保留**（暂不动，依赖较复杂） | 后续单独处理 |

### AgentSupervisor 新增 getter 方法

```rust
// 现有字段改可见性：pub catalog → private catalog
pub fn catalog(&self) -> &Arc<AgentCatalog>
pub fn tools(&self) -> &Arc<ToolRegistry>
pub fn memory(&self) -> &Arc<dyn WorkingMemory>
pub fn memory_store(&self) -> Option<&Arc<dyn MemoryStore>>
pub fn session_store(&self) -> Option<&Arc<dyn SessionStore>>
pub fn recorder(&self) -> Option<&Arc<ToolExecutionRecorder>>
pub fn provider_chain(&self) -> Option<&Arc<ProviderChain>>
```

---

## 改动文件一览

| 文件 | 改动类型 | 核心变化 |
|------|---------|---------|
| `crates/octo-engine/src/agent/runtime_registry.rs` | 修改 | 新增 getter 方法，catalog 改私有，新增 `provider_chain` 字段 |
| `crates/octo-engine/src/lib.rs` | 修改 | 重新导出新 getter 可见的类型（ProviderChain） |
| `crates/octo-server/src/state.rs` | 大幅简化 | 删除 7 个冗余字段 |
| `crates/octo-server/src/main.rs` | 简化 | AppState::new 调用参数减少 |
| `crates/octo-server/src/api/agents.rs` | 修改 | `s.catalog` → `s.agent_supervisor.catalog()` |
| `crates/octo-server/src/api/tools.rs` | 修改 | `state.tools` → `state.agent_supervisor.tools()` |
| `crates/octo-server/src/api/memories.rs` | 修改 | `state.memory` / `state.memory_store` → Supervisor getter |
| `crates/octo-server/src/api/sessions.rs` | 修改 | `state.sessions` → `state.agent_supervisor.session_store()` |
| `crates/octo-server/src/api/executions.rs` | 修改 | `state.recorder` → `state.agent_supervisor.recorder()` |
| `crates/octo-server/src/api/providers.rs` | 修改 | `state.provider_chain` → `state.agent_supervisor.provider_chain()` |

---

## Task 1：AgentSupervisor 新增 provider_chain 字段 + getter 方法

**Files:**
- Modify: `crates/octo-engine/src/agent/runtime_registry.rs`

### Step 1：在 use 区块新增 ProviderChain import

当前 use 区（line 1-17），在 `use crate::providers::Provider;` 后添加：

```rust
use crate::providers::ProviderChain;
```

### Step 2：在 AgentSupervisor struct 新增 `provider_chain` 字段

当前 struct（line 23-38）：
```rust
pub struct AgentSupervisor {
    handles: DashMap<SessionId, AgentRuntimeHandle>,
    pub catalog: Arc<AgentCatalog>,         // ← 改为 catalog（去掉 pub）
    provider: Arc<dyn Provider>,
    tools: Arc<ToolRegistry>,
    ...
    recorder: Option<Arc<ToolExecutionRecorder>>,
}
```

修改为：
```rust
pub struct AgentSupervisor {
    handles: DashMap<SessionId, AgentRuntimeHandle>,
    catalog: Arc<AgentCatalog>,             // pub → private（通过 getter 访问）
    provider: Arc<dyn Provider>,
    provider_chain: Option<Arc<ProviderChain>>,   // 新增
    tools: Arc<ToolRegistry>,
    skill_registry: Option<Arc<SkillRegistry>>,
    memory: Arc<dyn WorkingMemory>,
    memory_store: Option<Arc<dyn MemoryStore>>,
    session_store: Option<Arc<dyn SessionStore>>,
    default_model: String,
    event_bus: Option<Arc<EventBus>>,
    recorder: Option<Arc<ToolExecutionRecorder>>,
}
```

### Step 3：在 `AgentSupervisor::new()` 初始化块新增 `provider_chain: None`

当前 Self { ... }（line 48-60）中，在 `recorder: None,` 后或 `provider,` 后插入：
```rust
provider_chain: None,
```

### Step 4：新增 `with_provider_chain` builder 方法（紧接 `with_recorder` 之后）

```rust
pub fn with_provider_chain(mut self, chain: Arc<ProviderChain>) -> Self {
    self.provider_chain = Some(chain);
    self
}
```

### Step 5：新增全部 getter 方法（在 `resolve_runtime_config` 之前插入）

```rust
// ── Getter 方法（供 server API 层只读访问） ──────────────────────────────

pub fn catalog(&self) -> &Arc<AgentCatalog> {
    &self.catalog
}

pub fn tools(&self) -> &Arc<ToolRegistry> {
    &self.tools
}

pub fn memory(&self) -> &Arc<dyn WorkingMemory> {
    &self.memory
}

pub fn memory_store(&self) -> Option<&Arc<dyn MemoryStore>> {
    self.memory_store.as_ref()
}

pub fn session_store(&self) -> Option<&Arc<dyn SessionStore>> {
    self.session_store.as_ref()
}

pub fn recorder(&self) -> Option<&Arc<ToolExecutionRecorder>> {
    self.recorder.as_ref()
}

pub fn provider_chain(&self) -> Option<&Arc<ProviderChain>> {
    self.provider_chain.as_ref()
}
```

### Step 6：编译验证（引擎层）

```bash
cargo check -p octo-engine 2>&1 | head -30
```

Expected: 零错误（新字段 + getter 无副作用）

注意：`pub catalog` 改私有后 `agents.rs` 的 `s.catalog.*` 调用会报错——**这是预期的**，后续 Task 修复。

### Step 7：Commit

```bash
git add crates/octo-engine/src/agent/runtime_registry.rs
git commit -m "feat(supervisor): add provider_chain field, catalog getter, and all resource getters"
```

---

## Task 2：main.rs — 注入 provider_chain 到 AgentSupervisor，简化 AppState 构建

**Files:**
- Modify: `crates/octo-server/src/main.rs`

### Step 1：在 `agent_supervisor` 构建时追加 `.with_provider_chain()`

当前（main.rs 约 254-266 行）：
```rust
let agent_supervisor = Arc::new(
    AgentSupervisor::new(...)
    .with_skill_registry(skill_registry.clone())
    .with_memory_store(memory_store.clone())
    .with_session_store(sessions.clone())
    .with_recorder(recorder.clone()),
);
```

修改为（在 `.with_recorder(recorder.clone()),` 后追加）：
```rust
let agent_supervisor = Arc::new(
    AgentSupervisor::new(...)
    .with_skill_registry(skill_registry.clone())
    .with_memory_store(memory_store.clone())
    .with_session_store(sessions.clone())
    .with_recorder(recorder.clone())
    .with_provider_chain_opt(provider_chain.clone()),  // 见 Step 2
);
```

> **注意**：`provider_chain` 是 `Option<Arc<ProviderChain>>`，需要一个接受 Option 的 builder。

在 runtime_registry.rs 中还需添加（Task 1 Step 4 可顺带添加）：

```rust
pub fn with_provider_chain_opt(mut self, chain: Option<Arc<ProviderChain>>) -> Self {
    self.provider_chain = chain;
    self
}
```

或者在 main.rs 用条件：
```rust
let agent_supervisor = {
    let mut s = AgentSupervisor::new(
        agent_catalog.clone(),
        provider.clone(),
        tools.clone(),
        memory.clone(),
        default_model,
    )
    .with_skill_registry(skill_registry.clone())
    .with_memory_store(memory_store.clone())
    .with_session_store(sessions.clone())
    .with_recorder(recorder.clone());
    if let Some(ref chain) = provider_chain {
        s = s.with_provider_chain(chain.clone());
    }
    Arc::new(s)
};
```

**选用后者**（条件注入，无需额外 `_opt` 方法）。

### Step 2：简化 AppState::new() 调用

将当前（约 line 287-303）：
```rust
let state = Arc::new(AppState::new(
    provider_chain,
    tools,
    memory,
    sessions,
    memory_store,
    std::path::PathBuf::from(&db_path),
    mcp_manager,
    model,
    Some(recorder),
    skill_registry,
    scheduler,
    cfg.clone(),
    agent_catalog,
    agent_supervisor,
    agent_handle,
));
```

改为：
```rust
let state = Arc::new(AppState::new(
    std::path::PathBuf::from(&db_path),
    mcp_manager,
    scheduler,
    cfg.clone(),
    agent_supervisor,
    agent_handle,
));
```

### Step 3：删除 main.rs 中不再传入 AppState 的局部变量使用

`provider_chain`、`tools`、`memory`、`sessions`、`memory_store`、`model`、`recorder`、`skill_registry` 这些变量现在只传给 AgentSupervisor（已在 Step 1 注入），不再传给 AppState，因此 `let state = ...` 行之后不再引用它们。

检查编译时 `unused variable` warning 并删除多余的 `clone()` 调用（如果有）。

### Step 4：删除 main.rs 中不再使用的 imports

查找并删除：
- `SkillLoader`（如果 skill_registry 只传给 supervisor）
- `ToolExecutionRecorder`（如果 recorder 只传给 supervisor）

保留：仍在 main.rs 中使用的类型（AgentCatalog、AgentStore、AgentSupervisor、Database 等）。

### Step 5：编译验证（预期 state.rs 报错）

```bash
cargo check -p octo-server 2>&1 | head -40
```

Expected: `state.rs` 的 `AppState::new` 参数不匹配报错——**正常**，下一 Task 修复。

### Step 6：Commit（即使有编译错误，记录进度）

```bash
git add crates/octo-server/src/main.rs
git commit -m "refactor(main): inject provider_chain into AgentSupervisor, simplify AppState construction"
```

---

## Task 3：state.rs — 删除冗余字段，精简 AppState

**Files:**
- Modify: `crates/octo-server/src/state.rs`

### Step 1：删除冗余 import

将当前 import（line 4-12）：
```rust
use octo_engine::{
    auth::AuthConfig,
    mcp::{McpManager, McpStorage},
    metrics::MetricsRegistry,
    providers::ProviderChain,
    scheduler::Scheduler,
    AgentCatalog, AgentRuntimeHandle, AgentSupervisor, MemoryStore, SessionStore, SkillRegistry,
    ToolExecutionRecorder, ToolRegistry, WorkingMemory,
};
```

改为（删除不再需要的类型）：
```rust
use octo_engine::{
    auth::AuthConfig,
    mcp::{McpManager, McpStorage},
    metrics::MetricsRegistry,
    scheduler::Scheduler,
    AgentRuntimeHandle, AgentSupervisor,
};
```

保留 `Scheduler`（scheduler 字段暂保留），删除：`ProviderChain`、`AgentCatalog`、`MemoryStore`、`SessionStore`、`SkillRegistry`、`ToolExecutionRecorder`、`ToolRegistry`、`WorkingMemory`。

### Step 2：重写 AppState struct

将当前 17 个字段的 struct 改为 8 个字段：

```rust
pub struct AppState {
    pub db_path: PathBuf,
    pub mcp_manager: Arc<tokio::sync::Mutex<McpManager>>,
    /// Scheduler for periodic tasks (optional)
    pub scheduler: Option<Arc<Scheduler>>,
    /// Server configuration for frontend
    pub config: Config,
    /// Auth configuration for request authentication
    pub auth_config: AuthConfig,
    /// Metrics registry for collecting application metrics
    pub metrics_registry: Arc<RwLock<MetricsRegistry>>,
    /// Runtime supervisor: owns all agent dependencies and manages AgentRuntime lifecycle
    pub agent_supervisor: Arc<AgentSupervisor>,
    /// 主 AgentRuntime 的通信句柄（channels 唯一的 Agent 接入点）
    pub agent_handle: AgentRuntimeHandle,
}
```

### Step 3：重写 AppState::new() 签名和实现

```rust
impl AppState {
    pub fn new(
        db_path: PathBuf,
        mcp_manager: Arc<tokio::sync::Mutex<McpManager>>,
        scheduler: Option<Arc<Scheduler>>,
        config: Config,
        agent_supervisor: Arc<AgentSupervisor>,
        agent_handle: AgentRuntimeHandle,
    ) -> Self {
        let auth_config = config.auth.to_auth_config();
        let metrics_registry = Arc::new(RwLock::new(MetricsRegistry::new()));

        Self {
            db_path,
            mcp_manager,
            scheduler,
            config,
            auth_config,
            metrics_registry,
            agent_supervisor,
            agent_handle,
        }
    }

    /// Get MCP storage on-demand (creates new connection each time)
    pub fn mcp_storage(&self) -> Option<octo_engine::mcp::storage::McpStorage> {
        McpStorage::new(&self.db_path).ok()
    }

    /// Get audit storage on-demand (creates new connection each time)
    pub fn audit_storage(&self) -> Option<octo_engine::audit::AuditStorage> {
        octo_engine::audit::AuditStorage::new(&self.db_path).ok()
    }
}
```

### Step 4：编译验证（预期 API handler 报错）

```bash
cargo check -p octo-server 2>&1 | grep "^error" | head -20
```

Expected: `agents.rs`、`tools.rs`、`memories.rs`、`sessions.rs`、`executions.rs`、`providers.rs` 的 `state.xxx` 字段访问报错——**正常**，后续 Task 逐一修复。

### Step 5：Commit

```bash
git add crates/octo-server/src/state.rs
git commit -m "refactor(state): remove agent deps from AppState — now owned by AgentSupervisor"
```

---

## Task 4：修复 agents.rs — 通过 Supervisor 访问 catalog

**Files:**
- Modify: `crates/octo-server/src/api/agents.rs`

### Step 1：将所有 `s.catalog` 替换为 `s.agent_supervisor.catalog()`

当前调用点（约 8 处，line 42-128）：

| 原代码 | 改为 |
|--------|------|
| `s.catalog.list_all()` | `s.agent_supervisor.catalog().list_all()` |
| `s.catalog.register(manifest)` | `s.agent_supervisor.catalog().register(manifest)` |
| `s.catalog.get(&id)` | `s.agent_supervisor.catalog().get(&id)` |
| `s.catalog.mark_running(...)` | `s.agent_supervisor.catalog().mark_running(...)` |
| `s.catalog.mark_stopped(...)` | `s.agent_supervisor.catalog().mark_stopped(...)` |
| `s.catalog.mark_paused(...)` | `s.agent_supervisor.catalog().mark_paused(...)` |
| `s.catalog.mark_resumed(...)` | `s.agent_supervisor.catalog().mark_resumed(...)` |
| `s.catalog.unregister(...)` | `s.agent_supervisor.catalog().unregister(...)` |

### Step 2：删除不再需要的 import（如果有直接引用 AppState 以外的类型）

检查 `use crate::state::AppState;` 保持不变，其他 import 无需改动。

### Step 3：编译验证

```bash
cargo check -p octo-server 2>&1 | grep "agents" | head -10
```

Expected: agents.rs 无错误

### Step 4：Commit

```bash
git add crates/octo-server/src/api/agents.rs
git commit -m "refactor(agents-api): access catalog via agent_supervisor.catalog()"
```

---

## Task 5：修复 tools.rs — 通过 Supervisor 访问 tools

**Files:**
- Modify: `crates/octo-server/src/api/tools.rs`

### Step 1：替换字段访问

| 原代码 | 改为 |
|--------|------|
| `state.tools.specs()` | `state.agent_supervisor.tools().specs()` |
| `state.tools.get(&spec.name)` | `state.agent_supervisor.tools().get(&spec.name)` |

完整改后的 `list_tools`：
```rust
pub async fn list_tools(State(state): State<Arc<AppState>>) -> Json<Vec<ToolInfo>> {
    let tools = state.agent_supervisor.tools();
    let specs = tools.specs();
    let tools_list: Vec<ToolInfo> = specs
        .into_iter()
        .map(|spec| {
            let source = tools
                .get(&spec.name)
                .map(|t| t.source())
                .unwrap_or(ToolSource::BuiltIn);
            ToolInfo {
                name: spec.name,
                description: spec.description,
                source,
            }
        })
        .collect();
    Json(tools_list)
}
```

### Step 2：编译验证

```bash
cargo check -p octo-server 2>&1 | grep "tools" | head -10
```

### Step 3：Commit

```bash
git add crates/octo-server/src/api/tools.rs
git commit -m "refactor(tools-api): access tools via agent_supervisor.tools()"
```

---

## Task 6：修复 memories.rs — 通过 Supervisor 访问 memory/memory_store

**Files:**
- Modify: `crates/octo-server/src/api/memories.rs`

### Step 1：替换所有 `state.memory_store` 和 `state.memory`

| 原代码 | 改为 |
|--------|------|
| `state.memory_store.list(...)` | `state.agent_supervisor.memory_store().unwrap().list(...)` → 见下方封装 |
| `state.memory_store.search(...)` | 同上 |
| `state.memory_store.get(...)` | 同上 |
| `state.memory_store.delete(...)` | 同上 |
| `state.memory_store.delete_by_filter(...)` | 同上 |
| `state.memory_store.store(...)` | 同上 |
| `state.memory.get_blocks(...)` | `state.agent_supervisor.memory().get_blocks(...)` |

**封装局部变量避免重复 unwrap**：
```rust
// 在每个需要 memory_store 的函数开头添加：
let mem_store = match state.agent_supervisor.memory_store() {
    Some(s) => s,
    None => return Json(serde_json::json!({ "error": "memory store not configured" })),
};
```

### Step 2：逐函数修改

**search_memories**（line 41-73）：用局部变量 `mem_store`，替换 `state.memory_store.*`。

**get_working_memory**（line 80-92）：
```rust
match state.agent_supervisor.memory().get_blocks(&user_id, &sandbox_id).await {
```

**get_memory**（line 94-112）：用 `mem_store` 局部变量。

**delete_memory**（line 114-133）：用 `mem_store` 局部变量。

**delete_memories_by_filter**（line 141-164）：用 `mem_store` 局部变量。

**create_memory**（line 167-199）：用 `mem_store` 局部变量。

### Step 3：编译验证

```bash
cargo check -p octo-server 2>&1 | grep "memories" | head -10
```

### Step 4：Commit

```bash
git add crates/octo-server/src/api/memories.rs
git commit -m "refactor(memories-api): access memory deps via agent_supervisor getters"
```

---

## Task 7：修复 sessions.rs — 通过 Supervisor 访问 session_store

**Files:**
- Modify: `crates/octo-server/src/api/sessions.rs`

### Step 1：替换 `state.sessions`

| 原代码 | 改为 |
|--------|------|
| `state.sessions.list_sessions_for_user(...)` | 见下方 |
| `state.sessions.get_session_for_user(...)` | 见下方 |
| `state.sessions.get_messages(...)` | 见下方 |

**封装局部变量**：
```rust
let sessions = match state.agent_supervisor.session_store() {
    Some(s) => s,
    None => return Json(serde_json::json!({ "error": "session store not configured" })),
};
```

然后将 `state.sessions.*` 全部替换为 `sessions.*`。

### Step 2：编译验证

```bash
cargo check -p octo-server 2>&1 | grep "sessions" | head -10
```

### Step 3：Commit

```bash
git add crates/octo-server/src/api/sessions.rs
git commit -m "refactor(sessions-api): access session_store via agent_supervisor.session_store()"
```

---

## Task 8：修复 executions.rs — 通过 Supervisor 访问 recorder

**Files:**
- Modify: `crates/octo-server/src/api/executions.rs`

### Step 1：替换 `state.recorder`

当前代码模式（3 处）：
```rust
match &state.recorder {
    Some(recorder) => recorder.xxx(...),
    None => ...,
}
```

改为：
```rust
match state.agent_supervisor.recorder() {
    Some(recorder) => recorder.xxx(...),
    None => ...,
}
```

注意：`recorder()` 返回 `Option<&Arc<ToolExecutionRecorder>>`，`match` 绑定的是引用，需要调整 match 内部的借用（通常可以直接用，因为 `Arc` 是 `Clone`）：

如果编译器抱怨借用，改为：
```rust
if let Some(recorder) = state.agent_supervisor.recorder() {
    match recorder.xxx(...).await {
        Ok(result) => ...,
        Err(e) => ...,
    }
} else {
    Json(vec![])
}
```

`list_session_executions` 中还有 `state.sessions.get_session_for_user(...)` 的调用（用于鉴权），同步改为：
```rust
let sessions = match state.agent_supervisor.session_store() {
    Some(s) => s,
    None => return Json(vec![]),
};
let session = sessions.get_session_for_user(&session_id_obj, &user_id).await;
```

### Step 2：编译验证

```bash
cargo check -p octo-server 2>&1 | grep "executions" | head -10
```

### Step 3：Commit

```bash
git add crates/octo-server/src/api/executions.rs
git commit -m "refactor(executions-api): access recorder via agent_supervisor.recorder()"
```

---

## Task 9：修复 providers.rs — 通过 Supervisor 访问 provider_chain

**Files:**
- Modify: `crates/octo-server/src/api/providers.rs`

### Step 1：替换 `state.provider_chain`

当前模式（6 处）：
```rust
let chain = state.provider_chain.as_ref();
```

改为：
```rust
let chain = state.agent_supervisor.provider_chain();
```

`provider_chain()` 返回 `Option<&Arc<ProviderChain>>`，和 `state.provider_chain.as_ref()` 的类型一致（都是 `Option<&Arc<ProviderChain>>`），所以后续的 `match chain { Some(c) => ..., None => ... }` 代码**无需修改**。

### Step 2：编译验证

```bash
cargo check -p octo-server 2>&1 | grep "providers" | head -10
```

### Step 3：Commit

```bash
git add crates/octo-server/src/api/providers.rs
git commit -m "refactor(providers-api): access provider_chain via agent_supervisor.provider_chain()"
```

---

## Task 10：全量验证与收尾

### Step 1：全工作区编译

```bash
cargo check --workspace 2>&1 | grep "^error"
```

Expected: 无输出（零错误）

### Step 2：运行全量测试

```bash
cargo test --workspace 2>&1 | tail -10
```

Expected: `X tests passed`，0 failed

### Step 3：验证 AppState 字段已精简

```bash
grep -n "pub " crates/octo-server/src/state.rs
```

Expected: 只有 8 个 pub 字段（db_path、mcp_manager、scheduler、config、auth_config、metrics_registry、agent_supervisor、agent_handle）

### Step 4：验证 AgentSupervisor.catalog 已私有

```bash
grep -n "pub catalog" crates/octo-engine/src/agent/runtime_registry.rs
```

Expected: 无输出

### Step 5：验证 API 层无直接字段绕过

```bash
grep -rn "state\.\(tools\|memory\b\|memory_store\|sessions\|recorder\|provider_chain\|skill_registry\|model\b\|catalog\)\b" \
    crates/octo-server/src/api/ --include="*.rs"
```

Expected: 无输出（所有访问均通过 `state.agent_supervisor.xxx()`）

### Step 6：更新 checkpoint

更新 `docs/plans/.checkpoint.json`：
```json
{
  "plan_file": "docs/plans/2026-03-03-d6-supervisor-owns-agent-deps.md",
  "phase": "completed",
  "created_at": "2026-03-03T20:00:00+08:00",
  "updated_at": "2026-03-03T20:00:00+08:00",
  "completed_tasks": ["T1","T2","T3","T4","T5","T6","T7","T8","T9","T10"],
  "current_task": null,
  "execution_mode": "executing-plans",
  "phase_name": "D6: AgentSupervisor 统一管理 Agent 相关依赖",
  "notes": "D6 完成。AppState 精简为 8 个字段（server 基础设施），所有 agent 依赖统一由 AgentSupervisor 持有并通过 getter 方法暴露。API handler 全部改为通过 state.agent_supervisor.xxx() 访问。"
}
```

### Step 7：最终 Commit

```bash
git add docs/plans/.checkpoint.json
git commit -m "checkpoint: D6 complete - AgentSupervisor owns all agent deps, AppState simplified"
```

---

## 依赖图

```
main.rs
  ├── 构建所有依赖（provider、tools、memory、recorder 等）
  ├── 注入 AgentSupervisor（持有全部 agent 依赖）
  └── 构建 AppState（只传 server 基础设施 + supervisor）

AppState（server 层）
  ├── db_path, mcp_manager, scheduler, config, auth_config, metrics_registry
  ├── agent_supervisor ──── AgentSupervisor（agent 系统边界）
  │                           ├── catalog（agent 定义）
  │                           ├── provider / provider_chain（LLM）
  │                           ├── tools + skill_registry（工具）
  │                           ├── memory（working memory）
  │                           ├── memory_store（persistent memory）
  │                           ├── session_store（session 持久化）
  │                           └── recorder（工具执行记录）
  └── agent_handle ─────────→ AgentRuntimeHandle（主 Runtime channel）

REST API handlers
  └── state.agent_supervisor.catalog() / tools() / memory() / ...
```
