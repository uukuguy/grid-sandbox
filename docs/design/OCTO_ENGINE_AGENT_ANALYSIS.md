# octo-engine 自主智能体架构分析文档

> 分析日期: 2026-03-01
> 源码版本: analysis 分支

---

## 1. 概述

本文档详细分析 octo-engine 核心架构，解答以下问题：
1. octo-engine 是否是一个完整的自主智能体？
2. Agent Loop 的完整运行过程是什么？
3. octo-server 与 octo-engine 中 Session 的关系是什么？
4. 外部应用如何调用 octo-engine？

---

## 2. octo-engine 是否是完整的自主智能体？

**结论：是的，octo-engine 是一个完整的自主智能体框架。**

### 2.1 核心证据

| 组件 | 实现 | 位置 |
|------|------|------|
| LLM 推理引擎 | Provider trait (支持 Anthropic/OpenAI) | `providers/traits.rs` |
| 工具执行框架 | ToolRegistry + Tool trait | `tools/mod.rs`, `tools/traits.rs` |
| 多轮对话循环 | MAX_ROUNDS=30 | `agent/loop_.rs:21` |
| 上下文管理 | ContextBudgetManager 4 阶段降级 | `context/budget.rs` |
| 循环防护 | LoopGuard (重复调用+乒乓+断路器) | `agent/loop_guard.rs` |
| 记忆系统 | WorkingMemory + MemoryStore | `memory/traits.rs`, `memory/store_traits.rs` |
| 错误重试 | RetryPolicy 指数退避 | `providers/retry.rs` |
| 可观测性 | EventBus + ToolRecorder | `event/bus.rs`, `tools/recorder.rs` |

### 2.2 架构总览

```
┌─────────────────────────────────────────────────────────────────────┐
│                         AgentLoop                                    │
│  (crates/octo-engine/src/agent/loop_.rs)                           │
│                                                                     │
│  ┌──────────────┐  ┌──────────────┐  ┌────────────────────────┐   │
│  │  Provider    │  │ ToolRegistry │  │    Memory System     │   │
│  │  (LLM)       │  │ (16 tools)   │  │ (Working + Persistent)│   │
│  └──────────────┘  └──────────────┘  └────────────────────────┘   │
│                                                                     │
│  ┌──────────────┐  ┌──────────────┐  ┌────────────────────────┐   │
│  │  LoopGuard    │  │ ContextBudget│  │    EventBus          │   │
│  │  (循环检测)   │  │ (上下文管理)  │  │    (可观测性)        │   │
│  └──────────────┘  └──────────────┘  └────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 3. Agent Loop 完整运行过程

### 3.1 入口：WebSocket 消息处理

位置：`crates/octo-server/src/ws.rs` 第 128-214 行

```rust
// 1. 接收客户端消息
match client_msg {
    ClientMessage::SendMessage { session_id, content } => {
        // 2. 获取或创建 Session
        let session = state.sessions.get_or_create_session(...).await;

        // 3. 从 SessionStore 获取历史消息
        let mut messages = state.sessions.get_messages(&session.session_id).await;

        // 4. 添加用户新消息
        let user_msg = ChatMessage::user(&content);
        state.sessions.push_message(&session.session_id, user_msg).await;

        // 5. 创建 AgentLoop
        let mut agent_loop = AgentLoop::new(provider, tools, memory)
            .with_memory_store(store)
            .with_model(model);

        // 6. 调用 run()
        agent_loop.run(&session_id, &user_id, &sandbox_id, &mut messages, tx, tool_ctx).await;
    }
}
```

### 3.2 AgentLoop::run() 核心流程

位置：`crates/octo-engine/src/agent/loop_.rs` 第 117-500 行

```
┌──────────────────────────────────────────────────────────────────────┐
│ AgentLoop::run()                                                     │
├──────────────────────────────────────────────────────────────────────┤
│                                                                      │
│ 1. 断言: model 已设置 (line 127)                                     │
│                                                                      │
│ 2. 构建 System Prompt (line 135-148)                                │
│    memory.compile() → XML                                           │
│    ContextBuilder.build_system_prompt()                              │
│                                                                      │
│ 3. 获取工具规格 (line 149)                                           │
│    tools.specs() → Vec<ToolSpec>                                    │
│                                                                      │
│ 4. for round in 0..MAX_ROUNDS (30):                                │
│    ├── 4.1 上下文预算检查 (line 155-181)                             │
│    │       ContextBudgetManager.compute_degradation_level()           │
│    │       → SoftTrim / AutoCompaction / OverflowCompaction         │
│    │       → ContextPruner.apply()                                 │
│    │       → MemoryFlusher::flush() (必要时)                       │
│    │                                                                   │
│    ├── 4.2 构建请求 (line 183-191)                                  │
│    │       CompletionRequest {                                      │
│    │           model, system, messages, tools, stream: true        │
│    │       }                                                        │
│    │                                                                   │
│    ├── 4.3 LLM 调用 + 重试 (line 193-240)                          │
│    │       provider.stream() with RetryPolicy                        │
│    │                                                                   │
│    ├── 4.4 流式事件处理 (line 247-336)                              │
│    │       ├── TextDelta → 实时文本                                 │
│    │       ├── ThinkingDelta → 思考过程                             │
│    │       ├── ToolUseStart/InputDelta/Complete → 工具调用         │
│    │       └── MessageStop → 判断结束条件                          │
│    │                                                                   │
│    ├── 4.5 工具执行 (line 377-478)                                  │
│    │       ├── LoopGuard.record_call() → 检测循环                   │
│    │       ├── tools.get(name).execute() → 执行工具                 │
│    │       ├── maybe_trim_tool_result() → 大结果裁剪                │
│    │       └── 构建 ToolResult 消息                                 │
│    │                                                                   │
│    └── 4.6 消息更新 (line 480-483)                                  │
│            messages.push(assistant_message)  // tool_use            │
│            messages.push(user_message)        // tool_result        │
│                                                                      │
│ 5. 结束条件:                                                        │
│    ├── stop_reason != ToolUse (无需工具调用)                        │
│    ├── tool_uses.is_empty()                                         │
│    ├── MAX_ROUNDS (30) 超限                                        │
│    └── LoopGuard 触发                                               │
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘
```

### 3.3 关键数据流

```
User Message (WebSocket)
    ↓
SessionStore.get_messages()
    ↓
AgentLoop::run()
    ↓
┌─────────────────────────────────────────────────────┐
│ 每轮循环:                                           │
│                                                     │
│ messages + system_prompt → Provider.stream()        │
│         ↓                                          │
│   流式事件处理                                      │
│         ↓                                          │
│   有 ToolUse? ──YES──→ 执行工具 → 添加 ToolResult  │
│     │                                               │
│     NO                                              │
│     ↓                                               │
│   返回最终文本                                      │
└─────────────────────────────────────────────────────┘
    ↓
messages 更新回 SessionStore
    ↓
AgentEvent 推送回 WebSocket
```

### 3.4 上下文预算管理 (4 阶段降级)

位置：`crates/octo-engine/src/context/budget.rs`

| 使用率 | 级别 | 动作 |
|--------|------|------|
| <60% | None | 无 |
| 60-70% | SoftTrim | 工具结果头尾裁剪 |
| 70-90% | AutoCompaction | 保留最近 10 条消息 |
| >90% | OverflowCompaction | 保留 4 条 + Memory Flush |

### 3.5 循环检测 (LoopGuard)

位置：`crates/octo-engine/src/agent/loop_guard.rs`

检测三种循环模式：
1. **RepetitiveCall**: 同一工具+参数调用 ≥5 次
2. **PingPong**: A-B-A-B 模式检测
3. **CircuitBreaker**: 全局调用 ≥30 次

### 3.6 内置工具 (16 个)

| 类别 | 工具 | 功能 |
|------|------|------|
| 文件 | `file_read`, `file_write`, `file_edit` | 读写编辑文件 |
| 搜索 | `grep`, `glob`, `find` | 代码搜索 |
| 执行 | `bash` | shell 命令 |
| 网络 | `web_fetch`, `web_search` | HTTP 请求/搜索 |
| 记忆 | `memory_store`, `memory_search`, `memory_recall`, `memory_update`, `memory_forget` | 持久化记忆 |

---

## 4. octo-server 与 octo-engine 中 Session 的关系

### 4.1 结论：它们是同一个东西

`octo-server` 只是从 `octo-engine` re-export 了 session 模块：

```rust
// crates/octo-server/src/session.rs:1-3
pub use octo_engine::session::*;
```

### 4.2 数据流关系

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ octo-server (API Server)                                                   │
│                                                                           │
│  ws.rs:                                                                  │
│    let session = state.sessions.get_or_create_session(...).await;        │
│    let mut messages = state.sessions.get_messages(&session_id).await;     │
│    state.sessions.push_message(&session_id, user_msg).await;             │
│                                                                           │
│  state.rs:                                                               │
│    pub sessions: Arc<dyn SessionStore>,  ← 注入依赖                      │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ octo-engine (Core Engine)                                                │
│                                                                           │
│  session/mod.rs:                                                         │
│    - SessionData { session_id, user_id, sandbox_id }                     │
│    - SessionStore trait (接口)                                           │
│    - SqliteSessionStore (实现) ← 持久化到 SQLite                         │
│    - InMemorySessionStore (实现) ← 内存缓存                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 4.3 SessionStore 接口

位置：`crates/octo-engine/src/session/mod.rs`

```rust
#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn create_session(&self) -> SessionData;
    async fn get_session(&self, session_id: &SessionId) -> Option<SessionData>;
    async fn get_messages(&self, session_id: &SessionId) -> Option<Vec<ChatMessage>>;
    async fn push_message(&self, session_id: &SessionId, message: ChatMessage);
    async fn set_messages(&self, session_id: &SessionId, messages: Vec<ChatMessage>);
    async fn list_sessions(&self, limit: usize, offset: usize) -> Vec<SessionSummary>;
}
```

### 4.4 两种实现

| 实现 | 存储 | 用途 |
|------|------|------|
| `SqliteSessionStore` | SQLite + DashMap 缓存 | 生产环境（持久化） |
| `InMemorySessionStore` | DashMap | 开发/测试环境 |

### 4.5 Session 生命周期

```
1. WebSocket 连接
        ↓
2. ClientMessage::SendMessage
        ↓
3. state.sessions.create_session() 或 get_session()
        ↓
4. 获取历史 messages
        ↓
5. AgentLoop.run() 执行对话
        ↓
6. 每轮: state.sessions.push_message() 追加新消息
        ↓
7. Agent 完成，messages 已更新
```

---

## 5. 外部应用调用 octo-engine 完整框架

### 5.1 依赖关系图

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        外部应用 (如 octo-server)                           │
│                                                                         │
│  1. 创建 Provider (LLM)                                                  │
│  2. 创建 SessionStore (会话存储)                                          │
│  3. 创建 WorkingMemory (工作记忆)                                        │
│  4. 创建 MemoryStore (持久化记忆)                                        │
│  5. 创建 ToolRegistry (工具注册表)                                       │
│  6. 组合成 AppState                                                      │
│  7. WebSocket 调用 AgentLoop.run()                                      │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                        octo-engine 核心组件                                │
│                                                                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │
│  │  Provider  │  │  Session    │  │  Working    │  │   Tool      │  │
│  │  (LLM)     │  │  Store      │  │  Memory     │  │   Registry  │  │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘  │
│                                                                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │
│  │  Context    │  │  LoopGuard  │  │  EventBus   │  │   MCP       │  │
│  │  Budget     │  │  (循环检测) │  │  (事件)     │  │   Manager   │  │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘  │
│                                                                         │
│                         AgentLoop.run()                                  │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 5.2 完整初始化代码

位置：`crates/octo-server/src/main.rs`

```rust
use octo_engine::{
    create_provider, default_tools, register_memory_tools,
    Database, MemoryStore, SessionStore, WorkingMemory,
    SqliteMemoryStore, SqliteSessionStore, SqliteWorkingMemory,
    AgentLoop, Provider, ToolRegistry, ToolExecutionRecorder,
};

// ============ 1. 创建 LLM Provider ============
let provider: Arc<dyn Provider> = Arc::from(
    create_provider("anthropic", api_key, base_url)
);

// ============ 2. 创建数据库 ============
let db = Database::open(&db_path).await?;
let conn = db.conn().clone();

// ============ 3. 创建 Session Store ============
let sessions: Arc<dyn SessionStore> = Arc::new(
    SqliteSessionStore::new(conn.clone()).await?
);

// ============ 4. 创建 Working Memory (Layer 0) ============
let memory: Arc<dyn WorkingMemory> = Arc::new(
    SqliteWorkingMemory::new(conn.clone()).await?
);

// ============ 5. 创建 Persistent Memory Store (Layer 2) ============
let memory_store: Arc<dyn MemoryStore> = Arc::new(
    SqliteMemoryStore::new(conn.clone())
);

// ============ 6. 创建 Tool Registry ============
let mut tools = default_tools();                              // 内置工具
register_memory_tools(&mut tools, memory_store.clone(), provider.clone()); // 记忆工具

// 注册 Skill 工具
for skill in skill_registry.invocable_skills() {
    tools.register(SkillTool::new(skill));
}
let tools = Arc::new(tools);

// ============ 7. 创建 Tool Recorder (可选) ============
let recorder = Arc::new(ToolExecutionRecorder::new(conn.clone()));
```

### 5.3 调用 AgentLoop

位置：`crates/octo-server/src/ws.rs`

```rust
// 在 ws.rs 中 (WebSocket 处理)
async fn handle_message(state: &AppState, content: &str) {
    // 1. 获取或创建 Session
    let session = state.sessions.create_session().await;

    // 2. 获取历史消息
    let mut messages = state.sessions.get_messages(&session.session_id)
        .await
        .unwrap_or_default();

    // 3. 添加用户消息
    let user_msg = ChatMessage::user(content);
    state.sessions.push_message(&session.session_id, user_msg).await;

    // 4. 创建 AgentLoop
    let mut agent_loop = AgentLoop::new(
        state.provider.clone(),
        state.tools.clone(),
        state.memory.clone(),
    )
    .with_memory_store(state.memory_store.clone())
    .with_model(state.model.clone().unwrap())
    .with_recorder(state.recorder.clone());

    // 5. 创建事件通道
    let (tx, mut rx) = broadcast::channel::<AgentEvent>(256);

    // 6. 运行 Agent
    agent_loop.run(
        &session.session_id,
        &session.user_id,
        &session.sandbox_id,
        &mut messages,
        tx,
        tool_ctx,  // ToolContext { sandbox_id, working_dir }
    ).await?;

    // 7. 更新会话消息 (AgentLoop 会修改 messages)
    state.sessions.set_messages(&session.session_id, messages).await;
}
```

### 5.4 关键接口总结

| 组件 | 接口 | 必需 | 说明 |
|------|------|------|------|
| **Provider** | `trait Provider` | ✅ | LLM 调用 (Anthropic/OpenAI) |
| **SessionStore** | `trait SessionStore` | ✅ | 会话+消息持久化 |
| **WorkingMemory** | `trait WorkingMemory` | ✅ | 上下文编译 |
| **MemoryStore** | `trait MemoryStore` | 可选 | 长期记忆 |
| **ToolRegistry** | `struct ToolRegistry` | ✅ | 工具注册表 |
| **ToolExecutionRecorder** | `struct` | 可选 | 执行记录 |

### 5.5 事件驱动架构

AgentLoop 通过 `broadcast::Sender<AgentEvent>` 推送事件：

```rust
pub enum AgentEvent {
    TextDelta { text: String },        // 实时文本片段
    TextComplete { text: String },     // 文本完成
    ThinkingDelta { text: String },    // 思考过程
    ThinkingComplete { text: String }, // 思考完成
    ToolStart { tool_id, tool_name, input },  // 工具开始
    ToolResult { tool_id, output, success },  // 工具结果
    ToolExecution { execution },      // 执行记录
    TokenBudgetUpdate { budget },     // 配额更新
    Error { message: String },        // 错误
    Done,                             // 完成
}
```

### 5.6 完整调用链路

```
外部应用
    │
    ▼
┌─────────────────────────────────────────────────────────────┐
│ 1. 初始化阶段                                              │
│    create_provider() → Provider                            │
│    SqliteSessionStore → SessionStore                      │
│    SqliteWorkingMemory → WorkingMemory                    │
│    default_tools() + register_memory_tools() → ToolRegistry│
└─────────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────────┐
│ 2. 请求处理阶段                                            │
│    WebSocket/Http → ClientMessage::SendMessage            │
│    sessions.get_or_create_session()                        │
│    sessions.get_messages()                                │
└─────────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────────┐
│ 3. Agent 执行阶段                                          │
│    AgentLoop::new().with_model().with_memory_store()       │
│    agent_loop.run(session_id, user_id, sandbox_id,        │
│                   messages, tx, tool_ctx)                  │
│         │                                                  │
│         ├── 循环 (最多30轮)                                │
│         │    ├── ContextBudgetManager 检查                │
│         │    ├── Provider.stream() → LLM 调用            │
│         │    ├── 流式事件处理                              │
│         │    ├── ToolRegistry.execute() 工具执行         │
│         │    └── messages.push() 更新历史                 │
│         │                                                  │
│         └── 返回完成事件                                   │
└─────────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────────┐
│ 4. 结果持久化                                              │
│    sessions.set_messages() → 写回 SQLite                  │
│    AgentEvent → WebSocket 推送前端                        │
└─────────────────────────────────────────────────────────────┘
```

### 5.7 最小可用示例

```rust
use octo_engine::{
    AgentLoop, Provider, ToolRegistry, WorkingMemory,
    create_provider, default_tools, InMemorySessionStore,
    InMemoryWorkingMemory, SessionStore, ChatMessage,
};
use tokio::sync::broadcast;

#[tokio::main]
async fn main() {
    // 最小依赖
    let provider = create_provider("anthropic", "api-key", None);
    let tools = default_tools();
    let memory = InMemoryWorkingMemory::new();
    let sessions = InMemorySessionStore::new();

    // 创建会话
    let session = sessions.create_session().await;
    sessions.push_message(&session.session_id, ChatMessage::user("Hello!")).await;
    let mut messages = sessions.get_messages(&session.session_id).await.unwrap();

    // 运行 Agent
    let mut agent = AgentLoop::new(
        Arc::new(provider),
        Arc::new(tools),
        Arc::new(memory),
    )
    .with_model("claude-3-5-sonnet-20241022");

    let (tx, _rx) = broadcast::channel(256);
    agent.run(
        &session.session_id,
        &session.user_id,
        &session.sandbox_id,
        &mut messages,
        tx,
        tool_ctx,
    ).await;
}
```

---

## 6. 总结

octo-engine 是一个**功能完整的自主智能体框架**，具备现代 Agent 系统的所有核心组件：

- ✅ LLM 推理引擎 (Provider trait，支持 Anthropic/OpenAI)
- ✅ 工具执行框架 (ToolRegistry + Tool trait)
- ✅ 多轮对话循环 (MAX_ROUNDS=30)
- ✅ 上下文管理 (ContextBudgetManager 4 阶段降级)
- ✅ 循环防护 (LoopGuard: 重复调用 + 乒乓 + 断路器)
- ✅ 记忆系统 (WorkingMemory + MemoryStore)
- ✅ 错误重试 (RetryPolicy 指数退避)
- ✅ 可观测性 (EventBus + ToolRecorder)
- ✅ 流式输出 (WebSocket 实时推送)

外部应用只需实现/实例化 4 个核心 Trait (Provider, SessionStore, WorkingMemory, ToolRegistry)，然后调用 `AgentLoop.run()` 即可使用完整的智能体功能。

---

*文档生成时间: 2026-03-01*
*源码位置: crates/octo-engine/*
