# Phase AU — 自主运行模式完整实现

> 目标：补齐 AUTONOMOUS_MODE_DESIGN.md 剩余 5 个缺口，实现完整的企业级自主运行能力。
> 日期：2026-04-03
> 依据：AUTONOMOUS_MODE_DESIGN.md 差距分析（Phase AQ 基础上的 Phase 2+3 补全）
> 基线：Phase AQ 已完成基础自主循环（Config/State/Status/SleepTool/Tick/AgentEvent/AuditLog）
> 执行策略：3 Wave，Wave 1 三任务并行，Wave 2 两任务并行

---

## 一、设计决策

| 决策点 | 选项 | 决定 | 理由 |
|--------|------|------|------|
| harness 多路监听 | A:executor 转发 / **B:AutonomousControl 直传** / C:扩展 cancel_token | B | 最贴合设计文档，与 executor Pause/Resume 消息兼容 |
| AutonomousScheduler 定位 | **A:AgentRuntime 字段** / B:独立服务 | A | 设计文档架构图明确放在 Runtime 内 |
| Session pause/resume API | **A:新增 session 级端点** / B:复用 agent 级 | A | 设计文档明确要求 session 级接口 |
| WS 事件转发 | **A:新增 ServerMessage 变体** / B:通用 JSON 透传 | A | 类型安全，前端可精确匹配 |
| Cron 触发复用 | A:复用现有 scheduler/ / **B:独立** | B | AutonomousScheduler 管理自主 session，现有 scheduler 是通用 cron |

---

## 二、已完成基础（Phase AQ/AR 成果）

| 组件 | 文件 | 状态 |
|------|------|------|
| `AutonomousConfig` + `AutonomousTrigger` | `agent/autonomous.rs` | ✅ |
| `AutonomousState` + `AutonomousStatus` | `agent/autonomous.rs` | ✅ |
| `SleepTool` | `tools/sleep.rs` | ✅ 已注册 |
| `AUTONOMOUS_PROMPT` + `with_autonomous_mode()` | `context/system_prompt.rs` | ✅ |
| `AgentEvent::Autonomous*` (5 变体) | `agent/events.rs` | ✅ |
| `AgentLoopConfig.autonomous` | `agent/loop_config.rs` | ✅ |
| harness 基础 tick 循环 | `agent/harness.rs:1229+` | ✅ 基础版 |
| Executor Pause/Resume/UserPresence 消息 | `agent/executor.rs` | ✅ |
| TriggerSource trait + Channel/Polling | `agent/autonomous_trigger.rs` | ✅ |
| AutonomousAuditLog | `agent/autonomous_audit.rs` | ✅ |
| Webhook API 骨架 | `octo-server/api/autonomous.rs` | ⚠️ placeholder |

---

## 三、缺口清单（5 项）

| # | 缺口 | 设计文档章节 | 影响 |
|---|------|-------------|------|
| G1 | harness `tokio::select!` 缺少用户消息注入和暂停信号分支 | §二 Harness 集成 | 用户无法在自主模式下实时干预 |
| G2 | `AutonomousScheduler` 未实现 | §二 核心架构 / §五 集成 | 无法管理多自主 session 生命周期 |
| G3 | WebSocket 自主事件未转发 | §四 人工干预点 | 前端看不到自主模式状态变化 |
| G4 | Session 级 pause/resume REST API 缺失 | §四 人工干预点 | 用户无法通过 API 暂停/恢复自主 session |
| G5 | Webhook trigger handler 是 placeholder | §五 集成 / §二 触发模式 | Webhook 触发不可用 |

---

## 四、依赖图

```
G1 AutonomousControl + harness select!  ──┐
                                           ├── 零依赖，可并行 (Wave 1)
G2 AutonomousScheduler                   ──┤
                                           │
G3 WS 事件转发                           ──┘
        │
G4 Session pause/resume API ───────────── 依赖 G1 (AutonomousControl 通道)
        │
G5 Webhook trigger 接线 ──────────────── 依赖 G2 (AutonomousScheduler)
```

---

## 五、Wave 执行顺序

```
时间 →
─────────────────────────────────────────────────
Wave 1  │ G1 + G2 + G3 并行              │ 零依赖
────────┤                                  ├────
Wave 2  │ G4 + G5 并行                   │ G4←G1, G5←G2
─────────────────────────────────────────────────
```

---

## 六、任务详细设计

### Wave 1：核心基础设施（零依赖，可并行）

#### G1 — AutonomousControl + harness `tokio::select!` 完整分支 (~100 行)

**问题**: 当前 harness 自主循环只有 `tokio::time::sleep` + cancel_token 检查，是顺序执行。设计要求同时监听 sleep、用户消息注入、暂停信号三路。

**新增数据结构** (`agent/autonomous.rs`):

```rust
/// Control channels for autonomous mode, passed into AgentLoopConfig.
pub struct AutonomousControl {
    /// Notified when pause is requested.
    pub pause_signal: Arc<Notify>,
    /// Notified when resume after pause.
    pub resume_signal: Arc<Notify>,
    /// User messages injected during autonomous sleep.
    pub user_msg_rx: mpsc::Receiver<ChatMessage>,
    /// Sender side (held by executor/API layer).
    pub user_msg_tx: mpsc::Sender<ChatMessage>,
}

impl AutonomousControl {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(16);
        Self {
            pause_signal: Arc::new(Notify::new()),
            resume_signal: Arc::new(Notify::new()),
            user_msg_rx: rx,
            user_msg_tx: tx,
        }
    }
}
```

**AgentLoopConfig 新增字段** (`agent/loop_config.rs`):

```rust
pub autonomous_control: Option<AutonomousControl>,
```

**harness.rs 修改** — 替换当前顺序 sleep 为完整 `tokio::select!`:

```rust
// 当前代码 (harness.rs:1239-1257):
//   tokio::time::sleep(sleep_dur).await;
//   if cancel_token.is_cancelled() { ... } else { inject tick }
//
// 替换为:
let sleep_dur_duration = std::time::Duration::from_secs(sleep_dur);
tokio::select! {
    // 路径 1: 正常 sleep 完成 → tick
    _ = tokio::time::sleep(sleep_dur_duration) => {
        let _ = tx.send(AgentEvent::AutonomousTick { round: state.rounds_completed }).await;
        let tick_msg = if state.user_online {
            "<tick> Autonomous check-in. User is online."
        } else {
            "<tick> Autonomous check-in. Continue working."
        };
        messages.push(ChatMessage::user(tick_msg));
        continue;
    }
    // 路径 2: 用户消息到达 → 注入并继续
    msg = async {
        if let Some(ref mut ctrl) = config.autonomous_control {
            ctrl.user_msg_rx.recv().await
        } else {
            std::future::pending().await
        }
    } => {
        if let Some(user_msg) = msg {
            messages.push(user_msg);
            continue;
        }
    }
    // 路径 3: 暂停信号 → 等待恢复
    _ = async {
        if let Some(ref ctrl) = config.autonomous_control {
            ctrl.pause_signal.notified().await
        } else {
            std::future::pending().await
        }
    } => {
        state.status = AutonomousStatus::Paused;
        let _ = tx.send(AgentEvent::AutonomousPaused).await;
        // 等待恢复
        if let Some(ref ctrl) = config.autonomous_control {
            ctrl.resume_signal.notified().await;
        }
        state.status = AutonomousStatus::Running;
        let _ = tx.send(AgentEvent::AutonomousResumed).await;
        continue;
    }
    // 路径 4: 取消
    _ = config.cancel_token.cancelled() => {
        info!(session = %config.session_id, "Autonomous: cancelled during sleep");
    }
}
```

**Executor 接线** (`agent/executor.rs`):
- `AgentMessage::Chat` 消息转发到 `autonomous_control.user_msg_tx`
- `AgentMessage::Pause` 触发 `autonomous_control.pause_signal.notify_one()`
- `AgentMessage::Resume` 触发 `autonomous_control.resume_signal.notify_one()`
- `AgentMessage::UserPresence(online)` 更新 `auto_state.user_online`（通过 shared Arc<AtomicBool>）

**测试** (~40 行):
- `test_autonomous_control_new` — 验证通道创建
- `test_autonomous_select_user_interrupt` — 模拟用户消息注入
- `test_autonomous_select_pause_resume` — 模拟暂停/恢复流程

---

#### G2 — AutonomousScheduler (~100 行)

**新增文件**: `crates/octo-engine/src/agent/autonomous_scheduler.rs`

**职责**:
1. 维护活跃自主 session 列表
2. 接收 TriggerEvent → 调用回调创建/恢复自主 session
3. 提供查询接口（list/status/stop）

```rust
use dashmap::DashMap;
use octo_types::SessionId;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::{info, warn};

use super::autonomous::{AutonomousConfig, AutonomousState, AutonomousStatus};
use super::autonomous_trigger::{TriggerEvent, TriggerListener};

/// Manages multiple autonomous agent sessions.
pub struct AutonomousScheduler {
    /// Active autonomous sessions indexed by session ID.
    sessions: DashMap<SessionId, AutonomousState>,
    /// Background trigger listener handles.
    listener_handles: Vec<JoinHandle<()>>,
}

impl AutonomousScheduler {
    pub fn new() -> Self { ... }

    /// Register a session as autonomous.
    pub fn register(&self, state: AutonomousState) { ... }

    /// Unregister a session.
    pub fn unregister(&self, session_id: &SessionId) -> Option<AutonomousState> { ... }

    /// Get session state.
    pub fn get(&self, session_id: &SessionId) -> Option<AutonomousState> { ... }

    /// List all active autonomous sessions.
    pub fn list(&self) -> Vec<AutonomousState> { ... }

    /// Update session state.
    pub fn update_status(&self, session_id: &SessionId, status: AutonomousStatus) { ... }

    /// Start trigger listener with callback.
    pub fn start_triggers<F>(&mut self, listener: TriggerListener, callback: Arc<F>)
    where F: Fn(TriggerEvent) + Send + Sync + 'static { ... }

    /// Shutdown all listeners.
    pub async fn shutdown(&mut self) { ... }

    /// Number of active sessions.
    pub fn active_count(&self) -> usize { ... }
}
```

**AgentRuntime 集成** (`agent/runtime.rs`):

```rust
// 新增字段
autonomous_scheduler: AutonomousScheduler,

// 新增方法
pub fn autonomous_scheduler(&self) -> &AutonomousScheduler { ... }
```

**测试** (~30 行):
- `test_scheduler_register_unregister`
- `test_scheduler_list_and_count`
- `test_scheduler_update_status`

---

#### G3 — WebSocket 自主事件转发 (~50 行)

**修改文件**: `crates/octo-server/src/ws.rs`

**新增 ServerMessage 变体**:

```rust
// 在 ServerMessage enum 中新增:
AutonomousSleeping { session_id: String, duration_secs: u64 },
AutonomousTick { session_id: String, round: u32 },
AutonomousPaused { session_id: String },
AutonomousResumed { session_id: String },
AutonomousExhausted { session_id: String, reason: String },
```

**event match 新增分支** (在 ws.rs 的 `match event` 中):

```rust
AgentEvent::AutonomousSleeping { duration_secs } => ServerMessage::AutonomousSleeping {
    session_id: sid_str.clone(),
    duration_secs,
},
AgentEvent::AutonomousTick { round } => ServerMessage::AutonomousTick {
    session_id: sid_str.clone(),
    round,
},
AgentEvent::AutonomousPaused => ServerMessage::AutonomousPaused {
    session_id: sid_str.clone(),
},
AgentEvent::AutonomousResumed => ServerMessage::AutonomousResumed {
    session_id: sid_str.clone(),
},
AgentEvent::AutonomousExhausted { reason } => ServerMessage::AutonomousExhausted {
    session_id: sid_str.clone(),
    reason,
},
```

**测试**: ServerMessage 序列化测试 (~20 行)

---

### Wave 2：API 接线（依赖 Wave 1）

#### G4 — Session 级 pause/resume REST API (~50 行)

**依赖**: G1 (AutonomousControl)

**修改文件**: `crates/octo-server/src/api/sessions.rs`

**新增端点**:
- `POST /api/v1/sessions/{id}/pause` — 暂停自主 session
- `POST /api/v1/sessions/{id}/resume` — 恢复自主 session

**实现路径**:
1. 通过 `AppState.runtime` 获取 session 对应的 executor handle
2. 调用 `executor_handle.pause()` / `executor_handle.resume()`
3. 更新 `AutonomousScheduler` 中的 session 状态

```rust
async fn pause_session(
    State(s): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let session_id = SessionId::from_string(&id);
    // 1. Find executor handle for this session
    // 2. Send pause signal
    // 3. Update scheduler state
    // 4. Return status
}

async fn resume_session(
    State(s): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // Similar to pause but sends resume
}
```

**路由注册**: 在 `api/mod.rs` 的 sessions router 中添加两个路由。

**测试** (~20 行):
- `test_pause_resume_session_api` — 验证端点响应格式

---

#### G5 — Webhook trigger handler 接线 (~30 行)

**依赖**: G2 (AutonomousScheduler)

**修改文件**: `crates/octo-server/src/api/autonomous.rs`

**当前状态**: handler 返回 placeholder JSON，有 TODO 注释。

**接线改动**:

```rust
pub async fn trigger_autonomous(
    State(state): State<Arc<AppState>>,
    Json(body): Json<TriggerRequest>,
) -> impl IntoResponse {
    let session_id = body.session_id
        .map(|s| SessionId::from_string(&s))
        .unwrap_or_else(|| SessionId::from_string(&uuid::Uuid::new_v4().to_string()));

    let config = AutonomousConfig {
        enabled: true,
        max_autonomous_rounds: body.max_rounds.unwrap_or(100),
        ..Default::default()
    };

    // Register with scheduler
    let auto_state = AutonomousState::new(session_id.clone(), config.clone());
    state.runtime.autonomous_scheduler().register(auto_state);

    // TODO(Phase AU-D1): Actually start a session via runtime.start_session()
    // This requires wiring the autonomous config into the session creation flow.
    // For now, registration is complete and the session can be started
    // via the standard session creation API with autonomous config.

    (StatusCode::OK, Json(serde_json::json!({
        "session_id": session_id.as_str(),
        "status": "registered",
        "autonomous": {
            "enabled": true,
            "max_rounds": config.max_autonomous_rounds,
            "idle_sleep_secs": config.idle_sleep_secs,
        },
    })))
}
```

**测试**: 复用现有骨架测试，验证返回 JSON 包含实际 autonomous 配置。

---

## 七、Deferred 项

| ID | 描述 | 条件 | 来源 |
|----|------|------|------|
| AU-D1 | Webhook trigger → 自动创建 session + 启动 executor | 需要 `runtime.start_session()` 支持传入 `AutonomousConfig` | G5 |
| AU-D2 | Cron 触发集成 — TriggerListener → AutonomousScheduler → session 创建 | 需要 AU-D1 先完成 | 设计§二 |
| AU-D3 | MessageQueue 触发实现（Redis/NATS adapter） | 外部依赖，需选型 | AR-D3 |
| AU-D4 | 前端 Autonomous 面板（状态显示、暂停/恢复按钮） | 需 G3 WS 事件先就位 | 设计§四 |
| AU-D5 | 审计日志持久化 — AutonomousAuditLog → SQLite/AuditStorage 写入 | 可选增强 | 设计§四 |

---

## 八、文件变更矩阵

| 文件 | G1 | G2 | G3 | G4 | G5 | 变更类型 |
|------|----|----|----|----|----|----|
| `agent/autonomous.rs` | ✏️ | | | | | 新增 AutonomousControl |
| `agent/autonomous_scheduler.rs` | | 🆕 | | | | 新文件 |
| `agent/loop_config.rs` | ✏️ | | | | | +autonomous_control 字段 |
| `agent/harness.rs` | ✏️ | | | | | tokio::select! 替换 |
| `agent/executor.rs` | ✏️ | | | | | 消息转发接线 |
| `agent/runtime.rs` | | ✏️ | | | | +autonomous_scheduler 字段 |
| `agent/mod.rs` | | ✏️ | | | | pub mod 导出 |
| `octo-server/ws.rs` | | | ✏️ | | | +5 event match 分支 |
| `octo-server/api/sessions.rs` | | | | ✏️ | | +pause/resume handlers |
| `octo-server/api/autonomous.rs` | | | | | ✏️ | 接线替换 placeholder |
| `octo-server/api/mod.rs` | | | | ✏️ | | +路由注册 |

---

## 九、代码量估算

| 任务 | 新增行 | 修改行 | 测试行 |
|------|--------|--------|--------|
| G1 AutonomousControl + harness select! | ~40 | ~50 | ~40 |
| G2 AutonomousScheduler | ~90 | ~15 | ~30 |
| G3 WS 事件转发 | ~25 | ~15 | ~20 |
| G4 Session pause/resume API | ~40 | ~10 | ~20 |
| G5 Webhook trigger 接线 | ~25 | ~15 | ~10 |
| **合计** | **~220** | **~105** | **~120** |
| **总计（含测试）** | | | **~445 行** |

---

## 十、验证标准

### 编译验证
```bash
cargo check --workspace
```

### 单元测试
```bash
cargo test --workspace -- --test-threads=1 autonomous
```

### 功能验证点

| 验证项 | 验证方式 |
|--------|---------|
| harness select! 三路监听 | 单元测试模拟用户消息注入 |
| AutonomousScheduler 多 session | 单元测试 register/list/update |
| WS 事件转发 | ServerMessage 序列化测试 |
| Session pause/resume API | REST 端点响应格式测试 |
| Webhook trigger 接线 | 请求→注册→返回 JSON 测试 |
