# 自主智能体 Agent Loop 架构比较分析

> 分析日期: 2026-03-01

---

## 1. 概述

本文档对本地 `./github.com/` 目录下的多个自主智能体项目的 Agent Loop 实现进行比较分析。

**分析的项目**:
| 项目 | 路径 | 语言 | Agent Loop 实现方式 |
|------|------|------|-------------------|
| octo-engine | octo-sandbox-analysis/crates/octo-engine | Rust | 自主实现 |
| zeroclaw | octo-sandbox/github.com/zeroclaw | Rust | 自主实现 |
| openfang | octo-sandbox/github.com/openfang | Rust | 自主实现 |
| pi_agent_rust | octo-sandbox/github.com/pi_agent_rust | Rust | 自主实现 (最完整) |
| nanoclaw | octo-sandbox/github.com/nanoclaw | TypeScript | Claude Agent SDK 封装 |
| happyclaw | octo-sandbox/github.com/happyclaw | TypeScript | Claude Agent SDK 封装 |
| openclaw | octo-sandbox/github.com/openclaw | TypeScript | pi-agent-core 封装 |
| craft-agents-oss | octo-sandbox/github.com/craft-agents-oss | TypeScript | 基础库 (无 Agent Loop) |

---

## 2. Agent Loop 核心流程对比

### 2.1 流程图对比

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                             octo-engine                                     │
├─────────────────────────────────────────────────────────────────────────────┤
│ 1. 构建 System Prompt (memory.compile())                                  │
│ 2. for round in 0..MAX_ROUNDS (30):                                       │
│    ├─ 上下文预算检查 (ContextBudgetManager)                               │
│    ├─ LLM 流式调用 (Provider.stream())                                    │
│    ├─ 流式事件处理                                                        │
│    ├─ 无工具调用 → 完成                                                   │
│    └─ 有工具调用 → 执行 → 添加 ToolResult → 循环                          │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                               zeroclaw                                     │
├─────────────────────────────────────────────────────────────────────────────┤
│ 1. 构建 System Prompt (skills + tools)                                     │
│ 2. agent_turn() 循环:                                                     │
│    ├─ LLM 调用 (非流式 chat_with_history)                                │
│    ├─ 解析工具调用 (XML 风格 <tool_call>)                                  │
│    ├─ 无工具调用 → 完成                                                   │
│    └─ 有工具调用 → 执行 → 添加 ToolResult → 循环                          │
│    MAX_TOOL_ITERATIONS = 10                                               │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                               openfang                                     │
├─────────────────────────────────────────────────────────────────────────────┤
│ 1. 记忆召回 (向量搜索)                                                    │
│ 2. 构建 System Prompt                                                     │
│ 3. for iteration in 0..max_iterations (50):                             │
│    ├─ 上下文溢出恢复 (recover_from_overflow)                              │
│    ├─ 上下文守卫 (apply_context_guard)                                    │
│    ├─ LLM 调用 (非流式)                                                  │
│    ├─ 处理响应                                                            │
│    └─ 工具调用 → 执行 → 循环                                              │
│ 4. 保存 session + 记忆                                                    │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                            pi_agent_rust                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│ 1. 系统提示 + 历史 + 工具构建                                             │
│ 2. run_loop():                                                            │
│    ├─ 流式 LLM 调用 (StreamExt)                                          │
│    ├─ Steering/FollowUp 消息队列                                          │
│    ├─ Extension 生命周期事件                                              │
│    ├─ 工具并发执行 (MAX_CONCURRENT_TOOLS = 8)                           │
│    ├─ 上下文压缩 (CompactionWorker)                                      │
│    └─ 会话持久化                                                          │
│    max_tool_iterations = 50                                               │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. 关键配置对比

### 3.1 循环控制

| 配置项 | octo-engine | zeroclaw | openfang | pi_agent_rust |
|--------|-------------|----------|----------|---------------|
| **最大迭代次数** | 30 | 10 | 50 (可配置) | 50 |
| **历史消息限制** | 无明确限制 | 50 | 20 | 动态压缩 |
| **并发工具数** | 1 | 1 | 1 | 8 |
| **流式响应** | ✅ | ❌ | ❌ | ✅ |

### 3.2 LoopGuard 配置

| 功能 | octo-engine | zeroclaw | openfang | pi_agent_rust |
|------|-------------|----------|----------|---------------|
| **重复调用检测** | ✅ (≥5次) | ❌ | ✅ (≥5次) | ✅ |
| **乒乓检测** | ✅ | ❌ | ✅ | ✅ |
| **全局断路器** | ✅ (≥30) | ❌ | ✅ (30) | ✅ |
| **结果感知** | ❌ | ❌ | ✅ | ❌ |
| **警告机制** | ❌ | ❌ | ✅ | ❌ |
| **轮询工具处理** | ❌ | ❌ | ✅ (3x) | ❌ |

### 3.3 上下文管理

| 特性 | octo-engine | zeroclaw | openfang | pi_agent_rust |
|------|-------------|----------|----------|---------------|
| **预算系统** | ContextBudgetManager | trim_history | ContextBudget | CompactionWorker |
| **降级策略** | 4级降级 | 简单裁剪 | 百分比计算 | 动态压缩 |
| **向量记忆** | ❌ | ✅ | ✅ | ✅ |
| **记忆自动持久化** | ✅ | ✅ | ✅ | ✅ |

---

## 4. 外部调用框架对比

### 4.1 octo-engine 调用框架

```rust
// 1. 创建依赖
let provider = create_provider("anthropic", api_key, base_url);
let sessions = SqliteSessionStore::new(conn.clone()).await?;
let memory = SqliteWorkingMemory::new(conn.clone()).await?;
let tools = default_tools();

// 2. 创建 AgentLoop
let mut agent = AgentLoop::new(provider, tools, memory)
    .with_model("claude-3-5-sonnet-20241022")
    .with_memory_store(memory_store);

// 3. 运行
agent.run(&session_id, &user_id, &sandbox_id, &mut messages, tx, tool_ctx).await;
```

### 4.2 zeroclaw 调用框架

```rust
// 1. 创建依赖
let provider = create_routed_provider(...);
let mem = create_memory(...);
let tools = all_tools_with_runtime(...);

// 2. 构建 System Prompt
let system_prompt = build_system_prompt(...);
let skills = load_skills(...);

// 3. 运行 agent_turn
let response = agent_turn(provider, &mut history, &tools, observer, model, temp).await;
```

### 4.3 openfang 调用框架

```rust
// 1. 创建 Runtime Kernel
let kernel = Kernel::new(config, runtime, memory, provider).await?;

// 2. 运行 Agent Loop
let result = run_agent_loop(
    kernel,
    session_id,
    manifest,
    state,
    request,
    handler,
).await;
```

### 4.4 pi_agent_rust 调用框架

```rust
// 1. 创建 Agent 实例
let agent = Agent::new(
    provider,
    tools,
    session,
    ExtensionManager::new(),
    config,
);

// 2. 运行
let result = agent.run("user message", |event| {
    // 处理事件
}).await;
```

---

## 5. 特性对比矩阵

| 特性 | octo-engine | zeroclaw | openfang | pi_agent_rust |
|------|-------------|----------|----------|---------------|
| **语言** | Rust | Rust | Rust | Rust |
| **多 Provider** | Anthropic/OpenAI | 多 Provider 路由 | 多 Provider | 多 Provider |
| **MCP 支持** | ✅ | ❌ | ✅ | ✅ |
| **Skill 系统** | ✅ | ✅ | ✅ | Extension |
| **多通道** | WebSocket | CLI | 多通道 | 多通道 |
| **记忆层** | 3层 | 简单 | 向量 | 向量 |
| **事件系统** | EventBus | Observer | Hooks | AgentEvent |
| **安全策略** | ❌ | SecurityPolicy | ✅ | Extension 隔离 |
| **可观测性** | 日志 | Observer | PhaseCallback | 完整事件 |

---

## 6. 优缺点分析

### octo-engine

**优点**:
- 代码简洁，易于理解
- 流式响应支持
- 完善的事件系统
- 4级降级策略

**缺点**:
- LoopGuard 功能较基础
- 无结果感知检测
- 无警告机制

### zeroclaw

**优点**:
- 最简单的实现
- 多 Provider 路由
- 记忆自动持久化
- XML 风格工具调用

**缺点**:
- 非流式响应
- 无 LoopGuard
- 无上下文预算系统
- 迭代次数最低 (10)

### openfang

**优点**:
- 增强版 LoopGuard (结果感知、轮询处理、警告桶)
- 动态 ContextBudget
- 记忆向量搜索
- 多通道支持

**缺点**:
- 非流式响应
- 代码复杂度高

### pi_agent_rust

**优点**:
- 最完整的特性集
- 并发工具执行 (8个)
- Extension 系统
- 完整的消息队列 (Steering/FollowUp)
- 上下文压缩 Worker

**缺点**:
- 代码复杂度最高
- 学习曲线陡峭

---

## 7. TypeScript 项目分析 (SDK/Runtime 封装层)

### 7.1 nanoclaw / happyclaw

**架构**: 使用 `@anthropic-ai/claude-agent-sdk` 在 Docker 容器中运行

```typescript
// container/agent-runner/src/index.ts
import { query, HookCallback } from '@anthropic-ai/claude-agent-sdk';

// 核心流程:
// 1. 从 stdin 读取 ContainerInput (prompt, sessionId, groupFolder)
// 2. 使用 MessageStream 推送用户消息
// 3. 调用 SDK 的 query() 函数
// 4. 流式输出结果到 stdout
```

**特点**:
- Agent Loop 完全由 Claude Agent SDK 处理
- 自己只处理 IPC 消息队列和容器生命周期
- 支持多轮对话通过 IPC 文件

### 7.2 openclaw

**架构**: 基于 pi-agent-core (即 pi_agent_rust) 的上层封装

```typescript
// 核心流程 (来自 docs/concepts/agent-loop.md):
// 1. agent RPC 验证参数，解析 session
// 2. agentCommand 运行 agent:
//    - 加载 skills snapshot
//    - 调用 runEmbeddedPiAgent (pi-agent-core)
// 3. subscribeEmbeddedPiSession 桥接事件
// 4. agent.wait 等待完成
```

**特点**:
- 调用 pi_agent_rust 嵌入式运行时
- 支持 Gateway Hooks + Plugin Hooks
- 完整的生命周期事件 (start/end/error)
- 支持队列模式 (collect/steer/followup)
- 超时控制 (默认 600s)

### 7.3 craft-agents-oss

**结论**: 这是一个基础库项目 (`packages/core` 只包含类型定义)，不包含 Agent Loop 实现。

### 7.4 总结: TypeScript 项目分类

| 项目 | Agent Loop 来源 | 封装层次 |
|------|----------------|----------|
| nanoclaw | Claude Agent SDK | SDK 封装 |
| happyclaw | Claude Agent SDK | SDK 封装 |
| openclaw | pi-agent-core | Runtime 封装 |
| craft-agents-oss | 无 | 基础库 |

---

## 9. 总结

| 项目 | Agent Loop 来源 | 适用场景 | 推荐度 |
|------|----------------|----------|--------|
| **octo-engine** | 自研 | 简单项目、嵌入式 | ⭐⭐⭐⭐ |
| **zeroclaw** | 自研 | CLI 工具、快速原型 | ⭐⭐⭐ |
| **openfang** | 自研 | 生产环境、需要循环保护 | ⭐⭐⭐⭐⭐ |
| **pi_agent_rust** | 自研 | 复杂企业级 | ⭐⭐⭐⭐ |
| **openclaw** | pi-agent-core | 多通道企业应用 | ⭐⭐⭐⭐ |
| **nanoclaw** | Claude SDK | 容器化运行 | ⭐⭐⭐ |
| **happyclaw** | Claude SDK | 容器化运行 | ⭐⭐⭐ |

---

## 10. 关键代码路径

```
octo-engine:
├── crates/octo-engine/src/agent/loop_.rs          (核心循环)
├── crates/octo-engine/src/agent/loop_guard.rs     (循环防护)
└── crates/octo-engine/src/context/                (上下文管理)

zeroclaw:
├── src/agent/loop_.rs                            (核心循环)
├── src/memory/                                    (记忆系统)
└── src/providers/                                 (多 Provider)

openfang:
├── crates/openfang-runtime/src/agent_loop.rs     (核心循环)
├── crates/openfang-runtime/src/loop_guard.rs      (增强循环防护)
├── crates/openfang-runtime/src/context_budget.rs  (上下文预算)
└── crates/openfang-runtime/src/tool_runner.rs     (工具执行)

pi_agent_rust:
├── src/agent.rs                                   (最复杂的主循环)
├── src/extension_*.rs                            (Extension 系统)
├── src/compaction_*.rs                            (上下文压缩)
└── src/session/                                   (会话管理)
```

---

*文档生成时间: 2026-03-01*
