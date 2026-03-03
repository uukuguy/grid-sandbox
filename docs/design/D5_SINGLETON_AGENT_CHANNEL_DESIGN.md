# D5：AgentSupervisor 主 Runtime + Channels 解耦设计

## 一、背景与问题

### 当前架构问题

当前 `ws.rs` 以 `session_id` 为 key 调用 `AgentSupervisor.get_or_spawn()`，导致：

1. **AgentRuntime 与 session 生命周期耦合**：每个 WebSocket 连接对应一个独立的 AgentRuntime
2. **历史无法共享**：PC 和 Mobile 连接到同一 server，各有各的对话历史，互不可见
3. **ws.rs 承担了不属于它的职责**：session 创建、历史加载、Runtime 路由逻辑都在 ws.rs 中
4. **AgentSupervisor 被 channel 层直接持有**：ws.rs 持有 `Arc<AgentSupervisor>` 引用，耦合过深

### 用户场景

octo-workbench 是**单智能体实例系统**，主要供个人使用。用户通过多个 channels（PC Client、Telegram 等）访问，各终端**共享同一会话历史**，一个终端的对话即时显示在所有连接的终端上。

---

## 二、目标架构

### 核心原则

1. **AgentSupervisor 是实例**（通过 `Arc<AgentSupervisor>` 注入），server 生命周期内持续存在
2. **主 Runtime 在 server 启动时预热**，持续运行，不与任何 WebSocket 连接绑定
3. **AgentSupervisor 与 channels 完全解耦**：channels 只持有 `AgentRuntimeHandle`，不知道 AgentSupervisor 存在
4. **ws.rs 是纯 channel 层**：只负责消息转发，不做 session 管理和 Runtime 路由

### 架构图

```
Server 启动
  └─ AgentSupervisor.start_primary()
       └─ 返回 AgentRuntimeHandle
            └─ 存入 AppState.agent_handle

WebSocket Channel (ws.rs)        Telegram Channel（未来）
  └─ state.agent_handle              └─ state.agent_handle
       ├─ .subscribe()                    ├─ .subscribe()
       └─ .send(UserMessage)              └─ .send(UserMessage)
                 ↓
        AgentRuntime（主，持续运行）
                 ↓
        broadcast::Sender<AgentEvent>
          ├─ → ws.rs rx（PC Client）
          ├─ → ws.rs rx（Mobile）
          └─ → Telegram rx（未来）
```

### 解耦机制

`AgentRuntimeHandle`（已有类型，`runtime.rs:32`）包含：
- `tx: mpsc::Sender<AgentMessage>` — 向 Agent 发消息
- `broadcast_tx: broadcast::Sender<AgentEvent>` — 订阅 Agent 事件
- `session_id: SessionId` — 关联的 session（用于 history 持久化）

server 启动时，从 AgentSupervisor 取出 Handle 注入 AppState。channels 只和 `AppState.agent_handle` 交互，AgentSupervisor 对 channels 完全透明。

---

## 三、AgentCatalog 的角色

`AgentCatalog` 管理多个"身份"的智能体定义（AgentManifest），包含：
- `system_prompt` / `role` / `goal` / `backstory` — 身份设定
- `tool_filter` — 可用工具限制
- `model` — 模型选择
- `config` — AgentConfig（max_rounds、enable_parallel 等）

**server 启动时选定主 agent 身份**：从 catalog 中取第一个定义的 agent（或 None，使用默认 SOUL.md 配置），作为主 Runtime 的 manifest。

未来扩展：可通过 REST API 动态切换主 agent 身份（`POST /agents/:id/activate`）。

---

## 四、改动范围

| 文件 | 改动类型 | 描述 |
|------|---------|------|
| `crates/octo-engine/src/agent/runtime_registry.rs` | 新增方法 | `start_primary()` — 语义封装，调用 `get_or_spawn()` 并返回 Handle |
| `crates/octo-server/src/state.rs` | 新增字段 | `agent_handle: AgentRuntimeHandle` |
| `crates/octo-server/src/main.rs` | 新增逻辑 | 启动时创建 primary session，调用 `start_primary()`，注入 Handle |
| `crates/octo-server/src/ws.rs` | 大幅简化 | 删除 session 路由逻辑，改为直接使用 `state.agent_handle` |

---

## 五、关键设计决策

| 决策 | 选择 | 理由 |
|------|------|------|
| AgentRuntimeHandle 存储位置 | AppState.agent_handle | channels 通过 AppState 注入，符合 Axum 模式 |
| primary session 生命周期 | server 重启创建新 session | 简单可靠；history 通过 SQLite 持久化，重启后可通过 get_messages 恢复 |
| agent_supervisor 是否保留 | 保留在 AppState | REST API /agents 管理仍需要它 |
| get_or_spawn 是否保留 | 保留 | 供未来 sub-agent 使用 |
| ws.rs 是否保留 SessionCreated 消息 | 保留 | 前端需要 session_id 用于 UI 状态初始化 |

---

## 六、历史持久化路径

```
AgentRuntime.run() 每轮结束
  └─ session_store.set_messages(primary_session_id, history)
       └─ SQLite: sessions 表
            └─ 所有 channel 读到同一份 history
```

---

## 七、未来扩展点

1. **sub-agent 支持**：AgentSupervisor 已有 `handles: DashMap<SessionId, AgentRuntimeHandle>`，可以管理多个 Runtime
2. **跨重启持久化 session_id**：将 primary_session_id 写入 config 或 DB，重启后恢复
3. **动态切换 agent 身份**：REST API `POST /agents/:id/activate` 触发新 primary
4. **Telegram channel**：从 AppState 取 agent_handle，与 ws.rs 完全对等
