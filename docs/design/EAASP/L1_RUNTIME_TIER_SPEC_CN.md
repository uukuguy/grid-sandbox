# EAASP L1 Runtime 分层判据

> **用途**：替换 v2.0 设计规范 §8.5 对应中文章节
> **最后更新**：2026-04-12（R1-R4 源码验证后）

---

## 8.5.1 分层定义

L1 Runtime Pool 按 **adapter 厚度**——即将一个 runtime 包装为合规 EAASP L1 实例所需的工程量——将候选 runtime 分为四个 tier。目标是生态覆盖而非排名：每个 tier 代表不同团队背景和接入路径。

### T0 — Harness-Tools 容器分离型

**定义**：Agent 主体（harness）和 tools 执行环境（容器/VM/远程 sandbox）物理分离，通过解耦协议（Computer Protocol / Sandbox API / RPC）通信。凭证和治理策略通过协议层注入，不内嵌在 harness 或 tools 任一侧。

**判别特征**：

| 特征 | 说明 |
|------|------|
| 进程/容器分离 | harness 和 tools 在不同进程/容器/甚至不同机器 |
| 协议层解耦 | 协议层是解耦关键，而不是共享库 |
| Tools 可替换性 | tools 容器可以被替换而不影响 harness |

**适用场景**：支持"agent 在云端，tools 在客户侧内网"这类跨信任域部署，以及 tools 容器独立缩放/隔离/替换的生产需求。

**代表项目**：

| 项目 | 语言 | 验证状态 | 备注 |
|------|------|---------|------|
| **HexAgent** | Python | R4 源码验证（2026-04-12） | Computer 协议 6 方法；LocalNative / Lima VM / E2B Cloud 可插拔。Adapter 5-8 天 |
| Anthropic Computer Use | 商业 | Web 调研 | 理念同源，闭源 |
| E2B.dev | Python/TS | 生态已知 | 云端 sandbox 作为远程 tool 容器 |

**当前状态**：T0 未交付，Phase 0 明确不做。

---

### T1 — 完整三件套 + 薄 Adapter

**定义**：Runtime 原生提供 **MCP Client + Skills (Markdown+YAML frontmatter) + Hooks (PreToolUse/PostToolUse)** 三件套，且三件套直接对齐 EAASP 规范要求。Adapter 薄——只做协议转发。

**三件套判别矩阵**：

| 维度 | T1 要求 |
|------|--------|
| **MCP** | 原生 MCP Client（stdio + SSE 最少），能消费 EAASP `SessionPayload.mcp_servers` 5-block |
| **Skills** | Markdown + YAML frontmatter 格式，`name/description/version/allowed-tools` 字段，能无损加载或映射 EAASP Skill v2 扩展字段（`runtime_affinity/access_scope/scoped_hooks/dependencies`） |
| **Hooks** | function-call 级别（per-tool）的 PreToolUse/PostToolUse 拦截点，返回语义能映射 `{Allow / Deny / Modify}` 三元决策 |

**Adapter 厚度**：1-4 天（协议包装）。

**代表项目**：

| 项目 | 语言 | 状态 | Adapter |
|------|------|------|---------|
| **claude-code-runtime** | Python | 已交付，生产可运行 | 已完成 |
| **hermes-runtime** | Python | 已交付，生产可运行 | 已完成 |
| **OpenCode** | TypeScript | R1 源码验证 T1（2026-04-12） | 3-4 天 |
| CCB | TypeScript/Bun | 源码已读，待正式评估 | 待评估 |
| claw-code | Rust | 源码已读，hook 规范对齐最严但无 skill 无 server | 可能 T2 |

**当前状态**：已交付 2 个实例（均为 Python）。TypeScript 实例（OpenCode）已验证，adapter 尚未构建。

---

### T2 — 智能体框架，部分不完整

**定义**：Runtime 是完整的智能体框架，但 MCP/Skills/Hooks 三件套中至少一项不完整或不对齐 EAASP 规范。Adapter 要补齐缺失部分 + 做映射转换。

**Adapter 厚度**：3-7 天（协议包装 + 维度补齐）。

**典型不完整形态**：

| 缺失维度 | 典型代表 | Adapter 需补齐 |
|---------|---------|---------------|
| 无 Skill manifest（只有 recipe/代码注册） | Goose | Markdown frontmatter → Recipe 映射层 |
| Hook 粒度不对（批级/run 级，非 per-tool） | Nanobot, Agno 2.0 | 拆分/聚合到单 tool hook |
| Server 层薄弱 | Nanobot | 包 FastAPI/gRPC server 层 |
| 无 MCP（纯代码 tool） | 部分 framework | MCP client 适配层 |

**代表项目**：

| 项目 | 语言 | 不完整维度 | 状态 |
|------|------|-----------|------|
| **Agno 2.0** | Python | Hooks：agent-run 级别非 per-tool | R2 源码验证 T2（2026-04-12）。Adapter 5-7 天 |
| **Goose** | Rust | Skills：用 Recipe 非 Markdown frontmatter | 待深度验证 |
| **Nanobot** | Python | Hooks 批级 + Server 层薄弱 | 待深度验证 |

**当前状态**：T2 未交付。Agno 的 tier 归属已通过源码验证确认。

---

### T3 — 传统 AI Framework

**定义**：根本没有"agent runtime"概念，是 Python/TS 库。Agent 抽象是图节点（LangGraph）/ crew（CrewAI）/ conversation（AutoGen）/ decorator function（Pydantic AI）。通常没有 MCP，hook 语义错位（图节点级 / conversation 级 / 无 hook）。

**Adapter 厚度**：1-3 周（造 per-tool 拦截 + MCP 适配 + skill loader + 会话管理）。

**判别细节**：这类框架的"agent"概念和 EAASP 的"session + tool + hook + skill"模型语义错位——强行适配会在 16 方法 gRPC 契约上产生大量 impedance mismatch。T3 候选值得做 L1 的前提是给已有该框架资产的团队一条接入路径，而不是"选最佳 L1"。

**代表项目**：

| 项目 | 语言 | 适配难度 | 备注 |
|------|------|---------|------|
| **Pydantic AI** | Python | 中 | T3 里 hook 最干净（decorator function filter）+ 原生 MCP。已有 Pydantic 生态团队首选 |
| **Semantic Kernel** | .NET/Python | 中 | Function Invocation Filter + 2025-03 原生 MCP。.NET 团队独立入口 |
| **LangGraph** | Python | 高 | LangGraph Platform GA + 2025-07 MCP。但图节点级 hook 语义错位 |
| CrewAI | Python | 高 | workflow 静态，hook 语义错位 |
| AutoGen | Python | 高 | conversation 级，hook 缺失 |
| Google ADK | Python | 不推荐 | `before_tool_callback` 存在但 live path bug |

**当前状态**：T3 未交付。本轮评估（R1/R2）未涉及 T3 候选的源码验证。

---

## 8.5.2 T1/T2 分水岭

T1/T2 分水岭**不是**"有无 hook"——2025-2026 主流 runtime 都有 hook 了——而是 **per-tool hook 粒度**和**三件套的对齐完整度**。

| 判别点 | T1 | T2 |
|-------|----|----|
| Hook 触发粒度 | 每次 tool call 触发（per-tool） | 每次 agent run 触发或批级 |
| Hook 参数含 tool 上下文 | `tool_name`, `tool_args`, `tool_call_id` 可用 | 仅 run 级别上下文 |
| 三元决策覆盖 | Allow + Deny + Modify 三者可达（可通过组合系统） | 至少一项缺失或需侵入式改造 |
| Adapter 工作量 | 1-4 天 | 3-7 天 |

**源码验证实例**：

- **OpenCode (T1)**：`tool.execute.before/after` = per-tool hook + Permission 系统 = Allow/Deny。两系统组合覆盖三元决策，adapter 仅需桥接。
- **Agno 2.0 (T2)**：`pre_hooks/post_hooks` = agent-run 级别，签名无 `tool_name`。需在 `_run.py` tool-call loop 中侵入式注入拦截点（2-3 天）。

---

## 8.5.3 分层总结

| Tier | Adapter 厚度 | 三件套要求 | 已交付 | 源码验证 |
|------|-------------|-----------|-------|---------|
| T0 | 协议层开发 | N/A（分离架构为主特征） | 无 | HexAgent (R4) |
| T1 | 1-4 天 | MCP+Skills+Hooks 三件套完整对齐 | claude-code-runtime, hermes-runtime | OpenCode (R1) |
| T2 | 3-7 天 | 至少一项不完整 | 无 | Agno 2.0 (R2) |
| T3 | 1-3 周 | 语义错位，需大量适配 | 无 | 无 |
