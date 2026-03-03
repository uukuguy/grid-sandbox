# D5: AgentSupervisor 主 Runtime + Channels 解耦实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** server 启动时预热主 AgentRuntime，所有 channel 共享同一 Runtime，ws.rs 降级为纯 channel 层。

**Architecture:** AgentSupervisor 新增 `start_primary()` 方法（封装 `get_or_spawn()`），server 启动时调用一次并将返回的 `AgentRuntimeHandle` 注入 `AppState.agent_handle`。channels（ws.rs 等）只持有 `AgentRuntimeHandle`（含 `mpsc::Sender` + `broadcast::Sender`），不持有 `AgentSupervisor` 引用，实现完全解耦。

**Tech Stack:** Rust, Tokio (mpsc/broadcast), Axum, SQLite

---

## 改动文件一览

| 文件 | 改动类型 | 核心变化 |
|------|---------|---------|
| `crates/octo-engine/src/agent/runtime_registry.rs` | 新增方法 | `start_primary()` — 语义封装 `get_or_spawn()` |
| `crates/octo-server/src/state.rs` | 新增字段 | `agent_handle: AgentRuntimeHandle` |
| `crates/octo-server/src/main.rs` | 新增逻辑 | 启动时创建 primary session，调用 `start_primary()`，注入 AppState |
| `crates/octo-server/src/ws.rs` | 大幅简化 | 删除 session 路由逻辑，改为直接使用 `state.agent_handle` |

---

## 关键类型参考

**`AgentRuntimeHandle`**（`crates/octo-engine/src/agent/runtime.rs`）：
```rust
pub struct AgentRuntimeHandle {
    pub tx: mpsc::Sender<AgentMessage>,
    pub broadcast_tx: broadcast::Sender<AgentEvent>,
    pub session_id: SessionId,
}
// 已实现 Clone
impl AgentRuntimeHandle {
    pub fn subscribe(&self) -> broadcast::Receiver<AgentEvent> { ... }
    pub async fn send(&self, msg: AgentMessage) -> Result<(), mpsc::error::SendError<AgentMessage>> { ... }
}
```

**`AgentSupervisor.get_or_spawn()`**（`runtime_registry.rs:95-151`）：
```rust
pub fn get_or_spawn(
    &self,
    session_id: SessionId,
    user_id: UserId,
    sandbox_id: SandboxId,
    initial_history: Vec<ChatMessage>,
    agent_id: Option<&AgentId>,
) -> AgentRuntimeHandle
```

---

## Task 1：runtime_registry.rs — 新增 `start_primary()`

**Files:**
- Modify: `crates/octo-engine/src/agent/runtime_registry.rs:151`（在 `remove()` 方法之前插入）

### Step 1：在 `get_or_spawn()` 后（line 151）插入 `start_primary()` 方法

```rust
/// 启动主 Runtime 并返回其 Handle。
/// 由 main.rs 在 server 启动时调用一次。
/// channels 通过持有返回的 Handle 与 Agent 通信，无需持有 AgentSupervisor 引用（解耦）。
pub fn start_primary(
    &self,
    session_id: SessionId,
    user_id: UserId,
    sandbox_id: SandboxId,
    initial_history: Vec<ChatMessage>,
    agent_id: Option<&AgentId>,
) -> AgentRuntimeHandle {
    self.get_or_spawn(session_id, user_id, sandbox_id, initial_history, agent_id)
}
```

### Step 2：编译验证（引擎层）

```bash
cargo check -p octo-engine
```

Expected: 零错误，零 warning（新方法无副作用）

### Step 3：Commit

```bash
git add crates/octo-engine/src/agent/runtime_registry.rs
git commit -m "feat(agent): add start_primary() to AgentSupervisor — semantic wrapper for channel decoupling"
```

---

## Task 2：state.rs — AppState 新增 `agent_handle` 字段

**Files:**
- Modify: `crates/octo-server/src/state.rs`

### Step 1：在 import 中添加 `AgentRuntimeHandle`

当前 `state.rs` line 10：
```rust
use octo_engine::{
    auth::AuthConfig,
    mcp::{McpManager, McpStorage},
    metrics::MetricsRegistry,
    providers::ProviderChain,
    scheduler::Scheduler,
    AgentCatalog, AgentSupervisor, MemoryStore, SessionStore, SkillRegistry,
    ToolExecutionRecorder, ToolRegistry, WorkingMemory,
};
```

修改为（在 `AgentCatalog, AgentSupervisor` 后添加 `AgentRuntimeHandle`）：
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

### Step 2：在 AppState struct 末尾新增 `agent_handle` 字段

当前 struct 末尾（`state.rs:39-42`）：
```rust
    /// Agent catalog for agent definitions and lifecycle state
    pub catalog: Arc<AgentCatalog>,
    /// Runtime supervisor: holds shared deps and manages AgentRuntime lifecycle
    pub agent_supervisor: Arc<AgentSupervisor>,
}
```

修改为：
```rust
    /// Agent catalog for agent definitions and lifecycle state
    pub catalog: Arc<AgentCatalog>,
    /// Runtime supervisor: holds shared deps and manages AgentRuntime lifecycle
    pub agent_supervisor: Arc<AgentSupervisor>,
    /// 主 AgentRuntime 的通信句柄（channels 唯一的 Agent 接入点）。
    /// channels 通过此 handle 发消息、订阅事件，无需持有 AgentSupervisor。
    pub agent_handle: AgentRuntimeHandle,
}
```

### Step 3：在 `AppState::new()` 签名末尾新增参数

当前 `new()` 签名结尾（`state.rs:58-59`）：
```rust
        catalog: Arc<AgentCatalog>,
        agent_supervisor: Arc<AgentSupervisor>,
    ) -> Self {
```

修改为：
```rust
        catalog: Arc<AgentCatalog>,
        agent_supervisor: Arc<AgentSupervisor>,
        agent_handle: AgentRuntimeHandle,
    ) -> Self {
```

### Step 4：在 `Self { ... }` 初始化块末尾添加 `agent_handle`

当前初始化块结尾（`state.rs:79-83`）：
```rust
            catalog,
            agent_supervisor,
        }
    }
```

修改为：
```rust
            catalog,
            agent_supervisor,
            agent_handle,
        }
    }
```

### Step 5：编译验证（server 层，预期 main.rs 报错）

```bash
cargo check -p octo-server 2>&1 | head -30
```

Expected: `state.rs` 零错误；`main.rs` 报错 "missing field `agent_handle`"（正常，下一 Task 修复）

### Step 6：Commit

```bash
git add crates/octo-server/src/state.rs
git commit -m "feat(server): add agent_handle field to AppState for channel decoupling"
```

---

## Task 3：main.rs — 启动时预热主 Runtime 并注入 AppState

**Files:**
- Modify: `crates/octo-server/src/main.rs`

### Step 1：在 `use octo_engine::{...}` 的导入列表中添加 `UserId` 相关类型

当前 main.rs line 21（`AgentStore` 等）中找到 octo_types 的 import：`main.rs` 未直接 import UserId，它来自 `sessions.create_session()` 的返回值。无需额外 import。

验证 `AgentRuntimeHandle` 是否已在 `AppState::new()` 的调用方可见：`octo_engine` 中它已被 `state.rs` 导入。`main.rs` 中需要将 `AgentRuntimeHandle` 通过变量传入，不需要额外 import（类型推断即可）。

### Step 2：在 `agent_supervisor` 构建完成后、`AppState::new()` 之前，插入主 Runtime 预热代码

定位插入点：`main.rs:266`（`let state = Arc::new(AppState::new(...))` 之前）

在当前代码：
```rust
    let state = Arc::new(AppState::new(
```

之前插入：
```rust
    // D5: 预热主 AgentRuntime（server 生命周期内持续运行，不与任何 WebSocket 绑定）
    let primary_session = sessions.create_session().await;
    let primary_history = sessions
        .get_messages(&primary_session.session_id)
        .await
        .unwrap_or_default();
    // 从 catalog 选定主 agent 身份（第一个，或 None → 使用默认 SOUL.md 配置）
    let primary_agent_id = agent_catalog.list_all().into_iter().next().map(|e| e.id);
    let agent_handle = agent_supervisor.start_primary(
        primary_session.session_id.clone(),
        primary_session.user_id.clone(),
        primary_session.sandbox_id.clone(),
        primary_history,
        primary_agent_id.as_ref(),
    );
    tracing::info!(
        session_id = %primary_session.session_id.as_str(),
        "Primary AgentRuntime started"
    );
```

### Step 3：在 `AppState::new(...)` 调用末尾添加 `agent_handle` 参数

当前末尾（`main.rs:280-283`）：
```rust
        cfg.clone(),
        agent_catalog,
        agent_supervisor,
    ));
```

修改为：
```rust
        cfg.clone(),
        agent_catalog,
        agent_supervisor,
        agent_handle,
    ));
```

### Step 4：编译验证

```bash
cargo check -p octo-server
```

Expected: 零错误（`agent_supervisor` 和 `sessions` 在此作用域内均已存在）

### Step 5：Commit

```bash
git add crates/octo-server/src/main.rs
git commit -m "feat(server): warm up primary AgentRuntime at startup, inject handle into AppState"
```

---

## Task 4：ws.rs — 简化为纯 channel 层

**Files:**
- Modify: `crates/octo-server/src/ws.rs`

### Step 1：移除不再需要的 imports

当前 `ws.rs` 顶部 import（lines 1-15）：
```rust
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Request, State, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

use octo_engine::auth::{get_user_context, UserContext};
use octo_engine::{AgentEvent, AgentMessage};
use octo_types::{SessionId, UserId};

use crate::state::AppState;
```

修改为（删除 `SessionId`, `UserId`）：
```rust
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Request, State, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

use octo_engine::auth::{get_user_context, UserContext};
use octo_engine::{AgentEvent, AgentMessage};

use crate::state::AppState;
```

### Step 2：简化 `ClientMessage`，移除 `session_id` 字段

**`SendMessage` 变体** 不再需要 `session_id`（主 Runtime 固定）：
```rust
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ClientMessage {
    #[serde(rename = "send_message")]
    SendMessage {
        #[serde(default)]
        session_id: Option<String>,   // 保留字段（向后兼容，忽略值）
        content: String,
    },
    #[serde(rename = "cancel")]
    Cancel {
        #[serde(default)]
        session_id: Option<String>,   // 保留字段（向后兼容，忽略值）
    },
}
```

> **注意**：`session_id` 字段保留但标注 `#[serde(default)]`，避免前端发送旧格式时 JSON 解析失败。

### Step 3：替换 `handle_socket` 中的 `SendMessage` 分支

**删除**：lines 139-330（从 `ClientMessage::SendMessage` 到整个 `loop { rx.recv() }` 闭合括号）

**替换为**：
```rust
        match client_msg {
            ClientMessage::SendMessage { content, .. } => {
                // 直接使用注入的主 Handle，不持有 AgentSupervisor
                let handle = &state.agent_handle;
                let sid_str = handle.session_id.as_str().to_string();

                // 告知客户端 session_id（前端 UI 初始化用）
                let created_msg = ServerMessage::SessionCreated {
                    session_id: sid_str.clone(),
                };
                let _ = sender
                    .send(Message::Text(
                        serde_json::to_string(&created_msg).unwrap().into(),
                    ))
                    .await;

                // 先订阅，再发消息（避免丢失事件）
                let mut rx = handle.subscribe();
                let _ = handle
                    .send(AgentMessage::UserMessage {
                        content,
                        channel_id: "websocket".to_string(),
                    })
                    .await;

                // 转发 agent 事件到 WebSocket
                loop {
                    match rx.recv().await {
                        Ok(event) => {
                            let server_msg = match event {
                                AgentEvent::TextDelta { text } => ServerMessage::TextDelta {
                                    session_id: sid_str.clone(),
                                    text,
                                },
                                AgentEvent::TextComplete { text } => ServerMessage::TextComplete {
                                    session_id: sid_str.clone(),
                                    text,
                                },
                                AgentEvent::ThinkingDelta { text } => {
                                    ServerMessage::ThinkingDelta {
                                        session_id: sid_str.clone(),
                                        text,
                                    }
                                }
                                AgentEvent::ThinkingComplete { text } => {
                                    ServerMessage::ThinkingComplete {
                                        session_id: sid_str.clone(),
                                        text,
                                    }
                                }
                                AgentEvent::ToolStart {
                                    tool_id,
                                    tool_name,
                                    input,
                                } => ServerMessage::ToolStart {
                                    session_id: sid_str.clone(),
                                    tool_id,
                                    tool_name,
                                    input,
                                },
                                AgentEvent::ToolResult {
                                    tool_id,
                                    output,
                                    success,
                                } => ServerMessage::ToolResult {
                                    session_id: sid_str.clone(),
                                    tool_id,
                                    output,
                                    success,
                                },
                                AgentEvent::ToolExecution { execution } => {
                                    ServerMessage::ToolExecutionEvent {
                                        session_id: sid_str.clone(),
                                        execution,
                                    }
                                }
                                AgentEvent::TokenBudgetUpdate { budget } => {
                                    ServerMessage::TokenBudgetUpdate {
                                        session_id: sid_str.clone(),
                                        budget,
                                    }
                                }
                                AgentEvent::Typing { state } => ServerMessage::Typing {
                                    session_id: sid_str.clone(),
                                    state,
                                },
                                AgentEvent::Error { message } => ServerMessage::Error {
                                    session_id: sid_str.clone(),
                                    message,
                                },
                                AgentEvent::Done => {
                                    let done_msg = ServerMessage::Done {
                                        session_id: sid_str.clone(),
                                    };
                                    let _ = sender
                                        .send(Message::Text(
                                            serde_json::to_string(&done_msg).unwrap().into(),
                                        ))
                                        .await;
                                    break;
                                }
                            };

                            if let Ok(json) = serde_json::to_string(&server_msg) {
                                if sender.send(Message::Text(json.into())).await.is_err() {
                                    break;
                                }
                            }
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            debug!("Broadcast lagged by {n} messages");
                        }
                    }
                }
            }
            ClientMessage::Cancel { .. } => {
                // 向主 Runtime 发送 Cancel（取消当前 round）
                let _ = state.agent_handle.send(AgentMessage::Cancel).await;
                info!("Agent cancellation requested");
            }
        }
```

### Step 4：同时移除 `handle_socket` 中的 `user_ctx` 变量和 `user_id_opt`

`user_ctx` 在 ws_handler 中仍用于注入，但 `handle_socket` 函数签名可以简化。当前：
```rust
async fn handle_socket(socket: WebSocket, state: Arc<AppState>, user_ctx: UserContext) {
```

由于 channels 不再需要 user 路由，可保留签名不变（避免改动 ws_handler）。但删除 `handle_socket` 函数体内的：
```rust
        // Convert user_id from Option<String> to Option<UserId>
        let user_id_opt = user_ctx.user_id.as_ref().map(|s| UserId::from_string(s));
```

（删除 line 137）

### Step 5：编译验证

```bash
cargo check -p octo-server
```

Expected: 零错误。可能有 unused import 警告（`UserContext`，来自 `user_ctx`），若有则删除相应 import。

### Step 6：全工作区编译

```bash
cargo check --workspace
```

Expected: 零错误

### Step 7：运行测试

```bash
cargo test --workspace
```

Expected: 所有测试通过（测试不覆盖 ws.rs，主要验证 engine 层无回归）

### Step 8：Commit

```bash
git add crates/octo-server/src/ws.rs
git commit -m "refactor(ws): simplify to pure channel layer — use state.agent_handle directly"
```

---

## Task 5：验证与收尾

### Step 1：全工作区最终编译

```bash
cargo build --workspace 2>&1 | tail -5
```

Expected: `Finished` 无错误

### Step 2：运行全量测试

```bash
cargo test --workspace 2>&1 | tail -20
```

Expected: `X tests passed`，0 failed

### Step 3：集成测试（手动，运行服务器后验证）

启动服务器：
```bash
make dev
```

打开两个浏览器窗口连接 `http://localhost:5180`：

**验证点 A — 共享 session_id**：
- 窗口 A 连接后收到 `{"type":"session_created","session_id":"xxx"}`
- 窗口 B 连接后收到相同的 `session_id`（共享主 Runtime）

**验证点 B — 消息广播**：
- 窗口 A 发送消息 → 窗口 A 收到 TextDelta 流 → 窗口 B **同时收到** 完整事件流
- 窗口 B 发送消息 → 窗口 B 收到 TextDelta 流 → 窗口 A **同时收到** 完整事件流

**验证点 C — ws.rs 无 AgentSupervisor 引用**：
```bash
grep -n "agent_supervisor" crates/octo-server/src/ws.rs
```
Expected: 无输出（ws.rs 不再引用 AgentSupervisor）

**验证点 D — ws.rs 无 SessionId 引用**：
```bash
grep -n "SessionId\|get_or_spawn\|create_session" crates/octo-server/src/ws.rs
```
Expected: 无输出

### Step 4：更新 checkpoint

```bash
# 更新 .checkpoint.json
```

更新 `docs/plans/.checkpoint.json`：
```json
{
  "plan_file": "docs/plans/2026-03-03-d5-singleton-agent-channel.md",
  "phase": "completed",
  "created_at": "2026-03-03T16:00:00+08:00",
  "updated_at": "2026-03-03T17:00:00+08:00",
  "completed_tasks": ["T1", "T2", "T3", "T4", "T5"],
  "current_task": null,
  "execution_mode": "subagent-driven-development",
  "phase_name": "D5: AgentSupervisor 主 Runtime + Channels 解耦",
  "notes": "D5 完成。主 Runtime 在 server 启动时预热，所有 channel 共享同一对话历史。ws.rs 为纯 channel 层，不再持有 AgentSupervisor 引用。"
}
```

### Step 5：最终 Commit

```bash
git add docs/plans/.checkpoint.json
git commit -m "checkpoint: D5 complete - primary AgentRuntime, channels decoupled from AgentSupervisor"
```

---

## 解耦验证图

```
main.rs
  └─ agent_supervisor.start_primary(primary_session_id, ...)
       └─ returns AgentRuntimeHandle { tx, broadcast_tx, session_id }
            └─ 存入 AppState.agent_handle

ws.rs（PC Client）              Telegram（未来）
  └─ state.agent_handle            └─ state.agent_handle
       ├─ .subscribe()                  ├─ .subscribe()
       └─ .send(UserMessage)            └─ .send(UserMessage)
                  ↓
         AgentRuntime（主，持续运行）
                  ↓
         broadcast::Sender<AgentEvent>
           ├─ → ws.rs rx (PC)
           ├─ → ws.rs rx (Mobile)
           └─ → Telegram rx（未来）
```

channels 只知道 `AgentRuntimeHandle`。AgentSupervisor 在 handle 取出后对所有 channels 完全透明。
