# Phase BE — EAASP L1 Runtimes + Protocol Foundation

> **Started**: 2026-04-05 17:50
> **Updated**: 2026-04-06 00:00（W1 完成 — runtime.proto v1.2 + hook.proto v1.0）
> **Status**: W1 完成 — proto 全局化完成，W2 HookBridge Rust 核心待实现
> **Context**: Brainstorming output from `/dev-phase-manager:start-phase` + `/brainstorming` + web research (2026-04-05)

---

## 1. 背景与目标

### 1.1 EAASP 四层架构（已修正）

```
EAASP 平台（L4 人机协作 → L3 治理 → L2 统一资产层 → L1 执行）

L3 治理层
  ├── Hook 策略下发（原子化，session 级生效）
  ├── Session 生命周期管理
  ├── Quota 控制
  └── Telemetry 收集聚合
         │
         │ 13 方法契约（gRPC + managed_hooks_json）
         │
L2 统一资产层（统一管理，部署为独立 MCP Server）
  ├── Skills      ──→ SKILL.md 格式 ──→ MCP Server
  ├── MCP Servers ──→ 原生 MCP Server ──→ MCP Server
  └── Ontology   ──→ 本体对象服务 ────→ MCP Server
  （逻辑统一管理，部署为三个独立区域）
         │
         │ MCP 协议（stdio / gRPC）
         │
L1 运行时（各运行时内部有独立 Skill System 和 MCP Client）
    ┌────┴─────┬──────────┬──────────┐
    │          │          │          │
grid-runtime  Claude-RT  LangGraph  AutoGen
(Rust, T1)   (Py/TS, T1) (Python, T2)(Python, T3)
 本地hooks    本地hooks   HookBridge  HookBridge
```

### 1.1.1 架构关键澄清（用户明确）

> **L2 统一资产层**：Skills / MCP / Ontology 由 L2 统一管理，全部暴露为 MCP Server 给 L1。
> 不存在独立的"Skills Engine"或"MCP Gateway"——它们在 L2 是同一种资产（Skill/MCP/Ontology），在 L2 MCP Server Layer 是同一种部署形式（MCP Server）。

> **L1 内部有独立子系统**：L1 运行时内部有 Skill System（MCP Client 之上）和 MCP Client（MCP 协议）两个独立子系统。Skill System 负责把 SKILL.md 格式转换为 L1 自己可执行的格式；MCP Client 负责处理 MCP 协议。

> **Skills 格式**：标准 Agent Skills 格式（SKILL.md），各 L1 运行时有责任将其转换为自己的可执行形式。

- **EAASP L1 运行时池**：多个运行时并存，平台按策略选择
- **L3 尚未建立**：所有层的 mock/模拟工作同期进行
- **Phase BD**：grid-runtime（W1 done, 02dfa82），W2~W6 pending

### 1.2 Phase BE 目标

1. **claude-code-runtime**（Python/TS）：Tier 1 Harness，基于 claude-agent-sdk，实现 EAASP L1 Runtime 最佳样板
2. **HookBridge**（Rust）：跨语言共享，Tier 2/3 必需，Tier 1 端到端测试工具
3. **eaasp-certifier**：Mock L3/L4，13 方法契约验证工具
4. **全局 proto**：所有 runtime 共享的协议定义，与 crate 解耦

### 1.3 claude-code-runtime 双重角色

| 目标 | 说明 | Tier | Hook 执行 |
|------|------|------|---------|
| **A**: 真实 EAASP L1 Runtime | 使用 claude-agent-sdk 作为 Agent 引擎，production quality | Harness (Tier 1) | 本地（SDK hooks），不用 HookBridge |
| **B**: Tier 2/3 参考实现 | 靠 HookBridge + eaasp-certifier 本身作为参考，任何 Tier 2/3 adapter 只需实现 13 方法 + 连 HookBridge | N/A | N/A |

> **结论**：不需要"另一个 runtime"来为 Tier 2/3 打样。HookBridge + eaasp-certifier 就是最好的参考。

---

## 2. 全局协议设计

### 2.0 L1 运行时内部子系统（已修正）

```
L1 运行时内部
├── Agent Loop              # 核心执行循环（各 runtime 独立实现）
├── Tool System（统一）      # 所有工具统一路由
│   ├── Skill System       # 处理 SKILL.md → 转换为自己可执行格式
│   │   └── 连接 L2 MCP Server Layer（via MCP Client）
│   └── MCP Client         # 处理 MCP 协议
│       └── 连接 L2 MCP Server Layer（MCP Servers: Skills/MCP/Ontology）
└── Hook Executor          # Tier 1 本地执行 / Tier 2/3 经 HookBridge
```

> **关键**：Skill System ≠ MCP Client。Skill System 关注"如何将 SKILL.md 转换为运行时可执行的 skill"，MCP Client 关注"如何通过 MCP 协议调用 MCP Server"。两者都可以调用 L2 MCP Server Layer 的服务，但目的不同。

### 2.1 Proto 目录结构（repo 顶层，全局共享）

```
proto/                              # 全局共享，与 crate 解耦
├── eaasp/
│   ├── runtime/v1/
│   │   ├── runtime.proto           # 13 方法契约（L1 ↔ L3）
│   │   └── buf.yaml               # protobuf 管理
│   ├── hook/v1/
│   │   └── hook.proto              # HookBridge ↔ L1（L3 下发 hook 策略）
│   └── registry/v1/
│       └── registry.proto          # L2 Registry MCP Server 注册协议（新增）
├── README.md                       # proto 版本策略说明
└── buf.lock
```

**proto 版本策略：**
- `runtime.proto` 变更 = 重大版本，需所有 runtime 同步升级
- 使用 `buf breaking` CI 检查兼容性
- 每个子目录独立版本递增

**当前 grid-runtime 的 proto 在 crate 内部（需迁移）：**
```
错误：crates/grid-runtime/proto/...
正确：proto/eaasp/runtime/v1/runtime.proto（迁移）
```

### 2.2 13 方法契约 (`runtime.proto`)

#### 2.2.1 服务定义

```protobuf
service RuntimeService {
  rpc Initialize(InitializeRequest) returns (InitializeResponse);
  rpc Send(SendRequest) returns (stream ResponseChunk);
  rpc LoadSkill(LoadSkillRequest) returns (LoadSkillResponse);
  rpc OnToolCall(ToolCallEvent) returns (HookDecision);        // Tier 1: 空操作
  rpc OnToolResult(ToolResultEvent) returns (HookDecision);   // Tier 1: 空操作
  rpc OnStop(StopRequest) returns (StopDecision);
  rpc GetState(GetStateRequest) returns (SessionState);
  rpc RestoreState(SessionState) returns (InitializeResponse);
  rpc ConnectMcp(ConnectMcpRequest) returns (ConnectMcpResponse);
  rpc DisconnectMcp(DisconnectMcpRequest) returns (DisconnectMcpResponse); // 新增
  rpc EmitTelemetry(EmitTelemetryRequest) returns (TelemetryBatch);
  rpc GetCapabilities(Empty) returns (CapabilityManifest);
  rpc Terminate(TerminateRequest) returns (TerminateResponse);
  rpc Health(Empty) returns (HealthStatus);
  rpc PauseSession(PauseRequest) returns (PauseResponse);      // 新增
  rpc ResumeSession(ResumeRequest) returns (ResumeResponse);   // 新增
}
```

#### 2.2.2 关键类型变更

```protobuf
// CapabilityManifest 新增字段（区分 Tier）
message CapabilityManifest {
  string runtime_id = 1;
  string runtime_name = 2;
  string tier = 3;                  // "harness" | "aligned" | "framework"
  bool requires_hook_bridge = 4;   // true = Tier 2/3, false = Tier 1
  // ... 其余字段不变
}

// SessionPayload 新增字段
message SessionPayload {
  string user_id = 1;
  string user_role = 2;
  string org_unit = 3;
  string managed_hooks_json = 4;
  map<string, string> quotas = 5;
  map<string, string> context = 6;
  string hook_bridge_url = 7;       // 可选，L3 下发，优先级高于环境变量
  string telemetry_endpoint = 8;     // 可选，L3 telemetry 接收地址
}

// 新增 Pause/Resume
message PauseRequest { string session_id = 1; }
message PauseResponse { bool success = 1; }
message ResumeRequest { string session_id = 1; }
message ResumeResponse { bool success = 1; string session_id = 2; }

// 新增 DisconnectMcp
message DisconnectMcpRequest { string session_id = 1; string server_name = 2; }
message DisconnectMcpResponse { bool success = 1; }

// SessionState 标注格式
message SessionState {
  string session_id = 1;
  bytes state_data = 2;
  string runtime_id = 3;
  string state_format = 4;   // "rust serde v1" | "python json" | "ts json"
  string created_at = 5;
}
```

#### 2.2.3 Tier 1 / Tier 2 / Tier 3 划分（已修正，2026-04-05）

**Tier 划分核心标准（用户明确）：**

| Tier | 定义 | EAASP 13 契约 | Hook 机制 | 接入成本 |
|------|------|:--------------:|---------|---------|
| **Tier 1** | 完整 Harness 实现，完美映射 EAASP 13 契约 | 直接实现 ✅ | 本地 native hooks | **零成本** |
| **Tier 2** | 基本完善的 Harness，可通过 HookBridge 补全缺失的 hooks | 需 adapter 层 ⚠️ | HookBridge 补全 | **低** |
| **Tier 3** | 传统 AI 框架，非原生 Harness | 需大量实现工作 ❌ | HookBridge 强制 | **高** |

**Tier 1 Hook 执行策略：**

```
┌──────────────────────────────────────────────────────────────────┐
│  Tier 1 (Harness): grid-runtime / claude-code-runtime           │
│  → Hook 在进程内执行（SDK native hooks）                         │
│  → OnToolCall / OnToolResult 永远返回 Allow                     │
│  → managed_hooks_json 作为策略种子，本地解析执行                 │
│  → HookBridge 仅用于端到端测试（HookBridge 是 Tier 1 的测试工具）│
│                                                                  │
│  Tier 2 (Aligned Harness): LangGraph / OpenDev / Vercel AI SDK  │
│  → 基本 harness 实现，HookBridge 补全缺失 hooks                 │
│  → OnToolCall / OnToolResult gRPC 调用 HookBridge               │
│  → HookBridge 执行 hook 策略，返回决策                          │
│                                                                  │
│  Tier 3 (Framework): AutoGen / PydanticAI / Semantic Kernel     │
│  → 传统 AI 框架，非原生 Harness                                 │
│  → 大量 EAASP 13 契约实现工作                                   │
│  → HookBridge 强制接入                                           │
└──────────────────────────────────────────────────────────────────┘
```

**优点：**
- Tier 1 零网络开销（延迟最低）
- HookBridge 是 Tier 2/3 必需品，Tier 1 可选
- L3 通过 `GetCapabilities().requires_hook_bridge` 判断是否需要 HookBridge
- Tier 1 的 eaasp-certifier 验证对 Tier 2/3 adapter 层同样有效

### 2.3 HookBridge 协议 (`hook.proto`)

```
proto/eaasp/hook/v1/hook.proto

service HookBridgeService {
  // L1 → HookBridge: 请求 hook 决策（Tier 2/3 用）
  rpc EvaluateHook(HookRequest) returns (HookResponse);

  // L1 → HookBridge: 上报 hook 执行审计（Tier 1 端到端测试用）
  rpc ReportHookDecision(HookAuditEvent) returns (Empty);

  // L3 → HookBridge: 下发/更新 hook 策略（运行时由 L3 管理）
  rpc UpdatePolicies(stream HookPolicy) returns (stream PolicyAck);
}
```

```protobuf
message HookRequest {
  string session_id = 1;
  string hook_type = 2;        // "pre_tool_call" | "post_tool_result" | "on_stop"
  string tool_name = 3;
  string input_json = 4;
  string user_id = 5;
  string timestamp = 6;
}

message HookResponse {
  string decision = 1;          // "allow" | "deny" | "modify"
  string reason = 2;
  string modified_input = 3;
}

message HookAuditEvent {
  string session_id = 1;
  string hook_type = 2;
  string tool_name = 3;
  string decision = 4;
  string reason = 5;
  string runtime_id = 6;
  string timestamp = 7;
}

message HookPolicy {
  string policy_id = 1;
  string hook_type = 2;
  string pattern = 3;           // glob 或正则
  string action = 4;            // "allow" | "deny" | "modify"
  string conditions_json = 5;
}

message PolicyAck {
  string policy_id = 1;
  bool applied = 2;
  string error = 3;
}
```

### 2.4 HookBridge 发现机制

| 机制 | 优先级 | 适用场景 |
|------|--------|---------|
| `SessionPayload.hook_bridge_url` | 高（环境变量 fallback） | 生产环境，L3 下发 |
| `HOOK_BRIDGE_ADDR` 环境变量 | 中（默认） | 开发/测试 |
| `localhost:50051` | 低（硬编码 fallback） | 本地开发 |

---

## 3. claude-agent-sdk 分析

### 3.1 SDK 概述

**官方 Anthropic SDK**，将 Claude Code CLI 作为库暴露。Python v0.1.56（6.1k ⭐），TypeScript v0.2.92（1.2k ⭐）。

**关键架构事实：**
- SDK 在底层 **spawn Claude Code CLI 作为 subprocess**（stdin/stdout IPC），不是纯 HTTP 客户端
- 你无法替换底层模型提供者
- 它继承了完整的 Claude Code 工具集和 Agent Loop

### 3.2 SDK → 13 方法映射

| 13 方法 | SDK 对应 | 实现路径 |
|---------|---------|---------|
| `initialize` | `ClaudeSDKClient` 创建 | SDK session = L1 session |
| `send` | `query()` / `receive_response()` | async generator → ResponseChunk stream |
| `load_skill` | `system_prompt` fragment 或 MCP tool | SKILL.md prose → system prompt |
| `on_tool_call` | `PreToolUse` hook（同步） | Tier 1 本地执行，返回 Allow；Tier 2/3 走 HookBridge |
| `on_tool_result` | `PostToolUse` hook | 同上 |
| `on_stop` | `SessionEnd` hook | SDK hook → StopDecision |
| `get_state` | 不暴露内部状态 | 自定义 JSON 序列化（session_id + 对话历史） |
| `restore_state` | `resume=session_id` | 基本支持 |
| `connect_mcp` | `mcp_servers` dict | SDK 原生 stdio MCP；gRPC → stdio 桥接 |
| `emit_telemetry` | 无原生支持 | SDK callback 收集 + TelemetryEvent 上报 |
| `get_capabilities` | 无原生支持 | 全新定义 |
| `terminate` | client 关闭 | 基本支持 |
| `health` | 无原生支持 | provider ping |
| `pause/resume` | 无原生支持 | `max_turns` 控制 |

### 3.3 Python vs TypeScript

| 维度 | Python | TypeScript |
|------|--------|-----------|
| SDK 成熟度 | ⭐⭐⭐ v0.1.56, 6.1k ⭐ | ⭐ v0.2.92, 1.2k ⭐ |
| gRPC 库 | grpcio + grpcio-tools（成熟） | @grpc/grpc-js（中等） |
| subprocess 管理 | asyncio subprocess（标准） | Node.js child_process（标准） |
| 国内 AI 生态 | 更强 | 一般 |
| **推荐顺序** | **先做** | **后做（参照 Python）** |

---

## 4. 目录结构

```
lang/                                   # 新顶层目录
├── claude-code-runtime-python/          # Python 版本（T1 Harness）
│   ├── pyproject.toml
│   ├── build.py                         # grpcio-tools 从 proto/ 生成
│   ├── src/
│   │   ├── __main__.py                 # gRPC server 入口
│   │   ├── sdk_wrapper.py              # claude-agent-sdk 封装
│   │   ├── grpc_service.py             # 13 方法 gRPC service
│   │   ├── hook_executor.py             # 本地 hook 执行（Tier 1）
│   │   ├── hook_bridge_client.py       # HookBridge gRPC 客户端（可选，测试用）
│   │   ├── telemetry.py                # SDK callback → TelemetryEvent
│   │   ├── state_manager.py            # get_state / restore_state
│   │   ├── mapper.py                   # SDK event ↔ ResponseChunk 转换
│   │   ├── capabilities.py             # get_capabilities 实现
│   │   ├── skill_system.py             # SKILL.md → L1 可执行格式转换（新增）
│   │   └── mcp_client.py               # L2 MCP Server Layer 连接（新增）
│   └── tests/
│
├── claude-code-runtime-ts/              # TypeScript 版本（参照 Python）
│   ├── package.json
│   ├── build.ts                        # protoc-gen-ts 从 proto/ 生成
│   ├── src/
│   │   ├── main.ts                    # gRPC server 入口
│   │   ├── sdk-wrapper.ts             # claude-agent-sdk 封装
│   │   ├── grpc-service.ts            # 13 方法 gRPC service
│   │   ├── hook-executor.ts           # 本地 hook 执行（Tier 1）
│   │   ├── hook-bridge-client.ts      # HookBridge 客户端（测试用）
│   │   ├── telemetry.ts
│   │   ├── state-manager.ts
│   │   ├── mapper.ts
│   │   ├── capabilities.ts
│   │   ├── skill-system.ts            # SKILL.md → L1 格式（新增）
│   │   └── mcp-client.ts              # L2 MCP Server Layer 连接（新增）
│   └── tests/
│
tools/
├── eaasp-certifier/                    # Mock L3/L4 + 契约验证
│   ├── Cargo.toml
│   ├── build.rs                        # 从 proto/ 生成
│   ├── src/
│   │   ├── main.rs                    # CLI: verify / mock-l3 / mock-l4
│   │   ├── verifier.rs                # 13 方法逐一验证
│   │   ├── mock_l3.rs               # Mock L3: hook 下发 + telemetry 收集
│   │   ├── mock_l4.rs               # Mock L4: 三方握手 + 会话交互
│   │   ├── report.rs                  # 认证报告生成
│   │   └── l3_client.rs              # L3Client trait（L3 接口抽象）
│   └── test_fixtures/
│       ├── sample_hooks.json
│       └── sample_skill.md
│
proto/                                  # 全局共享（从 crates/ 迁移）
├── eaasp/
│   ├── runtime/v1/
│   │   ├── runtime.proto              # 13 方法契约（扩展版）
│   │   ├── buf.yaml
│   │   └── runtime_grpc.rs           # 生成的 Rust 类型
│   ├── hook/v1/
│   │   ├── hook.proto                # HookBridge ↔ L1 协议
│   │   ├── buf.yaml
│   │   └── hook_grpc.rs
│   └── registry/v1/
│       ├── registry.proto            # L2 MCP Server 注册协议（新增，待定义）
│       └── buf.yaml
└── README.md
```

---

## 5. 实施计划

### Phase BE 实施 Waves

| Wave | 内容 | 产出 | 依赖 |
|------|------|------|------|
| **W1** ✅ | **proto 全局化 + 扩展** | `proto/eaasp/` (runtime v1.2 + hook v1) + buf.yaml + proto/README.md + Rust contract.rs 同步 | 无 |
| **W2** | **HookBridge Rust 核心** | `hook.proto` 实现，`hook-bridge` crate，`EvaluateHook`/`ReportHookDecision` | W1 |
| **W3** | **eaasp-certifier 核心** | Mock L3/L4，`L3Client` trait，13 方法验证 | W1 |
| **W4** | **Enterprise SDK Python W1** | 项目骨架 + 抽象层 + Agent/Skill/Tool 核心 + proto 生成 | W1 |
| **W5** | **Enterprise SDK Python W2** | Policy + Playbook + Session + Message + YAML 配置 | W4 |
| **W6** | **Enterprise SDK Python W3** | SDK 测试 + 多语言规范(specs/) + gRPC service + Dockerfile | W5 |
| **W7** | **Enterprise SDK TypeScript** | 参照 Python 规范实现 | W5, W6 |
| **W8** | **集成验证** | eaasp-certifier 验证所有 runtime 契约合规性 | W2, W3, W6, W7 |

### 并行策略

```
W1 ─────────────────────────────────────────────────→ W1 完成
  │
  ├─ W2 ──────────────────────────→ W2 完成
  │       │
  │       └─ W3 ─────────────────→ W3 完成
  │
  └─ W4 ──────────────────────────→ W4 完成
          │
          └─ W5 ───┬─ W6 ───┬─ W7 ──→ W8 (集成)
                    │         │
                    └─────────┘
                   Python 完成后
                   TS 可独立跟进
```

---

## 6. 各组件详细设计

### 6.1 claude-code-runtime Python 核心实现

#### SDK → ResponseChunk 映射

```python
# sdk_wrapper.py

from claude_agent_sdk import ClaudeSDKClient, ClaudeAgentOptions

async def send_stream(session_id: str, message: UserMessage) -> AsyncIterator[ResponseChunk]:
    """SDK query() → ResponseChunk stream"""
    client = self.sessions[session_id]
    async for sdk_msg in client.query(prompt=message.content):
        if isinstance(sdk_msg, AssistantMessage):
            for block in sdk_msg.content:
                if hasattr(block, 'text'):
                    yield ResponseChunk(
                        chunk_type='text_delta',
                        content=block.text,
                    )
                elif block.type == 'thinking':
                    yield ResponseChunk(chunk_type='thinking', content=block.thinking)
                elif block.type == 'tool_use':
                    yield ResponseChunk(
                        chunk_type='tool_start',
                        content='',
                        tool_name=block.name,
                        tool_id=block.id,
                    )
        elif isinstance(sdk_msg, ToolResultMessage):
            yield ResponseChunk(
                chunk_type='tool_result',
                content=sdk_msg.content,
                tool_name=sdk_msg.tool_name,
                tool_id=sdk_msg.tool_id,
            )
        elif isinstance(sdk_msg, ResultMessage):
            if sdk_msg.subtype == 'success':
                yield ResponseChunk(chunk_type='done', content='')
            else:
                yield ResponseChunk(
                    chunk_type='error',
                    content=sdk_msg.content,
                    is_error=True,
                )
```

#### 本地 Hook 执行（Tier 1）

```python
# hook_executor.py

from pydantic import BaseModel
import json

class LocalHookPolicy(BaseModel):
    hook_type: str
    pattern: str      # glob pattern
    action: str       # "allow" | "deny"
    conditions: dict

class LocalHookExecutor:
    def __init__(self, managed_hooks_json: str | None):
        self.policies: list[LocalHookPolicy] = []
        if managed_hooks_json:
            raw = json.loads(managed_hooks_json)
            for p in raw.get('policies', []):
                self.policies.append(LocalHookPolicy(**p))

    def evaluate(self, hook_type: str, tool_name: str, input_json: str) -> HookDecision:
        for policy in self.policies:
            if policy.hook_type == hook_type and fnmatch(tool_name, policy.pattern):
                if policy.action == 'deny':
                    return HookDecision(decision='deny', reason=policy.conditions.get('reason', ''))
                elif policy.action == 'modify':
                    modified = self._apply_modify(input_json, policy.conditions)
                    return HookDecision(decision='modify', modified_input=modified)
        return HookDecision(decision='allow')

    def _apply_modify(self, input_json: str, conditions: dict) -> str:
        # apply sanitization / redaction rules
        ...
```

### 6.2 eaasp-certifier 设计

```rust
// l3_client.rs

/// L3 接口抽象，Mock 和 Real 两种实现
pub trait L3Client: Send + Sync {
    async fn initialize(&self, payload: SessionPayload) -> Result<String>;
    async fn receive_telemetry(&self, events: Vec<TelemetryEvent>) -> Result<()>;
    async fn report_hook_decision(&self, audit: HookAuditEvent) -> Result<()>;
}

/// Mock L3Client（开发期用）
pub struct MockL3Client {
    policies: Vec<HookPolicy>,
}

impl MockL3Client {
    pub fn new() -> Self { ... }
}

impl L3Client for MockL3Client {
    // 内存中模拟 L3 行为
}

impl L3Client for RealL3Client {
    // 真实 gRPC 调用
}
```

```rust
// verifier.rs

pub struct EaaspCertifier<L3: L3Client> {
    l3: L3,
    runtime_addr: SocketAddr,
}

impl<L3: L3Client> EaaspCertifier<L3> {
    /// 逐一验证 13 方法
    pub async fn verify(&self) -> VerificationReport { ... }
}
```

```bash
# CLI 用法
eaasp-certifier verify grpc://localhost:50051
  # → Mock L3 自动创建 session，执行 send，验证 chunk 类型
  # → 输出: PASS/FAIL per method + 详细报告

eaasp-certifier mock-l3 --port 50051
  # → 启动 Mock L3（独立进程，HookBridge 测试用）
```

### 6.3 HookBridge Rust 设计

```rust
// hook_executor.rs

#[derive(Clone)]
pub struct HookBridgeServer {
    policies: Arc<RwLock<Vec<HookPolicy>>>,
    audit_tx: mpsc::Sender<HookAuditEvent>,
}

impl HookBridgeService for HookBridgeServer {
    async fn evaluate_hook(
        &self,
        request: Request<HookRequest>,
    ) -> Result<Response<HookResponse>, Status> {
        let req = request.into_inner();
        let decision = self.evaluate(&req).await;
        Ok(Response::new(decision.into()))
    }

    async fn report_hook_decision(
        &self,
        request: Request<HookAuditEvent>,
    ) -> Result<Response<()>, Status> {
        let audit = request.into_inner();
        self.audit_tx.send(audit).await.ok();
        Ok(Response::new(()))
    }
}
```

---

## 7. 关键技术决策

| # | 决策 | 理由 |
|---|------|------|
| KD1 | proto 升到 repo 顶层 | 全局共享，与 crate 解耦 |
| KD2 | Tier 1 本地 hook 执行，Tier 2/3 走 HookBridge | Tier 1 零网络开销，HookBridge 是 Tier 2/3 必需品 |
| KD3 | HookBridge Rust 独立实现 | 跨语言复用，性能最优 |
| KD4 | Python 先，TS 后 | SDK Python 更成熟，国内生态更强 |
| KD5 | eaasp-certifier 使用 `L3Client` trait | Mock 和 Real 实现可替换 |
| KD6 | `managed_hooks_json` 内嵌在 `SessionPayload` | 务实，无需定义 hook schema |
| KD7 | HookBridge 发现：环境变量 + `SessionPayload` fallback | 开发/生产两相宜 |
| KD8 | HookBridge 服务端点用 `hook.proto`（独立于 `runtime.proto`） | 关注点分离，HookBridge 可独立演进 |
| KD9 | claude-code-runtime 不需要 HookBridge 做 hook 执行 | Tier 1 Harness，本地执行；HookBridge 仅用于测试 |
| KD10 | L2 = 统一资产层，Skills/MCP/Ontology 全部暴露为 MCP Server | 逻辑统一管理，部署为三个独立 MCP Server 区域 |
| KD11 | L1 Skill System ≠ MCP Client | Skill System 处理 SKILL.md 转换；MCP Client 处理 MCP 协议；两者独立 |
| KD12 | Skills 格式 = 标准 Agent Skills（SKILL.md），各 L1 负责转换 | SKILL.md 是统一的，L1 负责到自己格式的转换 |
| KD13 | Tier 1 = 完整 Harness 实现，零成本 EAASP 13 契约对接 | NanoBot/OpenClaw/Vercel AI SDK 原生支持 SKILL.md，grid-runtime/claude-code-runtime 原生实现 |
| KD14 | Tier 2 = 基本 Harness + HookBridge 补全 hooks | LangGraph/OpenDev/Vercel AI SDK 需 adapter 层 |
| KD15 | Tier 3 = 传统 AI 框架，非原生 Harness | AutoGen/PydanticAI/Semantic Kernel 需大量实现工作 |
| KD16 | Tier 3（Multi-Runtime Team）暂不定义 | EAASP L1 = 单 session 单 agent；L4 多 agent 是平台层事情 |
| KD17 | Tier 2 第一试点 = LangGraph，PydanticAI 第二 | Python 生态，checkpointer 成熟，企业采纳度高 |
| KD18 | "Harness" = EAASP L1 Runtime，与行业术语对齐 | Microsoft/OpenDev/OpenHarness 三方印证 |

---

## 8. 已知限制

| 限制 | 影响 | 应对 |
|------|------|------|
| 跨 runtime 会话迁移 | `get_state` 格式不统一 | 接受限制，state_data 为 opaque bytes |
| SDK subprocess 管理 | SDK CLI crash 需要重启 | SDK 层处理，gRPC session 感知 |
| L3 不存在 | 无法端到端测试 | Mock L3 + eaasp-certifier 覆盖 |
| Python gRPC subprocess | 在容器内需处理 stdio | Docker 隔离环境配置 |

---

## 8.1 L2 MCP Server Layer 架构问题（待用户确认）

| # | 问题 | 选项 | 待确认 |
|---|------|------|--------|
| OQ1 | L2 Registry 存储技术 | Git/YAML（轻量）vs PostgreSQL（企业）vs Nacos/Zookeeper | L3 部署在哪里 |
| OQ2 | MCP Server 认证方式 | 网络隔离（无认证）vs mcp-oidc（显式 AuthN） | L2 部署拓扑 |
| OQ3 | Ontology MCP Server 与普通 MCP Tool Server 的区别 | 语义层（元数据丰富）vs 工具层（功能调用）| L1 是否需要感知"是 Ontology" |
| OQ4 | SKILL.md 在 L2 的存储格式 | 仅 SKILL.md（prose）vs SKILL.md + 结构化工具定义（compiled） | L1 Skill System 转换成本 |
| OQ5 | L2 MCP Server 部署位置 | 与 L3 捆绑 vs 独立部署区域 | 三个独立部署区域的具体形态 |
| OQ6 | L3 完整职责边界 | MCP Gateway 功能是否归入 L3 | L3 ↔ L2 通讯协议 |

## 8.2 L1 运行时内部 Skill System 设计问题（待用户确认）

| # | 问题 | 关键考量 | 待确认 |
|---|------|---------|--------|
| OQ7 | SKILL.md → L1 格式的转换时机 | Initialize 时预转换 vs Send 时按需转换 | 性能与内存权衡 |
| OQ8 | L2 MCP Server 的发现机制 | L3 下发 server list vs L1 主动注册到 L2 Registry | L2 Registry 的能力 |
| OQ9 | Skill System 与 MCP Client 的调用关系 | Skill System 调用 MCP Client vs Skill System 独立 | L1 内部架构 |
| OQ10 | Pause/Resume 对 L2 MCP Server 连接的影响 | 保持连接 vs 重新连接 | L1 ↔ L2 契约 |

## 8.3 L3 治理层职责边界（待用户确认）

| # | 问题 | 关键考量 | 待确认 |
|---|------|---------|--------|
| OQ11 | L3 是否负责 L2 MCP Server 的路由 | L3 下发 MCP server list vs L1 直接访问 L2 Registry | 架构层次 |
| OQ12 | L3 的 MCP Gateway 具体做什么 | 协议转换 vs 认证代理 vs 流量控制 | L3 职责细化 |
| OQ13 | Telemetry 上报的详细程度 | 工具调用粒度 vs Session 聚合粒度 | L3 数据需求 |
| OQ14 | SKILL.md 是否足够作为各框架通用 Skill 输入格式 | 需结构化中间格式 vs SKILL.md prose 足够 | NanoBot/OpenClaw/Vercel AI SDK 原生支持 |
| OQ15 | 各 Tier 1 框架 `get_state` 是否需要统一 schema | 接受跨框架不兼容 vs 定义 `BaseState` 规范 | 框架调研见第 10 节 |

---

## 9. Tier 1/2/3 框架调研（网络研究，2026-04-05）

> 调研时间：2026-04-05
> 数据来源：NanoBot 文档（mintlify.com/HKUDS/nanobot）、OpenClaw 文档（docs.openclaw.ai）、
> OpenDev 论文（arXiv 2603.05344）、Microsoft Agent Framework 博客、Vercel AI SDK 文档

### 9.1 Tier 1 框架详细分析

#### 9.1.1 grid-runtime（Tier 1 ✅，原生）

| 维度 | 详情 |
|------|------|
| **语言** | Rust |
| **EAASP 13 契约** | 从设计阶段就按 EAASP 契约实现，完美对齐 ✅ |
| **Hook 机制** | 本地 native hooks，进程内执行 |
| **Skill 系统** | 需 adapter 层，SkillContent → runtime 格式 |
| **状态管理** | `serde` 序列化，`state_data` opaque bytes |
| **Tier 判定理由** | 原生实现，EAASP 契约定义者之一 |

#### 9.1.2 claude-code-runtime / claude-agent-sdk（Tier 1 ✅，SDK = 完整 harness）

| 维度 | 详情 |
|------|------|
| **语言** | Python (v0.1.56, 6.1k ⭐) / TypeScript (v0.2.92, 1.2k ⭐) |
| **EAASP 13 契约** | 完整封装 SDK，零成本映射 13 方法 ✅ |
| **Hook 机制** | SDK PreToolUse / PostToolUse hooks（Tier 1 本地执行）|
| **Skill 系统** | `load_skill` → SKILL.md prose → system prompt fragment |
| **状态管理** | SDK session 封装，`max_turns` 控制 pause/resume |
| **Tier 判定理由** | claude-agent-sdk = 完整 Claude Code harness，零成本映射 EAASP |

#### 9.1.3 NanoBot（Tier 1 ✅，~4000 行，SKILL.md native）

| 维度 | 详情 |
|------|------|
| **语言** | Python |
| **来源** | HKUDS，MIT License，2026-03 开源 |
| **代码量** | ~4,000 行（比 OpenClaw 的 430k+ 行轻量化 99%）|
| **核心组件** | AgentLoop + SkillsLoader + Tools System + Memory System |
| **Skill 系统** | **SKILL.md native** — `SkillsLoader` 类（`nanobot/agent/skills.py`），支持：|
|  | - YAML frontmatter metadata（`always`, `requires.bins`, `requires.env`）|
|  | - progressive loading（按需注入 context）|
|  | - XML skills summary 生成（注入 system prompt）|
|  | - ClawHub 集成（公共 Skill registry 发现和安装）|
| **MCP 集成** | MCP Configuration ✅，MCP Registry 集成外部工具 ✅ |
| **Hook 机制** | 无原生 hook API ⚠️（Tier 1 本地执行，非强制）|
| **Tool 系统** | `Tools System` 独立子系统，channel 无关 |
| **状态管理** | Session-based，无标准化跨框架序列化 |
| **Tier 判定理由** | 完整 harness 实现，SKILL.md native，~4k 行 adapter 工作量小 |
| **参考实现** | `SkillsLoader` 类 → 可直接对接到 L2 Skill System |

**NanoBot SKILL.md 格式示例：**

```markdown
---
title: Web Research
description: Search and extract information from the web
metadata: |
  {
    "nanobot": {
      "always": false,
      "requires": {
        "env": ["BRAVE_API_KEY"]
      }
    }
  }
---

# Web Research Skill

This skill teaches you how to perform effective web research...

## Tools

### web_search
**Parameters:**
- `query` (string, required): The search query

## Workflow
1. Use `web_search` to find relevant sources
2. Use `web_fetch` to extract content
```

#### 9.1.4 OpenClaw（Tier 1 ✅，最完整的 SKILL.md 实现）

| 维度 | 详情 |
|------|------|
| **语言** | TypeScript / Node |
| **来源** | Claude Code 原生实现方，最成熟 |
| **Skill 系统** | **AgentSkills-compatible SKILL.md 最完整实现**，支持：|
|  | - YAML frontmatter（name, description, metadata）|
|  | - `metadata.openclaw` 扩展（requires.bins/anyBins/env/config）|
|  | - security scan（dangerous-code scanner）|
|  | - skill allowlist（`agents.defaults.skills`）|
|  | - multi-agent skill scoping（per-agent / shared / project）|
|  | - plugin 打包（`openclaw.plugin.json`）|
|  | - ClawHub（clawhub.ai）公共 registry |
|  | - 6 层加载优先级（extra → bundled → managed → personal → project → workspace）|
|  | - hot reload（skills watcher，`watchDebounceMs`）|
|  | - token impact 公式（195 base + 97/skill chars）|
| **MCP 集成** | MCP 原生支持 ✅ |
| **Hook 机制** | 内置 approval（非 external hook API），skill 级别 gate |
| **Tool 系统** | 26 built-in tools，plugin tool 扩展 |
| **Tier 判定理由** | 最成熟的 SKILL.md + enterprise features，TS adapter 工作量中等 |
| **重要发现** | OpenClaw 的 ClawHub = NanoBot 的 ClawHub = Vercel AI SDK skill directory — **三方共享同一个公共 Skill registry 生态** |

**OpenClaw SKILL.md metadata 扩展：**

```markdown
---
name: gemini
description: Use Gemini CLI for coding assistance
metadata:
  {
    "openclaw": {
      "emoji": "♊️",
      "requires": { "bins": ["gemini"] },
      "install": [
        { "id": "brew", "kind": "brew", "bins": ["gemini"] }
      ]
    }
  }
---
```

### 9.2 Tier 2 框架详细分析

#### 9.2.1 LangGraph（Tier 2 ✅，Python，企业采纳度高）

| 维度 | 详情 |
|------|------|
| **语言** | Python |
| **架构** | Graph-based 状态机，`StateGraph` + `checkpointer` |
| **EAASP 13 契约** | 需 adapter 层 ⚠️（graph/node 定义与 session 不同）|
| **Hook 机制** | 有限（checkpointer 存储 hook 决策，非 external）⚠️ |
| **状态管理** | **checkpointer 成熟** — 任意序列化，支持 SQLite/Postgres/Memory |
| **Tool 系统** | tool executor 成熟，langchain-tools 生态丰富 |
| **Skill 系统** | tool 装饰器注册，无 SKILL.md native 支持 |
| **多 agent** | 多 graph 并行调用，各 graph 独立 checkpointer |
| **Tier 判定理由** | harness 基础完善，HookBridge 补全 hooks，接入成本低 |
| **推荐理由** | Python 生态最完整，checkpointer 成熟，LangChain 工具丰富 |

#### 9.2.2 OpenDev（Tier 2 ✅，Rust，Compound AI）

| 维度 | 详情 |
|------|------|
| **语言** | Rust |
| **来源** | arXiv 2603.05344（2026-03），Nghi D. Q. Bui |
| **架构** | Compound AI System — dual-agent（planning + execution）+ workload-specialized model routing |
| **核心特性** | - lazy tool discovery（按需发现）|
| | - adaptive context compaction（自适应 context 压缩）|
| | - event-driven system reminders（防止 instruction fade-out）|
| | - automated memory system（跨 session 累积项目知识）|
| **EAASP 13 契约** | 需 Rust adapter 层 ⚠️ |
| **Hook 机制** | 无外部 hook API ⚠️ |
| **状态管理** | session-based，无标准化序列化 |
| **Skill 系统** | 无 SKILL.md 概念，以 tool + system prompt 为核心 |
| **Tier 判定理由** | 技术上完整，但 Rust adapter 工作量大 |
| **不推荐优先** | Rust 重写成本高，建议在 Python Tier 2 稳定后再推进 |

#### 9.2.3 Vercel AI SDK（Tier 2 ✅，TS，轻量 skill system）

| 维度 | 详情 |
|------|------|
| **语言** | TypeScript / JavaScript |
| **架构** | `ai` SDK — `generateText` / `streamText` / `ToolLoopAgent` |
| **EAASP 13 契约** | 需 adapter 层 ⚠️ |
| **Hook 机制** | 有限 ⚠️ |
| **Skill 系统** | `skills` 目录，`SKILL.md` 读入 context（`ai-skills` npm）|
| **Tool 系统** | tool calling 原生支持，streaming 完善 |
| **Tier 判定理由** | skill 作为 context 注入，轻量，TS 生态丰富 |
| **备注** | Skill system 与 OpenClaw/NanoBot 功能对齐，但更轻量 |

### 9.3 Tier 3 框架详细分析

#### 9.3.1 AutoGen（Tier 3 ❌，conversation-based）

| 维度 | 详情 |
|------|------|
| **语言** | Python |
| **来源** | Microsoft，2023 开源，enterprise 用户多 |
| **架构** | Agent Group Chat — conversation-based，多 agent 通过消息传递协作 |
| **EAASP 13 契约** | 大量工作 ❌（conversation 模型与 EAASP session 模型差异大）|
| **Hook 机制** | conversation-based，非 hook 设计 ❌ |
| **状态管理** | Component configuration（declarative YAML/JSON 序列化）|
| **Skill 系统** | declarative component config，非 SKILL.md |
| **Tier 判定理由** | 设计哲学与 EAASP harness 差异大，adapter 工作量最大 |
| **不推荐优先** | agent group 概念强但模型差异大 |

#### 9.3.2 PydanticAI（Tier 3 ❌，type-safe）

| 维度 | 详情 |
|------|------|
| **语言** | Python |
| **架构** | Type-safe agent framework，结构化输出优先 |
| **EAASP 13 契约** | 大量工作 ❌ |
| **Hook 机制** | type-safe agent，hook 概念弱 ❌ |
| **Skill 系统** | tool 装饰器，structured agent，非 SKILL.md |
| **Tier 判定理由** | type-safe 优先，coding agent 场景非核心 |
| **不推荐优先** | 与 EAASP harness 模型差异大 |

#### 9.3.3 Semantic Kernel（Tier 3 ❌，.NET 专属）

| 维度 | 详情 |
|------|------|
| **语言** | C# / .NET |
| **架构** | Plugin architecture，enterprise 导向，Azure 集成 |
| **EAASP 13 契约** | 大量工作 ❌（.NET ↔ Python/Rust 技术栈差异）|
| **Hook 机制** | Plugin architecture，非 harness ❌ |
| **Skill 系统** | Plugin，非 SKILL.md |
| **Tier 判定理由** | .NET 专属，技术栈与 EAASP 差异最大 |
| **不推荐优先** | 偏离主要技术栈（Python/Rust/TypeScript） |

### 9.4 框架 Skill 格式兼容性矩阵

```
SKILL.md 标准格式（YAML frontmatter + markdown body）
     │
     ├── NanoBot  ✅ 原生支持（SkillsLoader）
     ├── OpenClaw ✅ 最完整（AgentSkills-compatible）
     ├── Vercel AI SDK ✅ 原生（ai-skills）
     ├── claude-agent-sdk ✅（SKILL.md → system prompt）
     ├── LangGraph ❌（tool 装饰器注册）
     ├── AutoGen ❌（declarative component config）
     ├── PydanticAI ❌（type-safe tool 装饰器）
     └── OpenDev ❌（无 SKILL.md 概念）

结论：SKILL.md 是 NanoBot/OpenClaw/Vercel AI SDK 三方的共同标准，
EAASP L2 Skill System 设计正确。
Tier 1 / Tier 2 / Tier 3 框架均需要 L1 Skill System 做格式转换。
```

### 9.5 "Harness" 术语行业印证

> **三方同时独立使用 "Harness" 描述同一个概念——这是行业正在收敛的术语：**

| 来源 | 定义 |
|------|------|
| **Microsoft Agent Framework 博客** | *Harness is the layer where model reasoning connects to real execution: shell and filesystem access, approval flows, and context management across long-running sessions.* |
| **OpenDev 论文 (arXiv 2603.05344)** | Compound AI System — dual-agent architecture separating **planning from execution** + **lazy tool discovery** |
| **OpenHarness（Reddit，开源 SDK）** | *A composable SDK to build agent harnesses* |
| **EAASP L1 Runtime** | Agent Loop + Tool System（Skill System + MCP Client）+ Hook Executor |

**结论：EAASP L1 Runtime = "Harness" Tier 1**，术语与行业完全对齐。

### 9.6 最终 Tier 分布（调研结论）

```
Tier 1（零成本对接 EAASP）：
  ├── grid-runtime          (Rust, 原生 EAASP 契约)
  ├── claude-code-runtime   (Py/TS, claude-agent-sdk = 完整 harness)
  ├── NanoBot              (Python, ~4k 行, SKILL.md native, SkillsLoader)
  └── OpenClaw             (TS, 最完整 SKILL.md + enterprise + ClawHub)

Tier 2（HookBridge 补全 hooks）：
  ├── LangGraph             (Python, checkpointer 成熟, enterprise 采纳高)
  ├── OpenDev               (Rust, compound AI, adapter 工作量大)
  └── Vercel AI SDK         (TS, 轻量 skill system)

Tier 3（大量实现工作）：
  ├── AutoGen               (Python, conversation-based ≠ EAASP session)
  ├── PydanticAI            (Python, type-safe, hook 概念弱)
  └── Semantic Kernel       (C#/.NET, 技术栈差异大)

Tier 3 Future（暂不定义）：
  └── Multi-Runtime Team — EAASP L4 多 L1 Runtime 协作场景，
      保留 HookBridge 分布式扩展接口即可。
```

### 9.7 Tier 2 试点推荐（调研结论）

| 优先级 | 框架 | 理由 |
|--------|------|------|
| 🥇 第一 | **LangGraph** | Python 生态，checkpointer 成熟（SQLite/Postgres），tool executor 完善，enterprise 采纳度高 |
| 🥈 第二 | **PydanticAI** | Type-safe，轻量，比 LangGraph 更接近 harness 模型 |
| 🥉 第三 | **AutoGen** | 企业用户多，但 conversation-based 模型 adapter 工作量最大 |

### 9.8 Tier 3 Future 扩展说明（调研结论）

Tier 3（Multi-Runtime Team）在当前 EAASP 设计中**意义不大**：
- EAASP L1 Runtime = 单 session 单 agent → Tier 3 的"跨 agent hook"不适用
- L4 多 agent 协作是**平台层**事情，不影响 L1 Runtime tier 划分
- HookBridge 已支持任意多 agent 场景（内部实现决定 hook 粒度）

---

## 10. Deferred Items

| ID | 内容 | 前置条件 | 状态 |
|----|------|---------|------|
| BE-D0 | **L2 Registry 存储技术与 MCP Server 部署模型确认** | 用户确认 OQ1~OQ5 | ⏳ |
| BE-D1 | grid-runtime proto 迁移（crates → repo 顶层） | W1 proto 重构 | ⏳ |
| BE-D2 | L2 Registry MCP Server 注册协议（registry.proto） | BE-D0 确认后 | ⏳ |
| BE-D3 | HookBridge ↔ L3 双向流策略下发 | L3 真实部署 | ⏳ |
| BE-D4 | Pause/Resume 跨 runtime 验证 | eaasp-certifier mock L4 | ⏳ |
| BE-D5 | buf breaking CI 配置 | proto 重构完成 | ⏳ |
| BE-D6 | **Tier 2 第一试点 = LangGraph**（PydanticAI 第二） | HookBridge 核心完成 | ⏳ |
| BE-D7 | L1 ↔ L2 通讯协议确认（MCP stdio vs MCP over gRPC） | BE-D0 确认后 | ⏳ |
| BE-D8 | Tier 3 Future — Multi-Runtime Team 扩展接口设计 | EAASP L4 多 runtime 协作需求 | 🔮 未来 |
| BD-D1 | grid-hook-bridge crate | HookBridge 核心完成 | ⏳ |
| BD-D2 | RuntimeSelector + AdapterRegistry | EAASP 平台层 | ⏳ |

---

## 10. Enterprise SDK 抽象层设计（2026-04-05）

> **设计目标**：业务智能体开发基于 EAASP 抽象概念展开，EAASP 配置细节及内部结构不需要知道。支持多语言 SDK（Python/TS/Java/Go/C#）。

### 10.1 核心设计原则

| 原则 | 说明 |
|------|------|
| **概念完整性** | 各语言 SDK 的抽象层完全一致，只是语法不同 |
| **实现解耦** | SDK 内部通过 EAASP 平台通信，业务开发者不感知 gRPC |
| **配置驱动** | Skill、Policy、Playbook 以标准格式（YAML/JSON）为主，代码为辅 |
| **渐进暴露** | 只暴露必要的概念——Skill 和 Tool 是核心，Policy 和 Playbook 是进阶 |
| **平台无关** | 换语言、换 Runtime、换部署方式，代码几乎不变 |

### 10.2 三层分离架构

```
┌─────────────────────────────────────────────────────────────────┐
│  概念层（Business Abstraction Layer）                            │
│  业务智能体开发者只需要理解 7 个抽象概念：                         │
│  Agent / Skill / Tool / Policy / Playbook / Session / Message  │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│  实现层（Platform SDK Layer）                                    │
│  各语言 SDK 实现：eaasp-sdk-python / typescript / java / go / csharp  │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│  基础设施层（Infrastructure）                                     │
│  EAASP 平台内部：gRPC L1 契约 + L1 Runtimes + L2/L3/L4          │
└─────────────────────────────────────────────────────────────────┘
```

### 10.3 7 个核心抽象概念

#### Agent — 智能体

```python
# Python SDK
agent = Agent(
    name="order-processor",
    description="负责订单全流程处理",
    skills=["order-management"],
    tools=["erp-lookup", "crm-update"],
    policies=["high-value-approval"],
)

response = agent.send("查询订单 ORD-2026-001 的状态")
```

```typescript
// TypeScript SDK — 相同抽象，不同语法
const agent = new Agent({
  name: "order-processor",
  description: "负责订单全流程处理",
  skills: ["order-management"],
  tools: ["erp-lookup", "crm-update"],
  policies: ["high-value-approval"],
});

const response = await agent.send("查询订单 ORD-2026-001 的状态");
```

#### Skill — 业务技能包

```python
skill = Skill(
    name="order-management",
    description="订单管理业务知识与操作规范",
    content=read("skills/order-management/SKILL.md"),
    metadata={
        "domain": "erp",
        "version": "1.0",
        "tags": ["order", "erp", "fulfillment"],
    },
)

skill.inject(session, context={"current_order_id": "ORD-001"})
```

> **SKILL.md 是 Skill 的标准序列化格式**，各语言 SDK 负责解析和转换。

#### Tool — 业务工具

```python
@tool(name="erp-lookup-order")
def erp_lookup_order(order_id: str) -> dict:
    """查询 ERP 系统中的订单状态"""
    return ERP_CLIENT.query(endpoint="/api/orders", params={"order_id": order_id})

tool = Tool(
    name="erp-lookup-order",
    description="查询 ERP 订单状态",
    input_schema={"order_id": {"type": "string"}},
    risk_level=RiskLevel.MEDIUM,
    requires_approval=True,
)

agent.register_tools([erp_lookup_order])
```

#### Policy — 治理策略

```python
from eaasp_sdk.policies import Approval, Audit, Quota, ContentFilter

policy = Policy(
    name="high-value-approval",
    rules=[
        Approval(tool="crm-update", condition="amount > 50000"),
        Audit(tool="erp-*", log_inputs=True, log_outputs=True, redact_pii=True),
        Quota(tool="external-api-*", max_calls_per_hour=100),
        ContentFilter(scope="all", blocked_patterns=["DELETE *", "DROP TABLE"]),
    ],
)

agent.attach_policy(policy)
```

#### Playbook — 编排脚本（YAML 标准格式）

```yaml
# order-processing.yaml
name: order-fullflow
description: 订单处理全流程

variables:
  order_id: string
  order_value: number

agents:
  - name: intake
    role: 接收并验证订单
    skills: [order-validation]
  - name: fulfillment
    role: 执行订单履约
    skills: [inventory-check, logistics-book]
  - name: reviewer
    role: 人工复核
    type: human-in-the-loop
    trigger: "order_value > 50000"

flow:
  - from: start    to: intake
  - from: intake   to: fulfillment  condition: intake.result.approved
  - from: fulfillment to: reviewer   condition: "order_value > 50000"
  - from: fulfillment to: complete    condition: "order_value <= 50000"
```

```python
playbook = Playbook.load("order-processing.yaml")
result = playbook.execute(variables={"order_id": "ORD-001", "order_value": 75000})
```

#### Session — 会话上下文

```python
session = agent.create_session(
    context={"user_id": "user-123", "org_unit": "华东区", "user_role": "sales-manager"},
)

response = session.send("帮我查询本周新订单")
state = session.get_state()  # 可序列化，用于中断恢复
session.restore(state)       # 中断后恢复
```

#### Message — 消息

```python
from eaasp_sdk.messages import UserMessage, SystemMessage, ToolResult

msg = UserMessage(content="帮我查询订单 ORD-001 的状态")
msg = SystemMessage(content="你是一个订单处理专家。")
msg = ToolResult(tool="erp-lookup-order", content="订单状态：已发货")
```

### 10.4 SDK 架构：跨语言一致性

```
eaasp-sdk/
├── specs/                    # 各语言 SDK 共用的规范定义（TOML）
│   ├── Agent.toml            # Agent 抽象规范
│   ├── Skill.toml            # Skill 抽象规范
│   ├── Tool.toml             # Tool 抽象规范
│   ├── Policy.toml           # Policy 抽象规范
│   ├── Playbook.toml         # Playbook 抽象规范
│   └── Session.toml          # Session 抽象规范
│
├── python/                   # Python SDK（参考实现）
│   ├── pyproject.toml
│   └── eaasp_sdk/
│       ├── __init__.py
│       ├── agent.py
│       ├── skill.py
│       ├── tool.py
│       ├── policy.py
│       ├── playbook.py
│       ├── session.py
│       └── messages.py
│
├── typescript/               # TypeScript SDK
├── java/                    # Java SDK
├── go/                      # Go SDK
└── csharp/                  # C# SDK
```

### 10.5 企业开发框架 vs EAASP 各层映射

| 企业开发框架层 | EAASP 层 | 企业关注点 | EAASP 平台提供 |
|---------------|---------|-----------|---------------|
| **Enterprise SDK** | 概念层 | 7 个抽象概念 | 多语言 SDK |
| **业务工具** | L1 Tool System | 业务逻辑（Tool 抽象） | 工具执行框架 |
| **领域 Skill** | L2 Skill System | SKILL.md 编写 | Skill MCP Server 存储发现 |
| **治理策略** | L3 Governance | Policy 抽象 | Hook 下发、Quota 执行 |
| **多 Agent 编排** | L4 Collaboration | Playbook 抽象 | 握手协议、协作 API |
| **业务 Ontology** | L2 Ontology | 实体模型 | Ontology MCP Server |

### 10.6 BE-WS1~W8 对 Enterprise SDK 的影响

| Wave | 原计划 | 修正后 |
|------|--------|--------|
| W1 | proto 全局化 | 不变 |
| **W4** | claude-code-runtime Python W1 | **改为** Enterprise SDK Python（SDK 抽象层 + Python 实现）|
| **W5** | claude-code-runtime Python W3 | **改为** Enterprise SDK Python W2（Tool System + Skill System + Policy + Playbook）|
| **W6** | claude-code-runtime Python W4 | **改为** Enterprise SDK Python W3（gRPC service + SDK 测试 + 多语言规范）|
| **W7** | claude-code-runtime TypeScript | **改为** Enterprise SDK TypeScript（参照 Python 规范实现）|

> **澄清**：`claude-code-runtime` 是 EAASP **Tier 1 Runtime 实现**，不是企业 SDK。Enterprise SDK 才是企业业务开发者使用的接口。两者不同。

---

## 11. EAASP L3/L4 演进路径：Mock → Production

> **设计目标**：明确 eaasp-certifier（Mock L3/L4）到真实 EAASP L3/L4 的演进阶段，每个 Stage 独立可用，接口不变实现变。

### 11.1 演进阶段概览

```
Stage 1: Mock L3/L4（当前）
  └─ 目标：验证 EAASP L1 契约
  └─ 实现：内存中硬编码 policies、无持久化
  └─ 用户：Runtime 开发者（测试自己的 Runtime）

Stage 2: Simulation L3/L4（下一阶段）
  └─ 目标：模拟真实 L3/L4 行为
  └─ 实现：配置文件驱动、SQLite 持久化、部分 telemetry
  └─ 用户：EAASP 平台开发者（验证平台设计）

Stage 3: Partial Real L3（中期）
  └─ 目标：L3 核心功能真实化
  └─ 实现：策略引擎 + Quota Enforcer + Telemetry 收集
  └─ 用户：早期企业用户（POC）

Stage 4: Full Production L3/L4（最终）
  └─ 目标：生产级 L3 + L4
  └─ 实现：多租户、真实认证、高可用部署
  └─ 用户：生产环境企业用户
```

### 11.2 Stage 1 → Stage 2：Mock → Simulation

**核心变化**：从硬编码到配置驱动

| 组件 | Stage 1（Mock）| Stage 2（Simulation）|
|------|---------------|---------------------|
| Policies | 硬编码 | YAML 配置文件 |
| Session 管理 | 内存 HashMap | SQLite 持久化 |
| Telemetry | 假数据 | 文件日志（JSON Lines）|
| Hook 策略 | 空操作或固定 Allow | 从配置文件加载并执行 |
| Quota | 无 | 计数器 + YAML 配置 |
| MCP Server 发现 | 无 | SQLite Registry |

**Simulation L3 YAML 配置**：

```yaml
# simulation-l3.yaml
governance:
  policies:
    - name: erp-approval
      tool_pattern: "erp_*"
      action: approval
      always: true
      
    - name: crm-audit
      tool_pattern: "crm_*"
      action: audit
      log_inputs: true
      log_outputs: true
      redact_pii: true
      
    - name: external-quota
      tool_pattern: "external-api-*"
      action: quota
      max_calls_per_hour: 100
      
    - name: content-filter
      scope: all
      action: block
      blocked_patterns:
        - "DELETE *"
        - "DROP TABLE"
        - "rm -rf /*"

quotas:
  session_max_duration_minutes: 30
  max_tool_calls_per_session: 1000

telemetry:
  enabled: true
  output: "logs/telemetry-{session_id}.jsonl"
  granularity: tool_call

registry:
  type: sqlite
  path: ".eaasp/simulation-registry.db"
```

### 11.3 Stage 2 → Stage 3：Simulation → Partial Real L3

**核心变化**：策略引擎从模拟到真实执行

```
EAASP L3 Governance Layer（Stage 3）
├── PolicyEngine          # ✅ 真实策略评估引擎
│   ├── condition_parser  # 表达式求值（amount > 50000）
│   ├── pattern_matcher   # glob/regex 工具名匹配
│   └── action_executor  # approval → 等待确认 / audit → 写日志
│
├── QuotaEnforcer         # ✅ 真实 Quota 执行
│   ├── rate_limiter     # token bucket 算法
│   ├── counter_store    # Redis 或 SQLite
│   └── quota_config     # per-tool / per-session / per-org
│
├── TelemetryCollector   # ✅ 真实 Telemetry
│   ├── event_receiver   # 接收 L1 上报的 TelemetryEvent
│   ├── aggregator       # 聚合统计
│   └── exporter         # Prometheus / CloudWatch
│
├── SessionManager       # ✅ 真实 Session 生命周期
│   ├── create           # 分配 session_id
│   ├── pause/resume     # 状态持久化
│   └── terminate       # 资源清理
│
└── HookBridgeServer     # ✅ 真实 HookBridge（供 Tier 2/3 Runtime）
    ├── evaluate_hook    # gRPC 服务
    └── update_policies  # L3 → HookBridge 策略下发
```

**Stage 3 不包含**：真实多租户认证、L2 MCP Server Layer 真实连接、高可用部署。

### 11.4 Stage 3 → Stage 4：Partial Real → Full Production

**核心变化**：多租户 + L4 协作平台 + 高可用

| 子阶段 | 目标 | 关键组件 |
|--------|------|---------|
| **Stage 4.1** | Production L3 | OIDC/JWT Auth、PostgreSQL、Redis Quota、K8s HA |
| **Stage 4.2** | Production L4 | MultiTenant、AgentRegistry、PlaybookExecutor、HITL |

**Stage 4 L4 组件**：

```
EAASP L4 Collaboration Platform
├── MultiTenantAuth          # OIDC / JWT / SSO
├── AgentRegistry            # 多租户 Agent 注册
├── PlaybookExecutor         # 多 Agent 协作编排
├── HumanInTheLoop           # 真实人工介入节点
├── AuditLogExporter         # 合规审计日志
└── MessageQueue             # RabbitMQ / SQS（L4 协作消息）
```

**Stage 4 基础设施**：

```
Production Infrastructure
├── Database              # PostgreSQL（多租户）
├── Cache                 # Redis（Quota、Session）
├── MessageQueue          # RabbitMQ / SQS
├── ObjectStorage         # S3/OSS（Audit logs、Telemetry blob）
├── HighAvailability      # K8s + HPA + Rolling update
└── Observability         # Prometheus + Grafana + AlertManager
```

### 11.5 完整演进地图

```
Mock L3/L4                          Production L3/L4
(eaasp-certifier)                   (EAASP Platform)
     │                                      │
     │  Stage 1                             │  Stage 4
     │  ├─ 硬编码 policies                   │  ├─ MultiTenantAuth
     │  ├─ 内存 session                     │  ├─ Real L4 Collaboration
     │  ├─ 假 telemetry                     │  ├─ PostgreSQL + Redis
     │  └─ L1 契约验证                      │  ├─ K8s HA Deployment
     │                                      │  ├─ Real L2 MCP Registry
     ▼                                      ▼
┌─────────────┐                      ┌─────────────────────────────┐
│ eaasp-      │ ──── Stage 2 ────▶  │ simulation-l3               │
│ certifier   │   ├─ YAML 配置文件    │ (配置驱动 + SQLite 持久化)   │
│ (当前)       │   ├─ SQLite 持久化    │                             │
│             │   ├─ 文件日志 telemetry│                             │
└─────────────┘                      └─────────────────────────────┘
                                              │
                           ┌────────────────┘
                           │
                           ▼ Stage 3
                    ┌─────────────────┐
                    │ partial-real-l3 │
                    │ (策略引擎+Quota  │
                    │  +真实Telemetry)│
                    └─────────────────┘
                           │
           ┌──────────────┴──────────────┐
           ▼                               ▼
    ┌──────────────────┐         ┌──────────────────┐
    │ production-l3    │         │ production-l4    │
    │ (多租户 L3)      │         │ (完整 L4 协作平台)│
    └──────────────────┘         └──────────────────┘
```

### 11.6 演进验收标准

| 阶段 | 验收标准 |
|------|---------|
| **Stage 1** ✅ | eaasp-certifier 能验证任意 EAASP L1 conformant Runtime 的 13 方法契约 |
| **Stage 2** | Simulation L3 能用 YAML 配置驱动，session 可中断/恢复，telemetry 写入文件 |
| **Stage 3** | 策略引擎能正确评估条件表达式（`amount > 50000`），Quota 真实限流，telemetry 上报到 Prometheus |
| **Stage 4.1** | 多租户 OIDC 认证通过，PostgreSQL 持久化验证，K8s 滚动部署成功 |
| **Stage 4.2** | 多 Agent Playbook 端到端执行，人工介入节点收到真实通知，审计日志满足合规要求 |

### 11.7 关键设计原则

1. **每个 Stage 独立可用**：Stage 1 能独立运行验证，Stage 2/3/4 在前一个基础上叠加
2. **接口不变，实现变**：`L3Client` trait 在所有 Stage 保持兼容，Mock → Simulation → Real 透明替换
3. **验收前置**：每个 Stage 完成才进入下个 Stage
4. **数据向上兼容**：SessionState 格式在所有 Stage 一致

---

## 12. 参考文档

**内部文档：**
- `docs/plans/2026-04-05-grid-runtime-design.md` — grid-runtime 完整设计
- `crates/grid-runtime/src/contract.rs` — 13 方法契约 Rust 定义
- `proto/eaasp/runtime/v1/runtime.proto` — gRPC 服务定义（待迁移扩展）

**框架文档（网络研究来源，2026-04-05）：**

| 框架 | 来源 | URL |
|------|------|-----|
| NanoBot | HKUDS 官方文档 | `https://mintlify.com/HKUDS/nanobot/concepts/architecture` |
| NanoBot Skills | HKUDS 官方文档 | `https://mintlify.com/HKUDS/nanobot/concepts/skills` |
| OpenClaw Skills | OpenClaw 官方文档 | `https://docs.openclaw.ai/tools/skills` |
| OpenDev Paper | arXiv 2603.05344 | `https://arxiv.org/abs/2603.05344` |
| Microsoft Agent Harness | Microsoft DevBlogs | `https://devblogs.microsoft.com/agent-framework/agent-harness-in-agent-framework` |
| OpenHarness | Reddit r/AgentsOfAI | `https://www.reddit.com/r/AgentsOfAI/comments/1s9sb8x` |
| ClawHub Skills Registry | VoltAgent GitHub | `https://github.com/VoltAgent/awesome-openclaw-skills` |
| NanoBot GitHub | HKUDS GitHub | `https://github.com/HKUDS/nanobot` |
