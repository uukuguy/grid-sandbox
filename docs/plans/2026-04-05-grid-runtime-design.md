# grid-runtime 设计文档 — EAASP L1 智能体执行层

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**目标:** 新建 `grid-runtime` crate，作为 EAASP L1 智能体运行时的 Rust 参考实现。遵守 13 方法运行时接口契约，以 gRPC 服务形式被平台拉起和管理。

**背景:** EAASP（企业自主智能体支撑平台）定义了四层架构（L4 人机协作 → L3 治理 → L2 技能资产 → L1 执行），grid-runtime 是 L1 层的 Tier 1 Harness 实现。

**参考:** `docs/design/Grid/EAASP_-_企业自主智能体支撑平台设计规范_v1.7_.pdf`

---

## 1. 架构定位

### 1.1 grid-runtime 在 EAASP 中的位置

```
EAASP 平台（L4+L3+L2）
         │
         │ 13 方法契约（gRPC）
         │
    ┌────┴─────┬──────────┬──────────┐
    │          │          │          │
grid-runtime  Aider-RT   Goose-RT   LangGraph-RT
(Rust容器)    (Python容器) (Rust容器)  (Python容器)
 Tier 1         Tier 2      Tier 2     Tier 3
 原生hooks     +HookBridge +HookBridge +HookBridge
```

- grid-runtime 是**自包含容器**，被平台拉起，与其他 runtime 相互独立
- 对平台上层而言，所有 runtime 行为完全一致（13 方法契约）
- grid-runtime 原生支持 hooks/MCP/skills，**不需要 HookBridge**
- HookBridge 是独立 sidecar 程序（`grid-hook-bridge`），给 Tier 2/3 用

### 1.2 Crate 依赖图

```
grid-types (0)
    ↓
grid-sandbox (1)   grid-engine (1)
                       ↓         ↓
               grid-runtime (2)  grid-cli (2)   grid-server (2)
```

三种"使用 grid-engine 的方式"：

| crate | 面向谁 | 交互方式 | 部署方式 |
|-------|-------|---------|---------|
| **grid-cli** | 终端用户 | CLI/TUI | 本地二进制 |
| **grid-server** | 浏览器用户 | Web UI + REST/WS | 本地/服务器 |
| **grid-runtime** | EAASP 平台 | 13 方法 gRPC 契约 | 容器（被平台拉起） |

grid-runtime **不**承担 grid-server 的功能。grid-server 是 grid-cli 的 Web 版。grid-runtime 纯粹服务 EAASP 平台。

### 1.3 设计原则

- **运行时无关:** 任何实现 13 方法契约的容器都可加入运行时池
- **零适配器:** Grid 作为 Tier 1 Harness，进程内直接调用 grid-engine，无序列化开销
- **扩展而非重建:** grid-engine 不变，grid-runtime 是增量新建
- **契约即文档:** `runtime.proto` 是 13 方法契约的机器可读定义

---

## 2. 运行时接口契约（13 方法）

### 2.1 Rust trait 定义

```rust
#[async_trait]
pub trait RuntimeContract: Send + Sync {
    async fn initialize(&self, payload: SessionPayload) -> Result<SessionHandle>;
    async fn send(&self, handle: &SessionHandle, message: UserMessage)
        -> Result<Pin<Box<dyn Stream<Item = ResponseChunk> + Send>>>;
    async fn load_skill(&self, handle: &SessionHandle, content: SkillContent) -> Result<()>;
    async fn on_tool_call(&self, handle: &SessionHandle, call: ToolCall) -> Result<HookDecision>;
    async fn on_tool_result(&self, handle: &SessionHandle, result: ToolResult) -> Result<HookDecision>;
    async fn on_stop(&self, handle: &SessionHandle) -> Result<StopDecision>;
    async fn get_state(&self, handle: &SessionHandle) -> Result<SessionState>;
    async fn restore_state(&self, state: SessionState) -> Result<SessionHandle>;
    async fn connect_mcp(&self, handle: &SessionHandle, servers: Vec<McpServerConfig>) -> Result<()>;
    async fn emit_telemetry(&self, handle: &SessionHandle) -> Result<Vec<TelemetryEvent>>;
    fn get_capabilities(&self) -> CapabilityManifest;
    async fn terminate(&self, handle: &SessionHandle) -> Result<()>;
    async fn health(&self) -> Result<HealthStatus>;
}
```

### 2.2 双形态

- **Rust trait** — GridHarness 进程内直接实现（零序列化开销）
- **runtime.proto** — 镜像的 gRPC service 定义，供外部适配器（Python/TS）使用
- Grid 不走 gRPC 调自己，外部适配器才走 gRPC

### 2.3 关键类型

| 类型 | 用途 |
|------|------|
| `SessionPayload` | 用户上下文、角色、组织单元、受管 hooks、配额（从 L3 三方握手获得） |
| `SessionHandle` | 不透明会话标识（session_id + 内部状态引用） |
| `UserMessage` | 用户消息或结构化意图 |
| `ResponseChunk` | 流式响应片段（TextDelta / ToolStart / ToolResult / Done） |
| `SkillContent` | SKILL.md 内容（frontmatter hooks + prose instructions） |
| `ToolCall` | 工具调用请求（名称、参数） |
| `ToolResult` | 工具执行结果（内容、是否错误） |
| `HookDecision` | Allow / Deny(reason) / Modify(transformed_input) |
| `StopDecision` | Complete / Continue(feedback)（对应 exit 0 / exit 2） |
| `SessionState` | 序列化的完整会话状态（对话历史、内核状态） |
| `TelemetryEvent` | 标准化遥测事件（session_id, runtime_id, event_type, payload, resource_usage） |
| `CapabilityManifest` | 能力清单（模型、上下文窗口、工具、原生 hooks/MCP/skills、成本） |
| `HealthStatus` | 运行时健康状态（provider 连通性、MCP 健康） |

### 2.4 契约方法与 Grid 现有能力的映射

| 契约方法 | grid-engine 调用路径 | 差距 |
|---------|---------------------|------|
| `initialize` | `AgentRuntime::builder().build()` | 需增加 SessionPayload 参数 |
| `send` | `AgentExecutor::run()` → broadcast stream | 需 AgentEvent → ResponseChunk 转换 |
| `load_skill` | `SkillRuntime::execute_skill()` | ✅ 已有 |
| `on_tool_call` | PreToolUse hook 原生触发 | Grid 无需额外代码，空操作 |
| `on_tool_result` | PostToolUse hook 原生触发 | Grid 无需额外代码，空操作 |
| `on_stop` | Stop hook 原生触发 | Grid 无需额外代码，空操作 |
| `get_state` | `SessionStore::get()` + executor 状态 | 需统一序列化 |
| `restore_state` | `SessionStore::restore()` | 需从序列化重建 executor |
| `connect_mcp` | `McpManager::start_server()` | ✅ 已有 |
| `emit_telemetry` | `EventBus::subscribe()` | 需定义标准 schema 并转换 |
| `get_capabilities` | **❌ 不存在** | 全新，需定义 Grid 能力清单 |
| `terminate` | 分散在多处 | 需统一清理路径 |
| `health` | provider ping + MCP health | 需聚合 |

---

## 3. Crate 结构

```
crates/grid-runtime/
├── Cargo.toml
├── build.rs                    # protobuf 编译（tonic-build）
├── Dockerfile                  # 容器镜像
├── src/
│   ├── main.rs                 # gRPC server 入口
│   ├── config.rs               # 运行时配置（gRPC 端口、engine 配置）
│   ├── contract.rs             # RuntimeContract trait + 全部类型定义
│   ├── harness.rs              # GridHarness: impl RuntimeContract（桥接 grid-engine）
│   ├── service.rs              # gRPC service impl（tonic，转发到 GridHarness）
│   └── telemetry.rs            # TelemetryEvent schema + EventBus 转换

proto/eaasp/runtime/v1/
└── runtime.proto               # 13 方法 gRPC service 定义（权威，多方共享）
```

### proto 共享

`runtime.proto` 放在 mono-repo 顶层 `proto/` 目录，供以下消费方共享：
- `grid-runtime`（Rust，tonic-build）
- `eaasp-certifier`（Rust，tonic-build）
- 未来的 Python/TS 适配器（grpcio / @grpc/grpc-js）

---

## 4. GridHarness 实现

GridHarness 将 grid-engine 的现有能力桥接到 13 方法契约：

- **核心工作量:**
  1. `initialize` — SessionPayload → AgentRuntime builder 参数翻译
  2. `send` — AgentEvent broadcast stream → ResponseChunk stream 转换
  3. `get_state/restore_state` — 统一序列化/反序列化
  4. `emit_telemetry` — EventBus 事件 → TelemetryEvent schema 转换
  5. `get_capabilities` — 定义 Grid 的 CapabilityManifest

- **空操作（原生 hooks 自动处理）:**
  - `on_tool_call` — Grid 的 PreToolUse hooks 在 AgentLoop 内联触发
  - `on_tool_result` — Grid 的 PostToolUse hooks 在 AgentLoop 内联触发
  - `on_stop` — Grid 的 Stop hooks 在 AgentLoop 内联触发

---

## 5. EAASP Certifier

独立工具，模拟 EAASP L4/L3 行为并认证运行时契约合规性，用于验证**任何** runtime 的契约合规性。

```
tools/eaasp-certifier/
├── Cargo.toml                  # 依赖 tonic (gRPC client) + grid-types
├── src/
│   ├── main.rs                 # CLI: eaasp-certifier verify <container-image>
│   ├── verifier.rs             # 13 方法逐一验证 + 认证报告
│   ├── mock_l3.rs              # 模拟 L3: hooks 下发、遥测收集
│   └── mock_l4.rs              # 模拟 L4: 三方握手、消息收发
└── test_fixtures/
    ├── sample_hooks.json       # 测试用 managed-settings
    └── sample_skill.md         # 测试用 workflow-skill
```

### 演进路径

- 现在：`eaasp-certifier verify grid-runtime` → 验证 Grid 通过契约
- 未来：`eaasp-certifier verify aider-runtime` → 验证 Aider 适配器通过契约
- 最终：验证逻辑迁入真实 EAASP 平台的 L1 运行时认证流水线

---

## 6. HookBridge（不在本阶段范围）

独立 crate `grid-hook-bridge`，给 Tier 2/3 运行时用的 sidecar 容器。grid-runtime 原生支持 hooks，不需要 HookBridge。等接入第一个 Tier 2 运行时时再建。

---

## 7. 实施计划

**基线:** grid-engine 2476+ tests，grid-cli 499 studio tests

| Wave | 内容 | 文件 | 预估 |
|------|------|------|------|
| **W1** | 新建 crate + proto 定义 | `Cargo.toml`, `build.rs`, `proto/eaasp/runtime/v1/runtime.proto`, `contract.rs` | 1 session |
| **W2** | GridHarness 实现 | `harness.rs`（13 方法桥接 grid-engine） | 2 sessions |
| **W3** | gRPC server | `service.rs`, `main.rs`, `config.rs` | 1 session |
| **W4** | 遥测 schema + 转换 | `telemetry.rs` | 1 session |
| **W5** | eaasp-certifier + 集成测试 | `tools/eaasp-certifier/` | 1 session |
| **W6** | Dockerfile + 容器化 | `Dockerfile`, Makefile 更新 | 1 session |

## Deferred（暂缓项）

| ID | 内容 | 前置条件 | 状态 |
|----|------|---------|------|
| BD-D1 | grid-hook-bridge crate（独立 sidecar） | 接入第一个 Tier 2 运行时 | ⏳ |
| BD-D2 | RuntimeSelector + AdapterRegistry（属于平台层） | EAASP 平台建设启动 | ⏳ |
| BD-D3 | 盲盒对比（dual output + vote） | 运行时池中有 2+ 运行时 | ⏳ |
| BD-D4 | managed-settings.json 分发机制 | L3 治理层建设 | ⏳ |
| BD-D5 | SessionPayload 中的组织层级（企业→BU→部门→团队） | L4 多租户建设 | ⏳ |
