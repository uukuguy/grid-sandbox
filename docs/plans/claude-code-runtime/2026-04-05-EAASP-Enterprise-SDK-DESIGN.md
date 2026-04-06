# EAASP L1 Enterprise SDK — 完整设计方案

> **文档版本**: v1.0
> **创建日期**: 2026-04-05
> **Phase**: Phase BE
> **状态**: 设计阶段完成，待进入实现

---

## 一、项目概述

### 1.1 背景

EAASP（Enterprise Autonomous Agent System Platform）是一个企业级智能体运行时平台，核心目标是：

1. **多 Runtime 支持**：grid-runtime（Rust）、Enterprise SDK（多语言）等多种 L1 运行时并存，平台统一治理
2. **企业级治理**：L3 治理层提供 Hook 策略下发、Quota 控制、Telemetry 收集
3. **统一资产层**：L2 通过 MCP Server 统一管理 Skills、MCP Tools、Ontology
4. **渐进式演进**：从 Mock → Simulation → Production，分阶段实现

### 1.2 核心目标

| 目标 | 说明 |
|------|------|
| **Tier 1 Runtime 生态** | 多个 EAASP L1 conformant Runtime 并存，共享 L2/L3 |
| **Enterprise SDK** | 企业业务智能体开发者使用多语言 SDK，基于 7 个抽象概念开发 |
| **HookBridge** | Tier 2/3 Runtime 的 Hook 治理桥梁 |
| **eaasp-certifier** | 验证 Runtime 契约合规性的工具 |
| **L3/L4 演进路径** | 4 阶段从 Mock 到 Production |

---

## 二、EAASP 四层架构

```
┌─────────────────────────────────────────────────────────────────────────┐
│  EAASP 平台（L4 人机协作 → L3 治理 → L2 统一资产层 → L1 执行）             │
│                                                                         │
│  ┌──────────────────────────────────────────────────────────────┐      │
│  │  L4 人机协作层                                                 │      │
│  │  • 多 Agent 协作（Playbook 编排）                              │      │
│  │  • 人工介入节点（Human-in-the-Loop）                           │      │
│  │  • 跨 Agent 会话管理                                           │      │
│  └──────────────────────────────────────────────────────────────┘      │
│                              ↕                                         │
│  ┌──────────────────────────────────────────────────────────────┐      │
│  │  L3 治理层                                                      │      │
│  │  • Hook 策略下发（原子化，session 级生效）                      │      │
│  │  • Session 生命周期管理                                         │      │
│  │  • Quota 控制                                                  │      │
│  │  • Telemetry 收集聚合                                          │      │
│  │  • 13 方法契约（gRPC + managed_hooks_json）                    │      │
│  └──────────────────────────────────────────────────────────────┘      │
│                              ↕                                         │
│  ┌──────────────────────────────────────────────────────────────┐      │
│  │  L2 统一资产层（部署为独立 MCP Server）                         │      │
│  │  • Skills → SKILL.md → MCP Server                             │      │
│  │  • MCP Servers → 原生 MCP Server                              │      │
│  │  • Ontology → 本体对象服务                                      │      │
│  └──────────────────────────────────────────────────────────────┘      │
│                              ↕                                         │
│  ┌──────────────────────────────────────────────────────────────┐      │
│  │  L1 运行时层（各 Runtime 内部有独立 Skill System 和 MCP Client）│      │
│  │  • grid-runtime（Rust, Tier 1）                              │      │
│  │  • Enterprise SDK（多语言, Tier 1）                            │      │
│  │  • LangGraph adapter（Python, Tier 2）                        │      │
│  │  • NanoBot adapter（Python, Tier 1）                          │      │
│  │  • OpenClaw adapter（TS, Tier 1）                              │      │
│  └──────────────────────────────────────────────────────────────┘      │
└─────────────────────────────────────────────────────────────────────────┘
```

### 2.1 架构关键澄清

| 澄清点 | 说明 |
|--------|------|
| **L2 = 统一 MCP Server Layer** | Skills/MCP/Ontology 全部暴露为 MCP Server，不存在独立的"Skills Engine"或"MCP Gateway" |
| **L1 内部有独立子系统** | Skill System（SKILL.md → 转换为自己可执行格式）和 MCP Client（MCP 协议）两个独立子系统 |
| **Skills 格式** | 标准 Agent Skills 格式（SKILL.md），各 L1 运行时有责任将其转换为自己的可执行形式 |
| **EAASP L1 运行时池** | 多个运行时并存，平台按策略选择 |

---

## 三、Tier 框架

### 3.1 Tier 划分标准

| Tier | 定义 | EAASP 13 契约 | Hook 机制 | 接入成本 |
|------|------|:--------------:|---------|---------|
| **Tier 1** | 完整 Harness 实现，完美映射 EAASP 13 契约 | 直接实现 ✅ | 本地 native hooks | **零成本** |
| **Tier 2** | 基本完善的 Harness，可通过 HookBridge 补全缺失的 hooks | 需 adapter 层 ⚠️ | HookBridge 补全 | **低** |
| **Tier 3** | 传统 AI 框架，非原生 Harness | 需大量实现工作 ❌ | HookBridge 强制 | **高** |

### 3.2 Tier 1 框架（8 框架调研结论）

```
Tier 1（零成本对接 EAASP）：
  ├── grid-runtime          (Rust, 原生 EAASP 契约)
  ├── Enterprise SDK        (Py/TS, 完整 harness + 多语言 SDK)
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
  └── Multi-Runtime Team — EAASP L4 多 L1 Runtime 协作场景
```

### 3.3 "Harness" 术语行业印证

| 来源 | 定义 |
|------|------|
| **Microsoft Agent Framework 博客** | *Harness is the layer where model reasoning connects to real execution* |
| **OpenDev 论文 (arXiv 2603.05344)** | Compound AI System — dual-agent architecture |
| **OpenHarness（Reddit，开源 SDK）** | *A composable SDK to build agent harnesses* |
| **EAASP L1 Runtime** | Agent Loop + Tool System（Skill System + MCP Client）+ Hook Executor |

---

## 四、全局协议设计

### 4.1 Proto 目录结构

```
proto/                              # 全局共享，与 crate 解耦
├── eaasp/
│   ├── runtime/v1/
│   │   ├── runtime.proto           # 13 方法契约（L1 ↔ L3）
│   │   └── buf.yaml               # protobuf 管理
│   ├── hook/v1/
│   │   └── hook.proto              # HookBridge ↔ L1（L3 下发 hook 策略）
│   └── registry/v1/
│       └── registry.proto          # L2 Registry MCP Server 注册协议
├── README.md                       # proto 版本策略说明
└── buf.lock
```

**proto 版本策略**：
- `runtime.proto` 变更 = 重大版本，需所有 runtime 同步升级
- 使用 `buf breaking` CI 检查兼容性
- 每个子目录独立版本递增

### 4.2 13 方法契约

```protobuf
service RuntimeService {
  rpc Initialize(InitializeRequest) returns (InitializeResponse);
  rpc Send(SendRequest) returns (stream ResponseChunk);
  rpc LoadSkill(LoadSkillRequest) returns (LoadSkillResponse);
  rpc OnToolCall(ToolCallEvent) returns (HookDecision);        // Tier 1: 空操作
  rpc OnToolResult(ToolResultEvent) returns (HookDecision);     // Tier 1: 空操作
  rpc OnStop(StopRequest) returns (StopDecision);
  rpc GetState(GetStateRequest) returns (SessionState);
  rpc RestoreState(SessionState) returns (InitializeResponse);
  rpc ConnectMcp(ConnectMcpRequest) returns (ConnectMcpResponse);
  rpc DisconnectMcp(DisconnectMcpRequest) returns (DisconnectMcpResponse);
  rpc EmitTelemetry(EmitTelemetryRequest) returns (TelemetryBatch);
  rpc GetCapabilities(Empty) returns (CapabilityManifest);
  rpc Terminate(TerminateRequest) returns (TerminateResponse);
  rpc Health(Empty) returns (HealthStatus);
  rpc PauseSession(PauseRequest) returns (PauseResponse);
  rpc ResumeSession(ResumeRequest) returns (ResumeResponse);
}
```

### 4.3 HookBridge 协议

```protobuf
service HookBridgeService {
  // L1 → HookBridge: 请求 hook 决策（Tier 2/3 用）
  rpc EvaluateHook(HookRequest) returns (HookResponse);

  // L1 → HookBridge: 上报 hook 执行审计（Tier 1 端到端测试用）
  rpc ReportHookDecision(HookAuditEvent) returns (Empty);

  // L3 → HookBridge: 下发/更新 hook 策略
  rpc UpdatePolicies(stream HookPolicy) returns (stream PolicyAck);
}
```

---

## 五、Enterprise SDK 抽象层设计

### 5.1 核心设计原则

| 原则 | 说明 |
|------|------|
| **概念完整性** | 各语言 SDK 的抽象层完全一致，只是语法不同 |
| **实现解耦** | SDK 内部通过 EAASP 平台通信，业务开发者不感知 gRPC |
| **配置驱动** | Skill、Policy、Playbook 以标准格式（YAML/JSON）为主，代码为辅 |
| **渐进暴露** | 只暴露必要的概念——Skill 和 Tool 是核心，Policy 和 Playbook 是进阶 |
| **平台无关** | 换语言、换 Runtime、换部署方式，代码几乎不变 |

### 5.2 三层分离架构

```
┌──────────────────────────────────────────────────────┐
│  概念层（Business Abstraction Layer）              │
│  业务智能体开发者只需要理解 7 个抽象概念：           │
│  Agent / Skill / Tool / Policy / Playbook /        │
│  Session / Message                                 │
└──────────────────────────────────────────────────────┘
                          ↓
┌──────────────────────────────────────────────────────┐
│  实现层（Platform SDK Layer）                       │
│  各语言 SDK 实现：                                  │
│  eaasp-sdk-python / typescript / java / go / csharp │
└──────────────────────────────────────────────────────┘
                          ↓
┌──────────────────────────────────────────────────────┐
│  基础设施层（Infrastructure）                       │
│  EAASP 平台内部：gRPC L1 契约 + L1 Runtimes +      │
│  L2/L3/L4                                          │
└──────────────────────────────────────────────────────┘
```

### 5.3 7 个核心抽象概念

#### Agent — 智能体

```python
agent = Agent(
    name="order-processor",
    description="负责订单全流程处理",
    skills=["order-management"],
    tools=["erp-lookup", "crm-update"],
    policies=["high-value-approval"],
)

response = agent.send("查询订单 ORD-2026-001 的状态")
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
```

#### Playbook — 编排脚本

```yaml
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

### 5.4 SDK 架构

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

---

## 六、L3/L4 演进路径

### 6.1 演进阶段概览

```
Stage 1: Mock L3/L4（当前）
  └─ 目标：验证 EAASP L1 契约
  └─ 实现：内存中硬编码 policies、无持久化
  └─ 用户：Runtime 开发者

Stage 2: Simulation L3/L4
  └─ 目标：模拟真实 L3/L4 行为
  └─ 实现：配置文件驱动、SQLite 持久化、部分 telemetry
  └─ 用户：EAASP 平台开发者

Stage 3: Partial Real L3
  └─ 目标：L3 核心功能真实化
  └─ 实现：策略引擎 + Quota Enforcer + Telemetry 收集
  └─ 用户：早期企业用户（POC）

Stage 4: Full Production L3/L4
  └─ 目标：生产级 L3 + L4
  └─ 实现：多租户、真实认证、高可用部署
  └─ 用户：生产环境企业用户
```

### 6.2 Stage 1 → Stage 2：Mock → Simulation

| 组件 | Stage 1（Mock）| Stage 2（Simulation）|
|------|---------------|---------------------|
| Policies | 硬编码 | YAML 配置文件 |
| Session 管理 | 内存 HashMap | SQLite 持久化 |
| Telemetry | 假数据 | 文件日志（JSON Lines）|
| Hook 策略 | 空操作或固定 Allow | 从配置文件加载并执行 |
| Quota | 无 | 计数器 + YAML 配置 |
| MCP Server 发现 | 无 | SQLite Registry |

### 6.3 Stage 2 → Stage 3：Simulation → Partial Real L3

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
│   └── terminate        # 资源清理
│
└── HookBridgeServer     # ✅ 真实 HookBridge
    ├── evaluate_hook    # gRPC 服务
    └── update_policies  # L3 → HookBridge 策略下发
```

### 6.4 Stage 3 → Stage 4：Partial Real → Full Production

| 子阶段 | 目标 | 关键组件 |
|--------|------|---------|
| **Stage 4.1** | Production L3 | OIDC/JWT Auth、PostgreSQL、Redis Quota、K8s HA |
| **Stage 4.2** | Production L4 | MultiTenant、AgentRegistry、PlaybookExecutor、HITL |

### 6.5 验收标准

| 阶段 | 验收标准 |
|------|---------|
| **Stage 1** ✅ | eaasp-certifier 能验证任意 EAASP L1 conformant Runtime 的 13 方法契约 |
| **Stage 2** | Simulation L3 能用 YAML 配置驱动，session 可中断/恢复，telemetry 写入文件 |
| **Stage 3** | 策略引擎能正确评估条件表达式，Quota 真实限流，telemetry 上报到 Prometheus |
| **Stage 4.1** | 多租户 OIDC 认证通过，PostgreSQL 持久化验证，K8s 滚动部署成功 |
| **Stage 4.2** | 多 Agent Playbook 端到端执行，人工介入节点收到真实通知，审计日志满足合规要求 |

---

## 七、实施计划

### 7.1 Wave 分解

| Wave | 内容 | 产出 | 依赖 |
|------|------|------|------|
| **W1** | **proto 全局化 + 扩展** | `proto/eaasp/` 迁移 + `runtime.proto` 新增字段 + `hook.proto` 新建 | 无 |
| **W2** | **HookBridge Rust 核心** | `hook.proto` 实现，`hook-bridge` crate，`EvaluateHook`/`ReportHookDecision` | W1 |
| **W3** | **eaasp-certifier 核心** | Mock L3/L4，`L3Client` trait，13 方法验证 | W1 |
| **W4** | **Enterprise SDK Python W1** | 项目骨架 + 抽象层 + Agent/Skill/Tool 核心 + proto 生成 | W1 |
| **W5** | **Enterprise SDK Python W2** | Policy + Playbook + Session + Message + YAML 配置 | W4 |
| **W6** | **Enterprise SDK Python W3** | SDK 测试 + 多语言规范(specs/) + gRPC service + Dockerfile | W5 |
| **W7** | **Enterprise SDK TypeScript** | 参照 Python 规范实现 | W5, W6 |
| **W8** | **集成验证** | eaasp-certifier 验证所有 runtime 契约合规性 | W2, W3, W6, W7 |

### 7.2 并行策略

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

### 7.3 目录结构

```
lang/                                   # 新顶层目录
├── eaasp-sdk-python/                   # Python 版本（企业 SDK）
│   ├── pyproject.toml
│   ├── src/
│   │   ├── __init__.py
│   │   ├── agent.py                  # Agent 抽象
│   │   ├── skill.py                 # Skill 抽象
│   │   ├── tool.py                  # Tool 抽象
│   │   ├── policy.py                # Policy 抽象
│   │   ├── playbook.py              # Playbook 抽象
│   │   ├── session.py              # Session 抽象
│   │   ├── messages.py             # Message 抽象
│   │   ├── policies/               # 策略规则
│   │   │   ├── __init__.py
│   │   │   ├── approval.py
│   │   │   ├── audit.py
│   │   │   ├── quota.py
│   │   │   └── content_filter.py
│   │   └── platform_client.py       # 平台通信层（隐藏 gRPC）
│   └── tests/
│
├── eaasp-sdk-typescript/               # TypeScript 版本
│   └── ...
│
├── eaasp-certifier/                    # Mock L3/L4 + 契约验证
│   ├── Cargo.toml
│   ├── build.rs
│   └── src/
│       ├── main.rs
│       ├── verifier.rs               # 13 方法逐一验证
│       ├── mock_l3.rs                # Mock L3: hook 下发 + telemetry 收集
│       ├── mock_l4.rs               # Mock L4: 三方握手 + 会话交互
│       ├── l3_client.rs             # L3Client trait（L3 接口抽象）
│       ├── simulation_l3.rs         # Stage 2 Simulation L3
│       └── report.rs
│
├── hook-bridge/                       # HookBridge Rust 核心
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── hook_executor.rs          # 策略评估引擎
│       ├── policy_store.rs          # 策略存储
│       └── audit_logger.rs         # 审计日志
│
proto/                                  # 全局共享（从 crates/ 迁移）
├── eaasp/
│   ├── runtime/v1/
│   │   ├── runtime.proto
│   │   ├── buf.yaml
│   │   └── runtime_grpc.rs
│   ├── hook/v1/
│   │   ├── hook.proto
│   │   └── hook_grpc.rs
│   └── registry/v1/
│       ├── registry.proto
│       └── buf.yaml
└── README.md
```

---

## 八、关键技术决策

| # | 决策 | 理由 |
|---|------|------|
| KD1 | proto 升到 repo 顶层 | 全局共享，与 crate 解耦 |
| KD2 | Tier 1 本地 hook 执行，Tier 2/3 走 HookBridge | Tier 1 零网络开销，HookBridge 是 Tier 2/3 必需品 |
| KD3 | HookBridge Rust 独立实现 | 跨语言复用，性能最优 |
| KD4 | Enterprise SDK 多语言 | Python 先，TS 后；各语言抽象层一致 |
| KD5 | eaasp-certifier 使用 `L3Client` trait | Mock 和 Real 实现可替换 |
| KD6 | Enterprise SDK 不暴露 gRPC | 企业开发者只需理解 7 个抽象概念 |
| KD7 | L3/L4 演进 4 阶段 | 每个 Stage 独立可用，接口不变实现变 |
| KD8 | SKILL.md 是 Skill 标准序列化格式 | NanoBot/OpenClaw/Vercel AI SDK 三方印证 |
| KD9 | Tier 1 = 完整 Harness 实现，零成本 EAASP 13 契约对接 | NanoBot/OpenClaw/grid-runtime/Enterprise SDK 原生支持 |
| KD10 | Tier 2 第一试点 = LangGraph，PydanticAI 第二 | Python 生态，checkpointer 成熟 |

---

## 九、已知限制

| 限制 | 影响 | 应对 |
|------|------|------|
| 跨 runtime 会话迁移 | `get_state` 格式不统一 | 接受限制，state_data 为 opaque bytes |
| SDK subprocess 管理 | SDK CLI crash 需要重启 | SDK 层处理，gRPC session 感知 |
| L3 不存在 | 无法端到端测试 | Mock L3 + eaasp-certifier 覆盖 |
| Python gRPC subprocess | 在容器内需处理 stdio | Docker 隔离环境配置 |

---

## 十、Deferred Items

| ID | 内容 | 前置条件 | 状态 |
|----|------|---------|------|
| BE-D0 | **L2 Registry 存储技术与 MCP Server 部署模型确认** | 用户确认 OQ1~OQ5 | ⏳ |
| BE-D1 | proto 迁移（grid-runtime proto → repo 顶层）| W1 proto 重构 | ⏳ |
| BE-D2 | L2 Registry MCP Server 注册协议（registry.proto）| BE-D0 确认后 | ⏳ |
| BE-D3 | HookBridge ↔ L3 双向流策略下发 | L3 真实部署 | ⏳ |
| BE-D4 | Pause/Resume 跨 runtime 验证 | eaasp-certifier mock L4 | ⏳ |
| BE-D5 | buf breaking CI 配置 | proto 重构完成 | ⏳ |
| BE-D6 | **Tier 2 第一试点 = LangGraph**（PydanticAI 第二）| HookBridge 核心完成 | ⏳ |
| BE-D7 | L1 ↔ L2 通讯协议（MCP stdio vs MCP over gRPC）| BE-D0 确认后 | ⏳ |
| BE-D8 | Tier 3 Future — Multi-Runtime Team 扩展接口 | EAASP L4 多 runtime 协作需求 | 🔮 未来 |
| BD-D1 | grid-hook-bridge crate | HookBridge 核心完成 | ⏳ |
| BD-D2 | RuntimeSelector + AdapterRegistry | EAASP 平台层 | ⏳ |

---

## 十一、Open Questions（待用户确认）

### L2 Registry

| # | 问题 | 选项 |
|---|------|------|
| OQ1 | L2 Registry 存储技术 | Git/YAML vs PostgreSQL vs Nacos/Zookeeper |
| OQ2 | MCP Server 认证方式 | 网络隔离（无认证）vs mcp-oidc（显式 AuthN）|
| OQ3 | Ontology MCP Server 与普通 MCP Tool Server 的区别 | 语义层 vs 功能层 |
| OQ4 | SKILL.md 在 L2 的存储格式 | 仅 prose vs prose + 结构化工具定义 |
| OQ5 | L2 MCP Server 部署位置 | 与 L3 捆绑 vs 独立部署区域 |

### L1 运行时内部

| # | 问题 | 选项 |
|---|------|------|
| OQ7 | SKILL.md → L1 格式的转换时机 | Initialize 预转换 vs Send 时按需转换 |
| OQ8 | L2 MCP Server 的发现机制 | L3 下发 list vs L1 主动注册 |
| OQ9 | Skill System 与 MCP Client 的调用关系 | Skill System 调用 MCP Client vs 独立 |
| OQ10 | Pause/Resume 对 L2 MCP Server 连接的影响 | 保持连接 vs 重新连接 |

### L3 治理层

| # | 问题 | 选项 |
|---|------|------|
| OQ11 | L3 是否负责 L2 MCP Server 的路由 | L3 下发 list vs L1 直接访问 |
| OQ12 | L3 MCP Gateway 具体功能 | 协议转换 vs 认证代理 vs 流量控制 |
| OQ13 | Telemetry 上报的详细程度 | 工具调用粒度 vs Session 聚合粒度 |
| OQ14 | SKILL.md 是否足够作为各框架通用 Skill 输入格式 | 需结构化 vs prose 足够 |
| OQ15 | 各 Tier 1 框架 `get_state` 是否需要统一 schema | 接受不兼容 vs 定义 BaseState 规范 |

---

## 十二、参考文档

### 内部文档

| 文档 | 主题 |
|------|------|
| `docs/plans/2026-04-05-grid-runtime-design.md` | grid-runtime 完整设计 |
| `crates/grid-runtime/src/contract.rs` | 13 方法契约 Rust 定义 |

### 框架文档（网络研究来源，2026-04-05）

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

---

## 附录：Enterprise SDK vs Runtime 区分

| 概念 | 定义 | 开发者 |
|------|------|--------|
| **EAASP L1 Runtime** | 执行 Agent 的运行时引擎（grid-runtime / LangGraph adapter 等）| Runtime 提供商 |
| **Enterprise SDK** | 企业业务智能体开发者使用的 SDK（多语言，7 个抽象概念）| 企业业务开发者 |
| **eaasp-certifier** | 验证 Runtime 契约合规性的工具 | Runtime 提供商 |
| **HookBridge** | Tier 2/3 Runtime 的 Hook 治理桥梁 | Runtime 提供商 |

**关键区分**：
- `claude-code-runtime` 是 **EAASP Tier 1 Runtime 实现**，不是企业 SDK
- Enterprise SDK 才是 **企业业务开发者使用的接口**
- 两者通过 EAASP L1 gRPC 契约连接
