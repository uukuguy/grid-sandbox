# R2: Agno 2.0 源码评估报告

> **评估日期**：2026-04-12
> **源码位置**：`3th-party/eaasp-runtimes/agno/`
> **评估目标**：确认 Agno 2.0 在 EAASP T0-T3 分层中的归属，评估 L1 Runtime 适配可行性

---

## 1. 项目概况

| 维度 | 详情 |
|---|---|
| **语言/框架** | Python 3.7+, Pydantic + FastAPI（AgentOS Server） |
| **版本** | 2.5.16 |
| **前身** | Phidata |
| **架构模式** | 单体库 + 可选 AgentOS FastAPI server，基于 dataclass 的 Agent 定义 + Model 抽象层 |
| **代码规模** | ~785 个 Python 文件，~289,000 行（含 tools/models 等大量集成） |
| **LLM Provider 覆盖** | 40+（Anthropic, OpenAI, Gemini, Groq, Ollama, DeepSeek, Mistral, Cohere, HuggingFace, vLLM, LiteLLM, llama.cpp, LMStudio 等） |

**包结构概览**:

| 模块 | 职责 |
|---|---|
| `agent/` | Agent 核心（agent.py 1729 行）— 定义、初始化、运行、hooks、tools |
| `models/` | LLM Provider 抽象（40+ adapters） |
| `tools/` | 120+ 内置工具（含 MCP client） |
| `hooks/` | Hook 装饰器 |
| `skills/` | Skill 加载系统（SKILL.md + YAML frontmatter） |
| `os/` | AgentOS — FastAPI server + routers + 多接口（A2A, Slack, Telegram, WhatsApp） |
| `team/` | 多 Agent 编排（Team 模式） |
| `workflow/` | Workflow 编排 |
| `memory/` | Memory 管理 |
| `knowledge/` | Knowledge/RAG |
| `guardrails/` | Guardrails（输入/输出检查） |
| `approval/` | 工具审批系统（@approval 装饰器） |
| `eval/` | 评估框架 |

---

## 2. 三件套评估

### 2.1 MCP Client

- **实现状态**: ✅ 完整
- **Transport**: stdio + SSE + Streamable HTTP（三种全支持）
- **实现质量**: 生产级——session 管理、TTL 清理、动态 header、per-run session 隔离

**证据**:

| 文件 | 关键代码 |
|---|---|
| `libs/agno/agno/tools/mcp/mcp.py` (663+ 行) | L21-26: 导入 `from mcp import ClientSession, StdioServerParameters` + `sse_client` + `stdio_client` + `streamablehttp_client` |
| `libs/agno/agno/tools/mcp/mcp.py` | L47: `transport: Optional[Literal["stdio", "sse", "streamable-http"]]` |
| `libs/agno/agno/tools/mcp/mcp.py` | L484-506: `_connect()` 按 transport 类型分别创建连接 |
| `libs/agno/agno/tools/mcp/mcp.py` | L574-643: `build_tools()` 调用 `session.list_tools()` 进行 tool discovery |
| `libs/agno/agno/tools/mcp/multi_mcp.py` (639 行) | `MultiMCPTools` 支持同时连接多个 MCP server |

**关键能力**：
- `include_tools` / `exclude_tools` — 工具过滤
- `requires_confirmation_tools` / `external_execution_required_tools` — per-MCP-tool 的 HITL 标记
- `header_provider` — 动态 HTTP header（per-run session 隔离）
- `weakref.finalize` 防 GC 泄漏

**EAASP 适配**：可直接消费 `SessionPayload.mcp_servers`——将 server 配置映射为 `MCPTools(command=..., url=..., transport=...)` 即可，**无需 wrapper**。

### 2.2 Skills

- **实现状态**: ✅ 完整（Markdown + YAML frontmatter + `allowed-tools`）

**证据**:

| 文件 | 关键代码 |
|---|---|
| `libs/agno/agno/skills/skill.py` | L23-34: `Skill` dataclass — `name`, `description`, `instructions`, `allowed_tools: Optional[List[str]]` |
| `libs/agno/agno/skills/loaders/local.py` | L44: 扫描 `SKILL.md` 文件；L127-158: `_parse_skill_md()` 解析 YAML frontmatter |
| `libs/agno/agno/skills/loaders/local.py` | L100: `allowed_tools = frontmatter.get("allowed-tools")` |
| `libs/agno/agno/skills/validator.py` | L13-20: `ALLOWED_FIELDS = {"name", "description", "license", "allowed-tools", "metadata", "compatibility"}` |
| `libs/agno/agno/skills/agent_skills.py` | L88-146: `get_system_prompt_snippet()` — XML 格式 system prompt 注入 |

**EAASP 适配**：格式完全匹配 EAASP 要求。只需在 frontmatter 扩展 `runtime_affinity` / `access_scope` / `scoped_hooks` / `dependencies` 字段即可。

### 2.3 Hooks

- **实现状态**: ❌ 不达标（agent-run 级别，非 per-tool 级别）
- **粒度**: **agent-run 级别**（整个 `agent.run()` 入口/出口各执行一次）
- **决策语义**: 通过 Guardrail 可实现 Deny（raise），但**不支持 per-tool Modify**

**证据**:

| 文件 | 关键代码 |
|---|---|
| `libs/agno/agno/agent/agent.py` | L183-187: `pre_hooks`, `post_hooks`, `_run_hooks_in_background` 定义 |
| `libs/agno/agno/agent/_hooks.py` | L42-148: `execute_pre_hooks()` — 接收 `run_input: RunInput`，在 agent.run() 入口前执行（一次） |
| `libs/agno/agno/agent/_hooks.py` | L261-359: `execute_post_hooks()` — 接收 `run_output: RunOutput`，在 agent.run() 完成后执行（一次） |
| `libs/agno/agno/agent/_run.py` | L419-434: `pre_hooks` 在 run 入口执行；L573-585: `post_hooks` 在 run 结束执行 |

**关键差距**:

| EAASP 要求 | Agno 实现 | 差距 |
|---|---|---|
| PreToolUse per-tool 拦截 | pre_hooks 在 run 入口一次 | **缺失**：无法在每次 tool call 前拦截 |
| PostToolUse per-tool 拦截 | post_hooks 在 run 结束一次 | **缺失**：无法在每次 tool call 后拦截 |
| Allow/Deny/Modify 返回值 | Guardrail 仅支持 Deny (raise) | **缺失**：无 Modify 语义 |
| tool_name 过滤 | 无 | **缺失**：hook 无法知道当前执行的是哪个 tool |

**替代机制（不等价）**：
- `requires_confirmation` / `external_execution`（per-tool HITL 暂停）— 是用户交互，非程序化 hook
- `@approval` 装饰器（审批系统）— 也是 HITL，非程序化 Allow/Deny
- Guardrails — 只能对 run input/output 做检查，非 per-tool

---

## 3. Tier 归属判定

### 结论: **T2**

| 维度 | 状态 | T1 达标 |
|---|---|---|
| **MCP** | 原生 MCP Client，stdio + SSE + Streamable HTTP | ✅ |
| **Skills** | SKILL.md + YAML frontmatter + allowed-tools | ✅ |
| **Hooks** | agent-run 级别 pre/post hooks，**无 per-tool 拦截** | ❌ |

MCP 和 Skills 两个维度完全达标（甚至超标——Agno 的 Skills 格式是所有候选中最接近 EAASP 的）。但 Hooks 是 agent-run 级别而非 EAASP 要求的 per-tool-call 级别（PreToolUse/PostToolUse），这是 T1→T2 分水岭。

**与之前 web 调研结论的修正**：之前认为 Agno "可能上移到 T1"，源码验证后确认维持 **T2**。`pre_hook/post_hook` 确实存在且支持 background 模式，但它们是 agent-run 级别（Run 开始/结束各一次），而非 per-tool 级别。Hook callback 签名中没有 `tool_name` / `tool_call_id` / `tool_args` 等参数。

---

## 4. EAASP Adapter 厚度估计

| 适配项 | 工作量 | 说明 |
|---|---|---|
| **PerToolUse Hook 注入点** | 2-3 天 | 在 `_run.py` tool-call loop 中注入 PreToolUse/PostToolUse 拦截点 |
| **Hook 参数扩展** | 1 天 | 在 hook callback 签名中传递 `tool_name`, `tool_args`, `tool_call_id` |
| **ScopedHookHandler 桥接** | 1-2 天 | 将 EAASP `ScopedHook` 翻译为 Agno 内部 hook 调用 |
| **SessionPayload 映射** | 1 天 | 5-block SessionPayload → Agno Agent 初始化参数 |
| **gRPC RuntimeService** | 2-3 天 | 实现 16 方法 gRPC 服务 |
| **Telemetry 采集** | 1 天 | 从 Agno 内部 metrics/events 收集 L3 telemetry |

**总估计: 5-7 天**（接近 T2 上界，但 MCP/Skills 零成本）

---

## 5. 对 EAASP 的关键贡献

| 能力 | EAASP 价值 |
|---|---|
| **120+ 内置工具** | 开箱即用的企业集成（DuckDuckGo, GitHub, Slack, Gmail, Jira, Postgres, Neo4j, E2B, Docker 等） |
| **40+ LLM Provider** | 最广泛的模型覆盖，含本地部署（Ollama, vLLM, llama.cpp, LMStudio） |
| **Team 多 Agent 编排** | 原生 `Team` 类支持多 agent 协作，含 task routing |
| **Workflow 编排** | 原生 `Workflow` 类支持 DAG 工作流 |
| **AgentOS FastAPI Server** | 完整 REST/WS API + 多接口（A2A, AG-UI, Slack, Telegram, WhatsApp） |
| **Knowledge/RAG** | 内置知识库 + 向量数据库集成 |
| **Memory** | 多层 memory（working/session/long-term） |
| **Eval 框架** | 内置评估系统 |
| **Approval/HITL** | `@approval` 装饰器 + `requires_confirmation` per-tool 标记 |

### Python AI/ML 生态覆盖价值

Agno 作为 Python 原生框架，天然覆盖 PyTorch / TensorFlow / Hugging Face / Pandas / NumPy 等 AI/ML 生态，这是 Rust-native grid-runtime 无法直接覆盖的领域。对于已有 Python AI/ML 资产的团队，Agno 是最自然的 L1 Runtime 接入起点。

---

## 6. Runtime 构建方案

```
agno process (Python)
    ├── gRPC RuntimeService（新增，16 方法）
    │   ├── Initialize → SessionPayload → Agent() 构建
    │   ├── Send → agent.run() / agent.arun()
    │   ├── Shutdown → 清理
    │   └── EmitEvent → agent events → gRPC stream
    ├── PerToolUse Hook 注入（需修改 agno 内部 _run.py）
    │   ├── PreToolUse → ScopedHookHandler → Allow/Deny/Modify
    │   └── PostToolUse → hook 执行 + 结果记录
    ├── ScopedHookHandler 桥接
    │   └── EAASP hooks → agno hooks
    └── Telemetry 采集
        └── agno metrics → L3 telemetry
```

### 改造点

1. **Core 侵入式改造**（~300 行）：
   - `agent/_run.py`：在 tool-call loop 中加入 PreToolUse/PostToolUse callback 调用点
   - `agent/agent.py`：新增 `tool_pre_hooks` / `tool_post_hooks` 属性
   - Hook callback 签名：`(tool_name, tool_args, tool_call_id, agent, session, run_context) → HookDecision`

2. **Adapter 层**（非侵入式，~500 行）：
   - `agno_runtime/service.py`：gRPC RuntimeService 16 方法
   - `agno_runtime/adapter.py`：SessionPayload → Agent 映射
   - `agno_runtime/hooks.py`：EAASP ScopedHook → agno hook 桥接
   - `agno_runtime/mapper.py`：Agno types ↔ proto types 转换

### 风险

| 风险 | 级别 | 说明 |
|---|---|---|
| Core 侵入式改造 | 中 | 需要 fork 或 PR 到 agno 仓库，持续跟踪上游版本 |
| agent.py 1729 行 | 低 | 代码结构清晰，hook 注入点明确 |
| 版本跟踪 | 中 | agno 更新频繁（v2.5.16），需关注 `_run.py` 变更 |
