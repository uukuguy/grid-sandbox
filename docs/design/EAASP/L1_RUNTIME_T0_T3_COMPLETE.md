# EAASP L1 Runtime T0-T3 完整技术参考

> **文档性质**：L1 Runtime 分层的完整技术参考，含源码验证证据、adapter 设计、贡献分析
> **最后更新**：2026-04-12（R1 OpenCode + R2 Agno + R3 AGT + R4 HexAgent 源码验证后）
> **关联评估报告**：
> - `L1_RUNTIME_R1_OPENCODE_EVAL.md` — OpenCode T1 评估全文
> - `L1_RUNTIME_R2_AGNO_EVAL.md` — Agno T2 评估全文
> - `L1_RUNTIME_R3_AGT_EVAL_DETAIL.md` + `R3_AGT_EVALUATION_MEMO.md` — AGT L3 治理评估
> - `L1_RUNTIME_R4_HEXAGENT_EVAL.md` — HexAgent T0 评估全文

---

## 一、L1 Runtime Pool 的战略定位

L1 Runtime Pool 的目标是**生态开放**而非"选最佳"。L1 的真实目的是让使用不同技术栈、不同 agent 框架的团队都能找到最接近他们现有资产的起点，把自己的 agent 接入 EAASP。

因此 L1 Runtime Pool 的价值**不在于挑冠军**，而在于**覆盖面足够宽**——Rust 团队、Python 团队、TypeScript 团队、.NET 团队、已有 LangChain 资产的团队、已有 Claude Code 资产的团队，都能在 Pool 里找到自己的路径。

---

## 二、T0-T3 分层定义

### T0 — Harness-Tools 容器分离型

**定义**：Agent 主体（harness）和 tools 执行环境（容器/VM/远程 sandbox）物理分离，通过解耦协议（Computer Protocol / Sandbox API / RPC）通信。凭证和治理策略通过协议层注入，不内嵌在 harness 或 tools 任一侧。

**判别特征**：
- harness 和 tools 在不同进程/容器/甚至不同机器
- 协议层是解耦关键，而不是共享库
- tools 容器可以被替换而不影响 harness

**适用场景**：支持"agent 在云端，tools 在客户侧内网"这类跨信任域部署，以及 tools 容器独立缩放/隔离/替换的生产需求。

**EAASP 本体呼应**：
- `SANDBOX_EXECUTION_DESIGN.md` 四种沙箱执行模式
- `grid-engine/src/sandbox/{docker,wasm,external,subprocess}.rs` 已有接口基础
- Grid 的 `external` sandbox 适配器已含部分 T0 理念

#### R4 源码验证：HexAgent（确认 T0）

**项目概况**：Python，Computer 协议，`github.com/UnicomAI/hexagent`

**Computer 协议**：定义在 `libs/hexagent/hexagent/computer/base.py`，仅 6 个方法：

| 方法 | 职责 |
|------|------|
| `is_running()` | 检查 tools 容器是否运行 |
| `start()` | 启动 tools 容器 |
| `run(command)` | 在 tools 容器内执行命令 |
| `upload(local, remote)` | 上传文件到 tools 容器 |
| `download(remote, local)` | 从 tools 容器下载文件 |
| `stop()` | 停止 tools 容器 |

协议使用 Python `typing.Protocol`（纯 async 方法调用），不是 protobuf/gRPC/JSON-RPC。

**三种实现**：

| 实现 | 文件 | 行数 | 说明 |
|------|------|------|------|
| `LocalNativeComputer` | `local/native.py` | 142 | 无隔离，subprocess shell，等同 EAASP Subprocess sandbox |
| `LocalVM` | `local/vm.py` + `_lima.py` | 988 | Lima VM 内 Linux user 隔离 + Mount 管理 + session 生命周期。**harness-tools 物理分离的核心创新** |
| `RemoteE2BComputer` | `remote/e2b.py` | 494 | E2B 云端沙箱，auto-pause/resume + sandbox_id 跨进程重连 |

**与 EAASP 的关键差异**：
- HexAgent 有 `upload/download` 文件传输原语，EAASP `RuntimeAdapter` 没有
- HexAgent 有 `start/stop` 生命周期管理，EAASP `RuntimeAdapter` 没有
- HexAgent 无 PostToolUse/Stop hook（仅 PreToolUse gate），不满足 EAASP 完整 hook 需求
- HexAgent 无审计日志、无策略分级，EAASP 有 SHA-256 hash-chain + SandboxPolicy 三级

**Adapter 厚度**：5-8 天（gRPC 16-method wrapper + PostToolUse/Stop hook 扩展）

---

### T1 — 完整三件套 + 薄 Adapter

**定义**：Runtime 原生提供 **MCP Client + Skills (Markdown+YAML frontmatter) + Hooks (PreToolUse/PostToolUse)** 三件套，且三件套直接对齐 EAASP 规范要求。Adapter 薄——只做协议转发。

**三件套判别矩阵**：

| 维度 | T1 要求 |
|------|--------|
| **MCP** | 原生 MCP Client（stdio + SSE 最少），能消费 `SessionPayload.mcp_servers` 5-block |
| **Skills** | Markdown + YAML frontmatter 格式，`name/description/version/allowed-tools` 字段，能无损加载 EAASP Skill v2 扩展字段 |
| **Hooks** | function-call 级别（per-tool）的 PreToolUse/PostToolUse 拦截点，返回语义映射 `{Allow / Deny / Modify}` 三元决策 |

**Adapter 厚度**：1-4 天。

**已交付实例**：
- ✅ `claude-code-runtime`（Python，Claude Agent SDK 包装）
- ✅ `hermes-runtime`（Python，Hermes agent 包装）

#### R1 源码验证：OpenCode（确认 T1）

**项目概况**：TypeScript 5.8 + Bun 1.3 + Effect-TS，v1.4.3，~60,407 行，20+ LLM Provider（Vercel AI SDK）

**三件套评估**：

| 维度 | 状态 | 关键证据 |
|------|------|---------|
| **MCP** | ✅ 完全达标 | stdio + SSE + Streamable HTTP 全覆盖；完整 OAuth 2.0 流程（PKCE + dynamic client registration）；多 Server 并发初始化 + namespace 隔离。核心文件：`packages/opencode/src/mcp/index.ts`（927 行） |
| **Skills** | ✅ 格式达标 | Markdown + YAML frontmatter（gray-matter 解析），已有 `name/description`。扩展 `version/allowed-tools/v2 字段` 仅需修改 `Skill.Info` schema（0.5 天）。核心文件：`packages/opencode/src/skill/index.ts` |
| **Hooks** | ✅ 组合达标 | per-tool `tool.execute.before/after`（Plugin 系统，`packages/plugin/src/index.ts` L232-246）提供 Modify 语义；独立 Permission 系统（`packages/opencode/src/permission/evaluate.ts`）提供 Allow/Deny/Ask 语义。EAASP adapter 桥接两系统即可覆盖完整三元决策 |

**Adapter 厚度明细**：

| 适配项 | 工作量 | 说明 |
|--------|-------|------|
| SessionPayload 注入 | 0.5 天 | `SystemPrompt.provider()` 已有系统提示构建点 |
| MCP Server 消费 | 0.5 天 | `MCP.add(name, config)` API 已就绪 |
| Skill frontmatter 扩展 | 0.5 天 | 在 `Skill.Info` schema 添加 v2 字段 |
| Hook→EAASP 桥接 | 1 天 | 创建 `EaaspHookBridge`：在 `tool.execute.before` 中调用 `ScopedHookHandler`，根据返回值调用 `Permission.DeniedError` 或修改参数 |
| Telemetry 上报 | 0.5 天 | `SessionProcessor.handleEvent` 的 `finish-step` 事件已有 token/cost |
| gRPC RuntimeService | 1 天 | 16 方法 gRPC 服务包装 |
| **合计** | **3-4 天** | |

**EAASP 关键贡献**：
- **TypeScript 生态覆盖**：填补 L1 Pool 的 TS 空白（grid-runtime=Rust, claude-code/hermes=Python）
- **20+ LLM Provider**：远超 grid-runtime（仅 Anthropic+OpenAI）和 claude-code-runtime（仅 Claude）
- **Effect-TS 架构**：类型安全的依赖注入，天然支持可测试性和资源安全
- **企业级 Permission 系统**：Rule-based Allow/Deny/Ask + 持久化审批记录 + 运行时动态规则叠加
- **Sub-agent 架构**：`TaskTool` + `Agent` 多 agent 并发执行，映射 EAASP agent 运行时隔离

---

### T2 — 智能体框架，部分不完整

**定义**：Runtime 是完整的智能体框架，但 MCP/Skills/Hooks 三件套中至少一项不完整或不对齐 EAASP 规范。Adapter 要补齐缺失部分 + 做映射转换。

**Adapter 厚度**：3-7 天（协议包装 + 维度补齐）。

**典型不完整形态**：

| 缺失维度 | 典型代表 | Adapter 需补齐 |
|---------|---------|---------------|
| 无 Skill manifest | Goose | Markdown frontmatter → Recipe 映射层 |
| Hook 粒度不对（run 级/批级） | Nanobot, Agno 2.0 | 拆分/聚合到单 tool hook |
| Server 层薄弱 | Nanobot | 包 FastAPI/gRPC server 层 |
| 无 MCP | 部分 framework | MCP client 适配层 |

**关键认知**：T1/T2 分水岭**不是**"有无 hook"（2025-2026 主流 runtime 都有 hook 了），而是**三件套的对齐完整度**，特别是 **per-tool hook 粒度**。

#### R2 源码验证：Agno 2.0（确认 T2）

**项目概况**：Python 3.7+ + Pydantic + FastAPI（AgentOS），v2.5.16（前身 Phidata），~289,000 行，40+ LLM Provider，120+ 内置工具

**三件套评估**：

| 维度 | 状态 | 关键证据 |
|------|------|---------|
| **MCP** | ✅ 完全达标 | stdio + SSE + Streamable HTTP 三种 transport；生产级实现（session 管理、TTL 清理、动态 header、per-run session 隔离）；`MultiMCPTools` 支持多 Server。核心文件：`libs/agno/agno/tools/mcp/mcp.py`（663+ 行） |
| **Skills** | ✅ 完全达标（甚至超标） | SKILL.md + YAML frontmatter + `allowed-tools`；validator 校验 `{name, description, license, allowed-tools, metadata, compatibility}`；XML 格式 system prompt 注入。核心文件：`libs/agno/agno/skills/` |
| **Hooks** | ❌ 不达标 | `pre_hooks/post_hooks` 在 `agent.run()` 入口/出口各**一次**（`agent/_hooks.py` L42-148 / L261-359），签名无 `tool_name` 参数。仅有 agent-run 级别粒度，**无 per-tool 拦截**。整个 tool-call loop 中没有 hook 拦截点（`agent/_run.py` L419-434 / L573-585） |

**Hooks 差距明细**：

| EAASP 要求 | Agno 实现 | 差距 |
|-----------|---------|------|
| PreToolUse per-tool 拦截 | `pre_hooks` 在 run 入口一次 | **缺失** |
| PostToolUse per-tool 拦截 | `post_hooks` 在 run 结束一次 | **缺失** |
| Allow/Deny/Modify 返回值 | Guardrail 仅支持 Deny (raise) | **Modify 缺失** |
| `tool_name` 过滤 | 无 | **缺失** |

**替代机制（不等价）**：
- `requires_confirmation` / `external_execution`：用户交互级 HITL 暂停，非程序化 hook
- `@approval` 装饰器：审批系统，也是 HITL，非程序化 Allow/Deny
- Guardrails：只能对 run input/output 做检查，非 per-tool

**与之前 web 调研的修正**：之前认为"可能上移到 T1"，源码验证后确认维持 T2。`pre_hook/post_hook` 确实存在且支持 background 模式，但它们是 agent-run 级别（Run 开始/结束各一次），而非 per-tool 级别。

**Adapter 厚度明细**：

| 适配项 | 工作量 | 说明 |
|--------|-------|------|
| PerToolUse Hook 注入点 | 2-3 天 | 在 `_run.py` tool-call loop 中注入 PreToolUse/PostToolUse 拦截点（**侵入式改造**） |
| Hook 参数扩展 | 1 天 | 在 hook callback 签名中传递 `tool_name`, `tool_args`, `tool_call_id` |
| ScopedHookHandler 桥接 | 1-2 天 | EAASP `ScopedHook` → Agno 内部 hook 翻译 |
| SessionPayload 映射 | 1 天 | 5-block SessionPayload → Agent 初始化参数 |
| gRPC RuntimeService | 2-3 天 | 16 方法 gRPC 服务 |
| Telemetry 采集 | 1 天 | Agno 内部 metrics → L3 telemetry |
| **合计** | **5-7 天** | 接近 T2 上界，但 MCP/Skills 零成本 |

**EAASP 关键贡献**：
- **120+ 内置工具**：开箱即用的企业集成（DuckDuckGo, GitHub, Slack, Gmail, Jira, Postgres 等）
- **40+ LLM Provider**：最广泛的模型覆盖，含本地部署（Ollama, vLLM, llama.cpp, LMStudio）
- **Team 多 Agent 编排**：原生 `Team` 类支持多 agent 协作 + task routing
- **Workflow 编排**：原生 `Workflow` 类支持 DAG 工作流
- **AgentOS FastAPI Server**：完整 REST/WS API + 多接口（A2A, AG-UI, Slack, Telegram, WhatsApp）
- **Knowledge/RAG + Memory + Eval**：内置知识库 + 多层 memory + 评估框架
- **Python AI/ML 生态**：天然覆盖 PyTorch / HuggingFace / Pandas 等，已有 Python AI/ML 资产的团队的最自然 L1 接入起点

---

### T3 — 传统 AI Framework

**定义**：根本没有"agent runtime"概念，是 Python/TS 库。Agent 抽象是图节点（LangGraph）/ crew（CrewAI）/ conversation（AutoGen）/ decorator function（Pydantic AI）。通常没有 MCP，hook 语义错位。

**Adapter 厚度**：1-3 周（造 per-tool 拦截 + MCP 适配 + skill loader + 会话管理）。

**判别细节**：这类框架的"agent"概念和 EAASP 的"session + tool + hook + skill"模型语义错位——强行适配会在 16 方法 gRPC 契约上产生大量 impedance mismatch。T3 候选值得做 L1 的前提是给已有该框架资产的团队一条接入路径。

**代表项目**：

| 项目 | 语言 | 适配难度 | 备注 |
|------|------|---------|------|
| **Pydantic AI** | Python | 中 | T3 里 hook 最干净（decorator function filter）+ 原生 MCP。已有 Pydantic 生态团队首选 |
| **Semantic Kernel** | .NET/Python | 中 | Function Invocation Filter + 2025-03 原生 MCP。.NET 团队独立入口 |
| **LangGraph** | Python | 高 | LangGraph Platform GA + 2025-07 MCP。图节点级 hook 语义错位 |
| CrewAI | Python | 高 | workflow 静态，hook 语义错位 |
| AutoGen | Python | 高 | conversation 级，hook 缺失 |
| Google ADK | Python | 不推荐 | `before_tool_callback` 存在但 live path bug |
| LlamaIndex | Python | 不推荐 | 定位 RAG 不是 agent runtime |

**当前状态**：T3 未交付。本轮评估未涉及 T3 源码验证。

---

## 三、T1/T2 分水岭

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

## 四、生态覆盖现状

| 团队背景 | 可选起点 | 状态 |
|---------|---------|------|
| Python + Claude 生态 | claude-code-runtime (T1) | ✅ 已可用 |
| Python + Hermes 生态 | hermes-runtime (T1) | ✅ 已可用 |
| Python + 传统 AI/ML | Agno 2.0 (T2 ✅) / Pydantic AI (T3) / LangGraph (T3) | ⚠️ Agno tier 确认，adapter 未交付 |
| Rust 系统团队 | Goose (T2) / Grid 自研 | ⚠️ Goose 未交付 |
| TypeScript 团队 | OpenCode (T1 ✅) / CCB (T1?) | ⚠️ OpenCode tier 确认，adapter 未交付 |
| .NET 团队 | Semantic Kernel (T3) | ⚠️ 候选已识别，未交付 |
| 跨信任域部署 | HexAgent (T0 ✅) | ⚠️ HexAgent tier 确认，adapter 未交付 |
| 合规/治理优先 | Microsoft AGT (L3 工作线 ✅) | ⚠️ Rust crate 可嵌入，独立工作线 |
| Go 团队 | 无 | ❌ 生态空白 |
| Java 团队 | 无 | ❌ 生态空白 |

---

## 五、分层总结

| Tier | Adapter 厚度 | 三件套要求 | 已交付 | 源码验证 |
|------|-------------|-----------|-------|---------|
| **T0** | 协议层开发 | N/A（分离架构为主特征） | 无 | HexAgent (R4) |
| **T1** | 1-4 天 | MCP+Skills+Hooks 完整对齐 | claude-code-runtime, hermes-runtime | OpenCode (R1) |
| **T2** | 3-7 天 | 至少一项不完整 | 无 | Agno 2.0 (R2) |
| **T3** | 1-3 周 | 语义错位，需大量适配 | 无 | 无 |

---

## 六、治理框架工作线（独立于 L1 Pool）

Microsoft Agent Governance Toolkit（AGT）不是 L1 候选，而是 L3 HookBridge 的可替换后端。R3 源码验证确认：

- `agentmesh` + `agentmesh-mcp` 两个 Rust crate（~8190 行），MIT License
- 6 种 MCP 安全扫描（ToolPoisoning / RugPull / CrossServerAttack / DescriptionInjection / SchemaAbuse / HiddenInstruction）
- 策略模型支持原生 YAML/JSON + OPA/Rego + Cedar 三后端
- 推荐对接方案：L3 Python 层引入 AGT PolicyEvaluator（2-3 人天），Phase 2+ 渐进 Rust 嵌入

详见 `R3_AGT_EVALUATION_MEMO.md` 和 `L1_RUNTIME_R3_AGT_EVAL_DETAIL.md`。
