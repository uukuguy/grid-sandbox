# 多会话复用设计与实施方案

## 概述

本文档描述在单个 octo 进程中实现多会话并发复用的完整设计方案。核心目标是让 octo-server 能在同一个 Tokio 异步运行时内同时管理 N 个独立会话，每个会话拥有完全隔离的上下文、工具和数据环境。

### 设计原则

1. **隔离优先** — 先修复共享状态泄漏，再实现多会话路由
2. **最小变更** — 复用现有 SessionId/AgentExecutor 架构，不重写
3. **后向兼容** — 单会话工作台模式（octo-server）行为不变

---

## 第一部分：会话隔离审计与修复

### 1.1 当前隔离状态

| 组件 | 隔离 | 风险 | 根因 |
|------|------|------|------|
| Working Memory (L0) | ✅ | 低 | 每 session 独立 `InMemoryWorkingMemory` 实例 |
| Session Memory (L1) | ✅ | 低 | SQL 按 `session_id` 过滤 |
| **Persistent Memory (L2)** | ⚠️ | **高** | `fts_search()` / `vector_search()` 只按 `user_id` 过滤 |
| **Knowledge Graph** | ❌ | **严重** | 全局单实例 `HashMap`，无 user/session 维度 |
| **Tool Registry** | ❌ | **严重** | 单个 `Arc<Mutex<ToolRegistry>>`，MCP 安装影响全局 |
| **MCP Manager** | ❌ | **高** | 单个 manager，所有 session 共享客户端连接 |
| Security Policy | ✅ | 低 | 每 executor 独立实例 |
| Sandbox (Docker) | ✅ | 低 | `SessionSandboxManager` 按 session_id 隔离 |
| Working Directory | ✅ | 低 | 每 executor 独立 PathBuf |
| Context Managers | ✅ | 低 | 每 loop 新建 |
| Event Bus | ⚠️ | 中 | 共享广播，事件含 session_id 但 history 不过滤 |
| Event Store | ✅ | 低 | SQL 按 session_id 过滤 |
| Metering | ✅ | 低 | 按 session_id 记录 |

### 1.2 攻击场景

**场景 A — 工具泄漏**：
Session A 调用 `mcp_install` 安装恶意 MCP server → Tool Registry 全局更新 → Session B 的下一个 round 从共享 registry 做 snapshot → **Session B 自动获得恶意工具**

**场景 B — 记忆泄漏**：
Session A 存储 `memory_store("api_key=sk-xxx")` → 记忆写入 memories 表 → Session B 调用 `memory_search("api_key")` → `fts_search()` 只按 user_id 过滤 → **如果同一个 user 跨 session，数据泄漏**

**场景 C — KG 泄漏**：
Session A 调用 `kg_add_entity("db_password", ...)` → 写入全局 KnowledgeGraph HashMap → Session B 调用 `kg_search("password")` → **直接获取**

---

## 第二部分：隔离修复方案

### 2.1 ToolRegistry 分层架构

**现状**：`AgentRuntime.tools: Arc<StdMutex<ToolRegistry>>` → 所有 executor 共享

**方案**：引入两层 registry 架构

```
GlobalToolRegistry（不可变基线工具，启动时注册一次）
    ├── bash, file_read, file_write, memory_search, ...（内置工具）
    └── execute_skill, kg_add_entity, ...（引擎工具）

SessionToolRegistry（每 session 独立，可变）
    ├── 继承 GlobalToolRegistry 的所有工具（snapshot）
    ├── + session 级 MCP 工具（mcp_install 只写这里）
    └── + session 级临时工具
```

**实现**：

- `AgentRuntime.tools` 改为 `base_tools: Arc<ToolRegistry>`（不可变，去掉 Mutex）
- 每个 `AgentExecutor` 持有自己的 `session_tools: Arc<StdMutex<ToolRegistry>>`
  - 初始化时从 `base_tools` 做一次 `snapshot()`
  - `mcp_install` / `mcp_remove` 只修改 `session_tools`
- executor 的 `tools_snapshot` 逻辑不变（每 round 从 `session_tools` snapshot）

**影响文件**：
- `crates/octo-engine/src/agent/runtime.rs` — `tools` 字段类型变更
- `crates/octo-engine/src/agent/executor.rs` — 新增 `session_tools` 字段，构造时 snapshot
- `crates/octo-engine/src/tools/mcp_manage.rs` — `McpManageHandle.tools` 指向 session 级 registry

### 2.2 MCP Manager 会话跟踪

**现状**：`AgentRuntime.mcp_manager: Arc<Mutex<McpManager>>` → 全局共享

**方案**：保留全局 McpManager 作为连接池，但增加 session 所有权跟踪

```rust
// McpManager 新增字段
struct McpManager {
    clients: HashMap<String, Arc<RwLock<Box<dyn McpClient>>>>,
    tool_infos: HashMap<String, Vec<McpToolInfo>>,
    runtime_states: HashMap<String, ServerRuntimeState>,
    // 新增：session → server names 映射
    session_servers: HashMap<String, HashSet<String>>,
}
```

- `mcp_install` 需传入 `session_id`，记录到 `session_servers`
- executor 的 tool snapshot 只包含该 session 拥有的 MCP server tools
- `mcp_remove` 只允许移除自己 session 安装的 server
- session 结束时自动清理其拥有的 MCP 连接

**影响文件**：
- `crates/octo-engine/src/mcp/manager.rs` — 新增 `session_servers` 字段和方法
- `crates/octo-engine/src/tools/mcp_manage.rs` — 传入 session_id

### 2.3 Memory Search 会话过滤

**现状**：`fts_search()` 和 `vector_search()` 的 SQL 只有 `WHERE user_id = ?`

**方案**：利用已有的 `SearchOptions.session_id` 字段（已存在但未使用）

```sql
-- fts_search 修改后
WHERE memories_fts MATCH ?1 AND m.user_id = ?2
  AND (?3 IS NULL OR m.session_id = ?3)  -- 新增

-- vector_search 修改后
WHERE user_id = ?1 AND embedding IS NOT NULL
  AND (?2 IS NULL OR session_id = ?2)     -- 新增
```

- `session_id` 为 `None` 时行为不变（后向兼容，搜索该用户所有记忆）
- `session_id` 为 `Some(id)` 时只搜索该 session 的记忆
- 多会话模式下，memory_search 工具默认传入当前 session_id

**影响文件**：
- `crates/octo-engine/src/memory/sqlite_store.rs` — `fts_search()` 和 `vector_search()` 加参数
- `crates/octo-engine/src/tools/memory.rs` — memory_search 工具传入 session_id

**设计决策**：session_id 过滤是可选的（`Option`），允许跨 session 搜索（例如管理工具场景）。默认行为是严格隔离。

### 2.4 Knowledge Graph 会话隔离

**现状**：全局单个 `KnowledgeGraph` 实例，内部 `HashMap<String, Entity>` 无 scope

**方案 A（简单）**：每 session 独立 KG 实例
- `AgentRuntime` 改为 `knowledge_graphs: DashMap<SessionId, Arc<RwLock<KnowledgeGraph>>>`
- session 创建时初始化空 KG，session 结束时 drop
- 优点：完全隔离，无侵入改动
- 缺点：跨 session 知识无法共享

**方案 B（推荐）**：Entity/Relation 加 scope 字段
- `Entity` 新增 `scope: KgScope` 枚举：`Global` / `User(String)` / `Session(String)`
- 查询时按 scope 过滤：session 查询只看 `Session(self_id)` + `User(self_user_id)` + `Global`
- 写入时默认 `Session(current_session_id)`
- 优点：支持跨 session 知识共享（通过 Global/User scope）
- 缺点：需修改 KG 所有查询方法

**推荐方案 A**（本阶段）：简单可靠，后续如需跨 session 知识共享再升级到方案 B。

**影响文件**：
- `crates/octo-engine/src/agent/runtime.rs` — KG 字段改为 DashMap
- `crates/octo-engine/src/agent/executor.rs` — 传入 session 级 KG
- `crates/octo-engine/src/tools/mod.rs` — KG 工具注册传入 session KG

### 2.5 Event Bus 会话过滤

**现状**：单个 `TelemetryBus`，所有 subscriber 收到所有事件

**方案**：在订阅端过滤（不改 bus 本身）

- `TelemetryBus` 新增 `subscribe_filtered(session_id: SessionId)` 方法
- 返回 `FilteredReceiver`，内部 skip 非本 session 的事件
- 或者：每 session 一个独立 `TelemetryBus`（更简单但消耗更多资源）

**推荐**：本阶段不改动——Event Bus 是观测层，事件已含 session_id，消费者可自行过滤。标记为 **Deferred**。

---

## 第三部分：多会话注册表

### 3.1 AgentRuntime SessionRegistry

```rust
pub struct AgentRuntime {
    // 现有字段...

    // 替代 primary_handle
    sessions: DashMap<SessionId, SessionEntry>,
    // 保留兼容
    primary_session_id: Mutex<Option<SessionId>>,
    // 基线工具（不可变）
    base_tools: Arc<ToolRegistry>,
    // 每 session 的 KG
    session_kgs: DashMap<SessionId, Arc<RwLock<KnowledgeGraph>>>,
    // 会话上限
    max_concurrent_sessions: usize,
}

struct SessionEntry {
    handle: AgentExecutorHandle,
    user_id: UserId,
    created_at: Instant,
    // session 级工具注册表
    tools: Arc<StdMutex<ToolRegistry>>,
}
```

### 3.2 核心 API

```rust
impl AgentRuntime {
    /// 创建并启动新会话
    pub async fn start_session(
        &self,
        session_id: SessionId,
        user_id: UserId,
        sandbox_id: SandboxId,
        history: Vec<ChatMessage>,
        agent_id: Option<&AgentId>,
    ) -> Result<AgentExecutorHandle, AgentError>;

    /// 停止并清理会话
    pub async fn stop_session(&self, session_id: &SessionId);

    /// 获取会话 handle
    pub fn get_session_handle(&self, session_id: &SessionId) -> Option<AgentExecutorHandle>;

    /// 列出活跃会话
    pub fn active_sessions(&self) -> Vec<SessionId>;

    /// 活跃会话数
    pub fn active_session_count(&self) -> usize;

    /// 兼容：启动 primary session
    pub async fn start_primary(...) -> AgentExecutorHandle {
        let handle = self.start_session(...).await?;
        *self.primary_session_id.lock().await = Some(session_id.clone());
        handle
    }
}
```

### 3.3 AgentExecutor 隔离字段

```rust
pub struct AgentExecutor {
    // 现有字段...

    // 改为 session 级 tools（替代共享 tools）
    session_tools: Arc<StdMutex<ToolRegistry>>,
    // session 级 KG
    knowledge_graph: Arc<RwLock<KnowledgeGraph>>,
    // session 级 MCP handle（限制 mcp_install 范围）
    mcp_handle: McpManageHandle, // tools 指向 session_tools
}
```

---

## 第四部分：WebSocket 多会话路由

### 4.1 协议变更

```
# 连接时指定 session
ws://host/ws?session_id=<id>

# 不指定 → 使用 primary session（后向兼容）
ws://host/ws
```

### 4.2 handle_socket 改造

```rust
async fn handle_socket(socket: WebSocket, state: Arc<AppState>, session_id: Option<String>) {
    let handle = match session_id {
        Some(id) => {
            let sid = SessionId::from_string(&id);
            state.agent_supervisor.get_session_handle(&sid)
                .unwrap_or_else(|| {
                    // 自动创建新 session
                    // ...
                })
        }
        None => state.agent_handle.clone(), // 后向兼容
    };
    // 其余逻辑不变...
}
```

### 4.3 REST API 扩展

| 端点 | 方法 | 说明 |
|------|------|------|
| `POST /api/sessions` | 创建 | 创建新会话并启动 executor |
| `DELETE /api/sessions/:id` | 删除 | 停止 executor + 清理 |
| `GET /api/sessions/:id/status` | 查询 | 返回 active/stopped + 统计 |
| `GET /api/sessions/active` | 列表 | 列出所有活跃会话 |

---

## 第五部分：配置

```yaml
# config.yaml 新增
sessions:
  max_concurrent: 64          # 最大并发会话数
  idle_timeout_secs: 3600     # 空闲超时（秒），0 = 不超时
  memory_isolation: strict    # strict = 按 session_id 隔离 / relaxed = 按 user_id
```

---

## 第六部分：Deferred 项

| ID | 描述 | 依赖 |
|----|------|------|
| AJ-D1 | IPC 健康检查心跳（Unix socket / gRPC） | 平台 Pod 架构 |
| AJ-D2 | 崩溃恢复 — EventStore 重放恢复会话 | 本阶段 session registry |
| AJ-D3 | 优雅关闭 — SIGTERM checkpoint 所有会话 | AJ-D2 |
| AJ-D4 | 会话 idle 超时自动回收 | 本阶段 session registry |
| AJ-D5 | 前端多会话 UI（tab 切换器） | WS 路由完成 |
| AJ-D6 | Event Bus 会话过滤订阅 | 观测需求明确后 |
| AJ-D7 | KG 方案 B — scope 字段（Global/User/Session） | 跨 session 知识需求 |

---

## 第七部分：风险评估

| 风险 | 影响 | 缓解 |
|------|------|------|
| 内存增长（每 session 独立 KG + WorkingMemory） | 中 | max_concurrent_sessions 上限 + idle 超时回收 |
| ToolRegistry snapshot 性能 | 低 | 当前 tools 数 <100，snapshot 代价 <1μs |
| MCP 连接池 session 跟踪复杂度 | 中 | 先实现简单方案（每 session 独立 MCP handle） |
| 后向兼容 | 低 | primary_session_id 保留，单用户模式行为不变 |
| 数据库锁竞争（多 session 并发写 SQLite） | 中 | WAL 模式已启用，读并发无锁；写串行可接受 |
