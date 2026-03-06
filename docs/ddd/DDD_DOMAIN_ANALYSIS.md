# Octo-Sandbox 领域驱动设计（DDD）分析报告

## 概览

本文档对 `octo-engine` 进行系统性的领域驱动设计分析，识别限界上下文、聚合根、领域事件及上下文映射关系，并基于分析结果提出 `AgentRuntime` God Object 问题的具体重构建议。

---

## 1. 限界上下文识别（Bounded Contexts）

### 1.1 核心域（Core Domain）

#### 1.1.1 Agent 执行上下文（Agent Execution Context）

**职责**：管理 Agent 的完整生命周期，从定义到运行时执行。

**关键类型**：

| 类型 | 文件 | 角色 |
|------|------|------|
| `AgentRuntime` | `agent/runtime.rs` | 聚合根（当前过于臃肿） |
| `AgentExecutor` | `agent/executor.rs` | 实体（持久化运行的智能体本体） |
| `AgentLoop` | `agent/loop_.rs` | 领域服务（单轮对话执行引擎） |
| `AgentCatalog` | `agent/catalog.rs` | 仓储（多索引 Agent 注册表） |
| `AgentManifest` | `agent/entry.rs` | 值对象（Agent 定义规格） |
| `AgentId` | `agent/entry.rs` | 值对象（类型化标识符） |
| `AgentStatus` | `agent/entry.rs` | 枚举值对象（状态机） |
| `AgentConfig` | `agent/config.rs` | 值对象（运行时配置参数） |
| `CancellationToken` | `agent/cancellation.rs` | 值对象（取消信号） |
| `TenantContext` | `agent/tenant.rs` | 值对象（租户隔离上下文） |

**状态机**：
```
Created → Running → Paused → Stopped
                 ↘ Error
```

**当前问题**：`AgentRuntime` 包含 16 个字段，跨越多个职责领域（内存管理、安全策略、MCP 管理、可观测性），是典型的 God Object 反模式。

---

#### 1.1.2 工具执行上下文（Tool Execution Context）

**职责**：管理工具注册、工具能力描述、工具执行及结果记录。

**关键类型**：

| 类型 | 文件 | 角色 |
|------|------|------|
| `ToolRegistry` | `tools/mod.rs` | 仓储（工具注册表） |
| `Tool` trait | `tools/traits.rs` | 领域接口（工具抽象） |
| `ToolContext` | `octo-types/src/tool.rs` | 值对象（工具执行上下文） |
| `ToolResult` | `octo-types/src/tool.rs` | 值对象（执行结果） |
| `ToolSpec` | `octo-types/src/tool.rs` | 值对象（工具规格描述） |
| `ToolSource` | `octo-types/src/tool.rs` | 枚举值对象（工具来源） |
| `BashTool` | `tools/bash.rs` | 领域服务（Shell 执行工具） |
| `ToolExecutionRecorder` | `tools/recorder.rs` | 领域服务（执行记录器） |
| `PathValidator` trait | `octo-types/src/tool.rs` | 领域接口（路径验证器） |

**上下文边界说明**：`ToolContext` 通过 `path_validator: Option<Arc<dyn PathValidator>>` 接受安全策略注入，是安全上下文与工具执行上下文之间的防腐层接触点。

---

### 1.2 支撑域（Supporting Domain）

#### 1.2.1 安全策略上下文（Security Policy Context）

**职责**：控制命令执行权限、路径访问范围、操作频率限制和自主级别管理。

**关键类型**：

| 类型 | 文件 | 角色 |
|------|------|------|
| `SecurityPolicy` | `security/policy.rs` | 聚合根（安全策略规则集） |
| `AutonomyLevel` | `security/policy.rs` | 枚举值对象（自主级别） |
| `CommandRiskLevel` | `security/policy.rs` | 枚举值对象（命令风险等级） |
| `ActionTracker` | `security/mod.rs` | 实体（行为频率跟踪器） |
| `ExecPolicy` | `tools/bash.rs` | 值对象（Shell 执行策略） |
| `ExecSecurityMode` | `tools/bash.rs` | 枚举值对象（执行安全模式） |

**领域规则（不变量）**：
- `ReadOnly` 自主级别下任何命令执行均被拒绝
- `Supervised` 级别下中风险和高风险命令须经人工审批
- 高风险命令（`rm -rf`、`dd` 等）在 `block_high_risk_commands=true` 时完全阻断
- 路径访问限制在 `workspace_only=true` 时严格约束到 `workspace_dir`
- 每小时操作数不得超过 `max_actions_per_hour`

**设计观察**：`SecurityPolicy` 实现了 `octo_types::PathValidator` trait，这是跨上下文的共享接口（发布语言）。`ExecPolicy`（在 `tools/bash.rs` 中）与 `SecurityPolicy` 的职责存在重叠——两者都定义了命令白名单，是潜在的领域模型混乱点。

---

#### 1.2.2 认证授权上下文（Auth Context）

**职责**：身份验证（API Key / JWT）、基于角色的权限控制（RBAC）、多租户隔离。

**关键类型**：

| 类型 | 文件 | 角色 |
|------|------|------|
| `AuthConfig` | `auth/config.rs` | 聚合根（认证配置） |
| `ApiKey` | `auth/config.rs` | 实体（API 密钥，含哈希存储） |
| `Permission` | `auth/config.rs` | 枚举值对象（Read/Write/Admin） |
| `AuthMode` | `auth/config.rs` | 枚举值对象（None/ApiKey/Full） |
| `JwtClaims` | `auth/config.rs` | 值对象（JWT 声明集） |
| `Role` | `auth/roles.rs` | 枚举值对象（Viewer/User/Admin/Owner） |
| `TenantContext` | `agent/tenant.rs` | 值对象（运行时租户上下文） |
| `Action` | `agent/tenant.rs` | 枚举值对象（可授权操作） |

**领域规则（不变量）**：
- API Key 以 HMAC-SHA256 哈希存储，原始 key 不持久化
- JWT 仅在 `AuthMode::Full` 下启用（为 `octo-platform` 保留）
- `TenantContext::can()` 实现基于角色的操作授权

**设计观察**：`TenantContext` 同时存在于 `auth/` 和 `agent/tenant.rs` 两个位置，存在职责模糊问题——它是认证上下文的产物（由 JWT/API Key 解析而来），却被注入到 Agent 执行上下文中使用。

---

#### 1.2.3 MCP 集成上下文（MCP Integration Context）

**职责**：管理外部 MCP 服务器的生命周期，桥接 MCP 工具到统一工具接口。

**关键类型**：

| 类型 | 文件 | 角色 |
|------|------|------|
| `McpManager` | `mcp/manager.rs` | 聚合根（服务器连接管理） |
| `McpClient` trait | `mcp/traits.rs` | 领域接口（客户端抽象） |
| `StdioMcpClient` | `mcp/stdio.rs` | 基础设施适配器（标准 IO 传输） |
| `SseMcpClient` | `mcp/sse.rs` | 基础设施适配器（SSE 传输） |
| `McpToolBridge` | `mcp/bridge.rs` | 防腐层（将 MCP 工具适配为 Tool trait） |
| `McpStorage` | `mcp/storage.rs` | 仓储（SQLite 持久化） |
| `McpServerConfig` | `mcp/traits.rs` | 值对象（服务器配置） |
| `McpServerConfigV2` | `mcp/traits.rs` | 值对象（支持多传输协议的配置 v2） |
| `McpToolInfo` | `mcp/traits.rs` | 值对象（工具元信息） |
| `ServerRuntimeState` | `mcp/manager.rs` | 枚举值对象（服务器运行时状态） |

**聚合不变量**：
- 同一名称的 MCP Server 不能重复注册（`clients` HashMap 以名称为键）
- `tool_infos` 与 `clients` 必须保持一致（添加/移除时同步更新）

---

#### 1.2.4 记忆管理上下文（Memory Context）

**职责**：管理多层级记忆系统，提供跨会话的持久化和语义检索能力。

**关键类型**：

| 类型 | 文件 | 角色 |
|------|------|------|
| `MemorySystem` | `memory/mod.rs` | 聚合根（统一内存系统门面） |
| `WorkingMemory` trait | `memory/traits.rs` | 领域接口（Layer 0 工作记忆） |
| `InMemoryWorkingMemory` | `memory/working.rs` | 实现（纯内存实现） |
| `SqliteWorkingMemory` | `memory/sqlite_working.rs` | 实现（SQLite 持久化实现） |
| `MemoryStore` trait | `memory/store_traits.rs` | 领域接口（Layer 2 持久化记忆） |
| `SqliteMemoryStore` | `memory/sqlite_store.rs` | 实现（长期记忆 SQLite 存储） |
| `KnowledgeGraph` | `memory/graph.rs` | 聚合根（知识图谱，实体-关系图） |
| `GraphStore` | `memory/graph_store.rs` | 仓储（知识图谱持久化） |
| `FtsStore` | `memory/fts.rs` | 仓储（全文检索存储） |
| `TokenBudgetManager` | `memory/budget.rs` | 领域服务（Token 预算管理） |
| `SemanticMemory` | `memory/semantic.rs` | 领域服务（语义实体管理） |

**记忆层次架构**：
```
Layer 0: WorkingMemory    — 当前对话上下文（每 session 独立实例）
Layer 1: SessionStore     — 按 session 持久化的对话历史
Layer 2: MemoryStore      — 跨 session 的长期知识存储
Layer 3: KnowledgeGraph   — 结构化实体-关系知识图谱（含 FTS）
```

---

#### 1.2.5 可观测性上下文（Observability Context）

**职责**：事件发布/订阅、Token 用量计量、工具执行记录、指标收集。

**关键类型**：

| 类型 | 文件 | 角色 |
|------|------|------|
| `EventBus` | `event/bus.rs` | 领域服务（事件总线，pub/sub） |
| `OctoEvent` | `event/bus.rs` | 值对象（事件载体） |
| `Metering` | `metering/mod.rs` | 聚合根（Token 用量计量器） |
| `MeteringSnapshot` | `metering/mod.rs` | 值对象（计量快照） |
| `ToolExecutionRecorder` | `tools/recorder.rs` | 仓储（工具执行历史记录） |
| `MetricsRegistry` | `metrics/mod.rs` | 领域服务（指标注册表） |
| `AgentEvent` | `agent/loop_.rs` | 值对象（Agent 执行事件） |

**事件流向**：
```
AgentLoop → AgentEvent (broadcast) → WebSocket/EventBus → Frontend/Subscribers
```

---

#### 1.2.6 会话管理上下文（Session Context）

**职责**：管理对话会话的生命周期和消息历史持久化。

**关键类型**：

| 类型 | 文件 | 角色 |
|------|------|------|
| `SessionStore` trait | `session/mod.rs` | 领域接口 |
| `SessionData` | `session/mod.rs` | 实体（会话元数据） |
| `SessionSummary` | `session/mod.rs` | 读模型（会话列表视图） |
| `SqliteSessionStore` | `session/sqlite.rs` | 仓储实现 |
| `InMemorySessionStore` | `session/memory.rs` | 仓储实现（测试用） |

---

### 1.3 通用域（Generic Domain）

#### 1.3.1 LLM Provider 上下文（Provider Context）

**关键类型**：`Provider` trait、`AnthropicProvider`、`OpenAIProvider`、`ProviderChain`（故障转移/负载均衡）、`MeteringProvider`（装饰器模式）。

#### 1.3.2 沙箱执行上下文（Sandbox Context）

**关键类型**：`RuntimeAdapter` trait、`SubprocessAdapter`（本地进程）、`WasmAdapter`（可选特性）、`DockerAdapter`（可选特性）。

---

## 2. 聚合根分析（Aggregate Roots）

### 2.1 AgentRuntime 的 God Object 问题

`AgentRuntime` 当前包含 **16 个字段**，跨越至少 **6 个不同职责域**：

```rust
pub struct AgentRuntime {
    // --- Agent 执行职责 ---
    primary_handle: Mutex<Option<AgentExecutorHandle>>,  // 执行句柄管理
    agent_handles: DashMap<AgentId, CancellationToken>,  // 取消令牌管理
    catalog: Arc<AgentCatalog>,                           // Agent 注册表
    default_model: String,                               // 模型配置

    // --- 工具执行职责 ---
    tools: Arc<StdMutex<ToolRegistry>>,                  // 工具注册表
    skill_registry: Option<Arc<SkillRegistry>>,          // 技能注册表
    working_dir: PathBuf,                                // 工作目录

    // --- LLM Provider 职责 ---
    provider: Arc<dyn Provider>,                         // LLM 提供者
    provider_chain: Option<Arc<ProviderChain>>,          // 提供者链

    // --- 记忆管理职责 ---
    memory: Arc<dyn WorkingMemory>,                      // 工作记忆
    memory_store: Arc<dyn MemoryStore>,                  // 持久化记忆
    session_store: Arc<dyn SessionStore>,                // 会话存储

    // --- MCP 集成职责 ---
    mcp_manager: Arc<Mutex<McpManager>>,                 // MCP 服务器管理

    // --- 可观测性职责 ---
    event_bus: Option<Arc<EventBus>>,                    // 事件总线
    recorder: Arc<ToolExecutionRecorder>,                // 执行记录器
    metering: Arc<Metering>,                             // 用量计量

    // --- 安全策略职责 ---
    security_policy: Arc<SecurityPolicy>,                // 安全策略

    // --- 多租户职责 ---
    tenant_context: Option<TenantContext>,               // 租户上下文
}
```

**问题诊断**：
1. **单一职责原则违反**：18 个字段映射到 6 个不同的限界上下文
2. **构造函数复杂度**：`new()` 方法包含 15 个初始化步骤（步骤 1-15 在代码注释中明确标注）
3. **锁污染**：`StdMutex<ToolRegistry>` 与 `Mutex<McpManager>` 混用（同步锁 + 异步锁），在异步上下文中存在死锁风险
4. **测试困难**：无法独立测试单个职责，必须初始化整个 `AgentRuntime`
5. **边界渗漏**：`TenantContext` 既属于认证授权上下文，又被嵌入 Agent 执行上下文

### 2.2 各限界上下文的理想聚合根

| 聚合根 | 限界上下文 | 不变量 |
|--------|------------|--------|
| `AgentRuntime`（拆分后） | Agent 执行 | agent_handles 中每个 AgentId 唯一 |
| `SecurityPolicy` | 安全策略 | autonomy + workspace_dir 组合决定所有权限判断 |
| `AuthConfig` | 认证授权 | API Key 以哈希形式唯一存储 |
| `McpManager` | MCP 集成 | clients 和 tool_infos 必须同步一致 |
| `MemorySystem` | 记忆管理 | 四层记忆的写入顺序（持久化优先于内存更新） |
| `Metering` | 可观测性 | 原子计数器确保并发安全 |

---

## 3. 领域事件（Domain Events）

### 3.1 当前 AgentEvent 事件流

`AgentEvent` 定义在 `agent/loop_.rs` 中，是 Agent 执行上下文对外发布的领域事件：

```
TextDelta          — LLM 正在流式生成文本（实时推送）
TextComplete       — 本轮文本生成完成
ThinkingDelta      — 扩展思考模式：思考文本流
ThinkingComplete   — 扩展思考完成
ToolStart          — 开始执行工具调用
ToolResult         — 工具执行结果返回
ToolExecution      — 工具完整执行记录（含时长）
TokenBudgetUpdate  — Token 预算更新通知
Typing             — 打字状态变更（started/stopped）
Error              — 执行过程中的错误
Done               — 本轮对话完成
```

### 3.2 缺失的领域事件（设计建议）

当前系统缺少以下重要的领域事件，限制了系统的可观测性和事件驱动架构能力：

| 缺失事件 | 所属上下文 | 业务意义 |
|----------|------------|----------|
| `AgentStarted` | Agent 执行 | Agent 进入 Running 状态 |
| `AgentStopped` | Agent 执行 | Agent 进入 Stopped 状态 |
| `AgentPaused` | Agent 执行 | Agent 进入 Paused 状态 |
| `McpServerConnected` | MCP 集成 | MCP 服务器建立连接 |
| `McpServerDisconnected` | MCP 集成 | MCP 服务器断开连接 |
| `SecurityViolation` | 安全策略 | 检测到安全策略违规 |
| `RateLimitExceeded` | 安全策略 | 超过操作频率限制 |
| `MemoryEvicted` | 记忆管理 | 工作记忆因预算超限被压缩 |
| `TokenBudgetWarning` | 可观测性 | Token 预算接近上限（80%） |

### 3.3 事件风暴（Event Storming）输出

```
命令（蓝色）           →    领域事件（橙色）          →    策略（紫色）
─────────────────────────────────────────────────────────────────────
SendMessage           →    TextDelta                 →    流式推送到 WebSocket
StartAgent            →    AgentStarted [缺失]        →    更新 Catalog 状态
StopAgent             →    AgentStopped [缺失]        →    清理 CancellationToken
AddMcpServer          →    McpServerConnected [缺失]  →    桥接工具到 ToolRegistry
RemoveMcpServer       →    McpServerDisconnected      →    注销 MCP 工具
ExecuteBashCommand    →    ToolStart / ToolResult     →    记录到 Recorder
RateLimitCheck        →    RateLimitExceeded [缺失]   →    拒绝执行
PathValidation        →    SecurityViolation [缺失]   →    返回错误 ToolResult
```

---

## 4. 上下文映射（Context Map）

### 4.1 上下文关系图

```
┌──────────────────────────────────────────────────────────────────┐
│                    OCTO-ENGINE 上下文映射                         │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────────┐   Partnership    ┌─────────────────┐        │
│  │   Agent 执行    │◀──────────────▶│   工具执行       │        │
│  │  AgentRuntime   │                  │  ToolRegistry    │        │
│  │  AgentExecutor  │                  │  Tool trait      │        │
│  │  AgentLoop      │                  │  ToolContext     │        │
│  └────────┬────────┘                  └────────┬────────┘        │
│           │                                    │                  │
│           │ Published Language                 │ ACL              │
│           │ (AgentEvent)                       │ (McpToolBridge)  │
│           ▼                                    ▼                  │
│  ┌─────────────────┐   Open Host Svc  ┌─────────────────┐        │
│  │   可观测性      │◀──────────────── │   MCP 集成      │        │
│  │  EventBus       │                  │  McpManager      │        │
│  │  Metering       │                  │  StdioClient     │        │
│  │  Recorder       │                  │  SseClient       │        │
│  └─────────────────┘                  └─────────────────┘        │
│                                                                  │
│  ┌─────────────────┐   ACL            ┌─────────────────┐        │
│  │   安全策略      │──────────────── ▶│   工具执行       │        │
│  │  SecurityPolicy │  (PathValidator) │  BashTool        │        │
│  │  AutonomyLevel  │                  │  ExecPolicy      │        │
│  └────────┬────────┘                  └─────────────────┘        │
│           │                                                       │
│           │ Customer-Supplier                                     │
│           ▼                                                       │
│  ┌─────────────────┐   Conformist     ┌─────────────────┐        │
│  │   认证授权      │──────────────── ▶│   Agent 执行     │        │
│  │  AuthConfig     │  (TenantContext) │  TenantContext   │        │
│  │  ApiKey / JWT   │                  │  verify_tenant   │        │
│  └─────────────────┘                  └─────────────────┘        │
│                                                                  │
│  ┌─────────────────┐   Shared Kernel  ┌─────────────────┐        │
│  │   记忆管理      │◀────────────────▶│   会话管理       │        │
│  │  WorkingMemory  │  (ChatMessage,   │  SessionStore    │        │
│  │  MemoryStore    │   SessionId)     │  SessionData     │        │
│  │  KnowledgeGraph │                  └─────────────────┘        │
│  └─────────────────┘                                             │
│                                                                  │
│  ┌─────────────────────────────────────────────────────────┐     │
│  │                   共享内核 (octo-types)                  │     │
│  │  ChatMessage, ToolSpec, ToolResult, ToolContext,         │     │
│  │  PathValidator, SessionId, TenantId, UserId, SandboxId  │     │
│  └─────────────────────────────────────────────────────────┘     │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

### 4.2 上下文关系模式说明

| 上下游关系 | 模式 | 说明 |
|------------|------|------|
| Agent 执行 ↔ 工具执行 | **伙伴关系（Partnership）** | 双方紧密协作，`AgentLoop` 直接调用 `ToolRegistry` |
| 安全策略 → 工具执行 | **防腐层（ACL）** | `PathValidator` trait 隔离安全策略实现细节 |
| MCP 集成 → 工具执行 | **防腐层（ACL）** | `McpToolBridge` 将 MCP 协议适配为统一 `Tool` 接口 |
| 认证授权 → Agent 执行 | **顺从者（Conformist）** | Agent 执行上下文直接使用认证上下文的 `TenantContext` 结构 |
| Agent 执行 → 可观测性 | **发布语言（Published Language）** | `AgentEvent` 是稳定的发布事件契约 |
| MCP 集成 → 可观测性 | **开放主机服务（Open Host Service）** | `McpManager` 通过标准化接口暴露服务状态 |
| 记忆管理 ↔ 会话管理 | **共享内核（Shared Kernel）** | 共享 `ChatMessage`、`SessionId` 等类型 |
| 所有上下文 → octo-types | **共享内核（Shared Kernel）** | 跨上下文的通用类型定义 |

---

## 5. 重构建议

### 5.1 AgentRuntime 拆分方案

**目标**：将 God Object 拆分为职责单一的服务组合，各聚合根独立管理自身一致性边界。

#### 5.1.1 拆分后的架构设计

```rust
/// 核心：Agent 执行上下文的聚合根（拆分后精简版本）
pub struct AgentRuntime {
    // Agent 执行核心（保留）
    primary_handle: Mutex<Option<AgentExecutorHandle>>,
    agent_handles: DashMap<AgentId, CancellationToken>,
    catalog: Arc<AgentCatalog>,

    // 通过服务门面注入外部依赖（替代直接持有大量字段）
    execution_context: Arc<ExecutionContext>,
    observability: Arc<ObservabilityContext>,
    tenant_context: Option<TenantContext>,
}

/// 执行上下文：工具 + Provider + 记忆 + 安全 + MCP 的统一门面
pub struct ExecutionContext {
    provider: Arc<dyn Provider>,
    provider_chain: Option<Arc<ProviderChain>>,
    tools: Arc<StdMutex<ToolRegistry>>,
    skill_registry: Option<Arc<SkillRegistry>>,
    memory: Arc<dyn WorkingMemory>,
    memory_store: Arc<dyn MemoryStore>,
    session_store: Arc<dyn SessionStore>,
    security_policy: Arc<SecurityPolicy>,
    mcp_manager: Arc<Mutex<McpManager>>,
    working_dir: PathBuf,
    default_model: String,
}

/// 可观测性上下文：事件 + 计量 + 记录的统一门面
pub struct ObservabilityContext {
    event_bus: Option<Arc<EventBus>>,
    recorder: Arc<ToolExecutionRecorder>,
    metering: Arc<Metering>,
}
```

#### 5.1.2 AgentRuntimeConfig 配置注入优化

```rust
/// 重构后的运行时配置（子配置分组，职责清晰）
pub struct AgentRuntimeConfig {
    pub db_path: String,
    pub provider: ProviderConfig,
    pub provider_chain: Option<ProviderChainConfig>,
    pub skills: SkillsConfig,        // 新增：技能相关配置聚合
    pub security: SecurityConfig,    // 新增：安全策略配置聚合
    pub observability: ObsConfig,    // 新增：可观测性配置聚合
    pub working_dir: Option<PathBuf>,
}

pub struct SkillsConfig {
    pub dirs: Vec<String>,
    pub enable_hot_reload: bool,
}

pub struct SecurityConfig {
    pub autonomy: AutonomyLevel,
    pub workspace_only: bool,
    pub max_actions_per_hour: u32,
}

pub struct ObsConfig {
    pub enable_event_bus: bool,
    pub enable_metering: bool,
}
```

### 5.2 安全策略上下文重构

**问题**：`SecurityPolicy`（在 `security/policy.rs`）和 `ExecPolicy`（在 `tools/bash.rs`）存在命令白名单职责重叠。

**建议**：
1. 将 `ExecPolicy` 的逻辑合并到 `SecurityPolicy` 的命令检查中
2. `BashTool` 仅通过 `ToolContext.path_validator` 接收统一的安全上下文
3. `SecurityPolicy` 成为唯一的安全策略聚合根

```rust
// 建议：BashTool 移除内部 ExecPolicy，完全依赖注入的 PathValidator
pub struct BashTool {
    // 移除: exec_policy: Option<ExecPolicy>
    // 通过 ctx.path_validator 和扩展后的 ToolContext 获取安全策略
}

// 建议：ToolContext 扩展以携带安全策略
pub struct ToolContext {
    pub sandbox_id: SandboxId,
    pub working_dir: PathBuf,
    pub path_validator: Option<Arc<dyn PathValidator>>,
    pub command_validator: Option<Arc<dyn CommandValidator>>,  // 新增
}

// 新增：命令验证器接口（发布语言）
pub trait CommandValidator: Send + Sync + Debug {
    fn check_command(&self, command: &str) -> Result<CommandRiskLevel, String>;
    fn requires_approval(&self, command: &str) -> bool;
}
```

### 5.3 领域事件体系完善

**建议**：定义完整的系统级领域事件（区别于 `AgentEvent` 的流式渲染事件）：

```rust
/// 系统级领域事件（用于持久化事件溯源和跨上下文通信）
pub enum SystemEvent {
    // Agent 执行上下文
    AgentStarted { agent_id: AgentId, session_id: SessionId, timestamp: DateTime<Utc> },
    AgentStopped { agent_id: AgentId, reason: StopReason, timestamp: DateTime<Utc> },
    AgentPaused  { agent_id: AgentId, timestamp: DateTime<Utc> },

    // MCP 集成上下文
    McpServerConnected    { server_name: String, tool_count: usize, timestamp: DateTime<Utc> },
    McpServerDisconnected { server_name: String, reason: String, timestamp: DateTime<Utc> },

    // 安全策略上下文
    SecurityViolation { violation_type: ViolationType, details: String, timestamp: DateTime<Utc> },
    RateLimitExceeded { action_count: usize, limit: usize, timestamp: DateTime<Utc> },

    // 记忆管理上下文
    MemoryEvicted    { session_id: SessionId, bytes_evicted: usize, timestamp: DateTime<Utc> },

    // 可观测性上下文
    TokenBudgetWarning { usage_pct: f32, remaining: u32, timestamp: DateTime<Utc> },
}
```

### 5.4 TenantContext 归属明确化

**问题**：`TenantContext` 同时出现在 `agent/tenant.rs` 和 `auth/` 上下文中，职责边界不清。

**建议**：
- `TenantContext` 属于**认证授权上下文**，由认证中间件在请求边界创建
- 通过**防腐层**（请求上下文 / `RequestContext`）传入 Agent 执行上下文
- `AgentRuntime` 不直接持有 `TenantContext`，而是在每次方法调用时接受传入

```rust
// 建议：将 tenant_context 从 AgentRuntime 字段移出
impl AgentRuntime {
    pub async fn start(
        &self,
        agent_id: &AgentId,
        session_id: SessionId,
        // 以参数传入，而非字段存储
        tenant_context: &TenantContext,
        ...
    ) -> Result<AgentExecutorHandle, AgentError> {
        // 在方法入口验证租户权限
        tenant_context.can(Action::RunAgent)
            .then_some(())
            .ok_or(AgentError::PermissionDenied(...))?;
        ...
    }
}
```

### 5.5 MCP 集成上下文的锁优化

**问题**：`mcp_manager: Arc<Mutex<McpManager>>` 使用粗粒度 `tokio::sync::Mutex`，所有 MCP 操作（包括工具调用）都需要持有全局锁。

**建议**：
1. `McpManager` 内部使用 `DashMap` 替代 `HashMap` 实现细粒度锁
2. 读操作（`call_tool`、`get_tool_infos`）使用无锁或读锁
3. 写操作（`add_server`、`remove_server`）使用写锁

```rust
pub struct McpManager {
    clients: DashMap<String, Arc<RwLock<Box<dyn McpClient>>>>,  // 细粒度锁
    tool_infos: DashMap<String, Vec<McpToolInfo>>,              // 无锁读
    runtime_states: DashMap<String, ServerRuntimeState>,         // 无锁读
}
```

---

## 6. 统一语言（Ubiquitous Language）

下表定义了 octo-sandbox 领域的核心术语，应在代码、注释、文档和团队沟通中统一使用：

| 术语 | 定义 | 对应代码类型 |
|------|------|-------------|
| **Agent** | 具有自主执行能力的智能体单元，由 Manifest 定义，由 Executor 运行 | `AgentManifest`, `AgentExecutor` |
| **Manifest** | Agent 的静态规格定义，包含角色、目标、工具过滤器 | `AgentManifest` |
| **Session** | 单次用户-Agent 对话会话，拥有独立消息历史 | `SessionData` |
| **Round** | 单轮对话循环：上下文构建 → LLM 调用 → 工具执行 → 重复 | `AgentLoop` |
| **Working Memory** | 当前对话窗口中的活跃上下文（Layer 0） | `WorkingMemory` |
| **Memory Store** | 跨 Session 的持久化长期知识（Layer 2） | `MemoryStore` |
| **Knowledge Graph** | 结构化实体-关系知识图谱（Layer 3） | `KnowledgeGraph` |
| **Tool** | Agent 可调用的功能单元（内置/MCP/技能） | `Tool` trait |
| **Skill** | 可热重载的 YAML 定义工具 | `SkillManifest` |
| **MCP Server** | 外部 Model Context Protocol 服务器 | `McpManager`, `McpClient` |
| **Provider** | LLM 服务提供者（Anthropic, OpenAI） | `Provider` trait |
| **Provider Chain** | 多 Provider 故障转移/负载均衡链 | `ProviderChain` |
| **Autonomy Level** | Agent 的自主执行权限级别 | `AutonomyLevel` |
| **Security Policy** | 控制命令执行、路径访问、频率限制的规则集 | `SecurityPolicy` |
| **Tenant** | 多租户场景中的独立隔离单元 | `TenantContext` |
| **Workspace** | Agent 文件操作的受限目录范围 | `working_dir` |
| **Metering** | Token 用量的计量与统计 | `Metering` |
| **Event Bus** | 系统内部事件发布/订阅总线 | `EventBus` |

---

## 7. 总结与优先级建议

### 7.1 发现的主要问题

| 问题 | 严重程度 | 影响范围 |
|------|----------|----------|
| `AgentRuntime` God Object（16 字段） | 高 | 可测试性、可维护性 |
| `ExecPolicy` 与 `SecurityPolicy` 职责重叠 | 中 | 安全策略一致性 |
| `TenantContext` 归属模糊 | 中 | 认证授权与执行上下文耦合 |
| `McpManager` 粗粒度全局锁 | 中 | 并发性能 |
| 缺少完整的系统级领域事件 | 中 | 可观测性、事件溯源 |
| `StdMutex` 在异步上下文的使用 | 低-中 | 潜在死锁风险 |

### 7.2 重构优先级

**第一优先级（架构安全性）**：
- 将 `ExecPolicy` 合并到 `SecurityPolicy`，消除安全职责重叠

**第二优先级（架构可维护性）**：
- 引入 `ExecutionContext` 和 `ObservabilityContext` 门面，精简 `AgentRuntime` 字段数量
- 将 `TenantContext` 从 `AgentRuntime` 字段移出，改为方法参数传入

**第三优先级（性能优化）**：
- `McpManager` 内部改用 `DashMap` 实现细粒度锁

**第四优先级（可观测性增强）**：
- 实现完整的系统级 `SystemEvent` 领域事件枚举
- 为 `EventBus` 添加事件持久化支持

---

*本文档基于 2026-03-06 代码快照分析生成，分析范围：`crates/octo-engine/src/` 全部核心模块。*
