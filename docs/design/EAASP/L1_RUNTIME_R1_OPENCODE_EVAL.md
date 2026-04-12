# R1: OpenCode 源码评估报告

> **评估日期**：2026-04-12
> **源码位置**：`3th-party/eaasp-runtimes/opencode/`
> **评估目标**：确认 OpenCode 在 EAASP T0-T3 分层中的归属，评估 L1 Runtime 适配可行性

---

## 1. 项目概况

| 维度 | 详情 |
|---|---|
| **语言/框架** | TypeScript 5.8, Bun 1.3, Effect-TS（函数式副作用管理） |
| **版本** | 1.4.3 |
| **架构模式** | 单进程 client+server 混合架构，基于 Effect-TS Service/Layer DI，事件驱动（Bus pub/sub） |
| **代码规模** | `packages/opencode/src/` 约 60,407 行 TypeScript |
| **LLM SDK** | Vercel AI SDK (`ai` v6.0.158）— `streamText`, `generateObject`, `dynamicTool` |
| **MCP SDK** | `@modelcontextprotocol/sdk` v1.27.1 |
| **LLM Provider 覆盖** | 20+（Anthropic, OpenAI, Azure, Google, Vertex, xAI, Groq, Mistral, DeepInfra, Cerebras, Cohere, TogetherAI, Perplexity, OpenRouter, GitLab, Copilot, Venice, Bedrock, Vercel, Gateway） |

---

## 2. 三件套评估

### 2.1 MCP Client

- **实现状态**: ✅ 完整
- **Transport**: stdio + SSE + Streamable HTTP（三种全覆盖）
- **OAuth**: 完整 OAuth 2.0 流程（PKCE, dynamic client registration, browser redirect）
- **多 Server**: 支持配置多个 MCP server，并发初始化，按 server name 命名空间隔离

**证据**:

| 文件 | 关键代码 |
|---|---|
| `packages/opencode/src/mcp/index.ts` (927 行) | L1-6: 三种 transport 导入；L300-315: `connectRemote()` 尝试 StreamableHTTP 回退 SSE；L382-412: `connectLocal()` 使用 StdioClientTransport |
| `packages/opencode/src/mcp/index.ts` | L134-162: `convertMcpTool()` 将 MCP tool 转换为 AI SDK `dynamicTool`；L164-175: `defs()` 调用 `client.listTools()` |
| `packages/opencode/src/mcp/index.ts` | L468-481: `watch()` 监听 `ToolListChangedNotification` 动态刷新 |
| `packages/opencode/src/config/config.ts` | L373-438: `McpLocal`（stdio command array）+ `McpRemote`（url + headers + oauth）配置 schema |

**EAASP 适配**：可直接消费 `SessionPayload.mcp_servers`——将 `McpServerEntry` 映射为 `MCP.add(name, config)` 调用即可，**零 wrapper**。

### 2.2 Skills

- **实现状态**: ⚠️ 部分（格式匹配，字段不完整）
- **格式**: Markdown + YAML frontmatter（gray-matter 解析）✅
- **已有字段**: `name`, `description`
- **缺失字段**: `version`, `allowed-tools`, `runtime_affinity`, `scoped_hooks`, `dependencies`

**证据**:

| 文件 | 关键代码 |
|---|---|
| `packages/opencode/src/skill/index.ts` (283 行) | L23-33: `Skill.Info` schema 仅 `name`, `description`, `location`, `content` |
| `packages/opencode/src/skill/index.ts` | L66-104: `add()` 用 `ConfigMarkdown.parse()` 解析 YAML frontmatter |
| `packages/opencode/src/skill/index.ts` | L137-188: `loadSkills()` 扫描 `.claude/skills/`, `.agents/skills/`, `.opencode/skills/` |
| `packages/opencode/src/config/markdown.ts` | L71-90: `parse()` 使用 `gray-matter` 库 |

**差异说明**：格式完全匹配 EAASP 要求（Markdown + YAML frontmatter），但 frontmatter 字段极简。Permission 控制是外挂在 agent 配置上的规则匹配（`permission/evaluate.ts`），非 skill 自身声明。扩展字段只需修改 `Skill.Info` schema，工作量 0.5 天。

### 2.3 Hooks

- **实现状态**: ⚠️ 部分（per-tool before/after 有，但 Deny 语义在独立 Permission 系统）
- **粒度**: **per-tool**（每次 tool call 触发）✅
- **决策语义**: Hook 系统仅支持 Modify（`Promise<void>`, 通过 mutate output 参数修改）；**Deny 能力在独立 Permission 系统**

**证据**:

| 文件 | 关键代码 |
|---|---|
| `packages/plugin/src/index.ts` | L232-246: `tool.execute.before` 和 `tool.execute.after` hook 签名 |
| `packages/opencode/src/session/prompt.ts` | L410-414: `plugin.trigger("tool.execute.before")` 执行前调用 |
| `packages/opencode/src/session/prompt.ts` | L425-429: `plugin.trigger("tool.execute.after")` 执行后调用 |
| `packages/opencode/src/permission/index.ts` | L167-183: `ask()` → evaluate ruleset → Allow/Deny/Ask |
| `packages/opencode/src/permission/evaluate.ts` | L133-136: `evaluate()` 基于 wildcard pattern 返回 `allow`/`deny`/`ask` |

**关键发现**：EAASP 要求的 Allow/Deny/Modify 三元决策分布在两个独立系统中：
- **Hook 系统**（Plugin `tool.execute.before/after`）→ **Modify** 能力
- **Permission 系统**（`permission/evaluate.ts`）→ **Allow/Deny/Ask** 能力

EAASP adapter 需要桥接这两个系统：Permission 提供 Deny 语义，Hook 提供 Modify 语义，组合覆盖完整三元决策。

---

## 3. Tier 归属判定

### 结论: **T1**

| 维度 | 状态 | T1 达标 |
|---|---|---|
| **MCP** | 原生 MCP Client，stdio + SSE + StreamableHTTP 全覆盖 | ✅ |
| **Skills** | Markdown + YAML frontmatter 格式，字段需扩展 | ✅（格式正确，字段适配 0.5 天） |
| **Hooks** | per-tool before/after + 独立 Permission Allow/Deny/Ask | ✅（两系统组合覆盖） |

三件套核心能力均存在且架构成熟。不完整之处均为**字段/接口适配**层面而非**架构缺失**。适配工作接近"薄 adapter"（3-4 天），确认为 T1。

---

## 4. EAASP Adapter 厚度估计

| 适配项 | 工作量 | 说明 |
|---|---|---|
| SessionPayload 注入 | 0.5 天 | `SystemPrompt.provider()` 已有系统提示构建点 |
| MCP Server 消费 | 0.5 天 | `MCP.add(name, config)` API 已就绪 |
| Skill frontmatter 扩展 | 0.5 天 | 在 `Skill.Info` schema 添加 v2 字段 |
| Hook → EAASP 桥接 | 1 天 | 创建 `EaaspHookBridge`：在 `tool.execute.before` 中调用 `ScopedHookHandler` |
| Telemetry 上报 | 0.5 天 | `SessionProcessor.handleEvent` 的 `finish-step` 事件已有 token/cost |
| gRPC RuntimeService | 1 天 | 将 OpenCode 进程包装为 16 方法 gRPC 服务 |

**总估计: 3-4 天**

---

## 5. 对 EAASP 的关键贡献

| 能力 | EAASP 价值 |
|---|---|
| **TypeScript 生态覆盖** | 填补 L1 Runtime Pool 的 TS 空白（grid-runtime=Rust, claude-code/hermes=Python） |
| **20+ LLM Provider** | 远超 grid-runtime（仅 Anthropic+OpenAI）和 claude-code-runtime（仅 Claude） |
| **Effect-TS Service/Layer 架构** | 类型安全的依赖注入，天然支持可测试性和资源安全 |
| **企业级 Permission 系统** | Rule-based Allow/Deny/Ask，持久化审批记录，运行时动态规则叠加 |
| **Sub-agent 架构** | `TaskTool` + `Agent` 多 agent 并发执行，映射 EAASP agent 运行时隔离 |

---

## 6. Runtime 构建方案

```
opencode process (Bun)
    ├── gRPC RuntimeService（新增，16 方法）
    │   ├── Initialize → 注入 SessionPayload, 启动 MCP servers, 加载 skills
    │   ├── Send → 调用 SessionPrompt.prompt(), 返回 streaming events
    │   ├── Pause/Resume/Close → Session lifecycle
    │   └── EmitEvent → 转发 Bus events
    ├── EaaspHookBridge（新增）
    │   ├── tool.execute.before → ScopedHookHandler → Allow/Deny/Modify
    │   └── tool.execute.after → PostToolUse hooks
    └── TelemetryForwarder（新增）
        └── finish-step events → L3 telemetry ingest
```

### 改造点

1. **gRPC 服务层**：新增 `packages/opencode-eaasp/`，实现 `eaasp.runtime.v2.RuntimeService`（使用 `@grpc/grpc-js` 或 `nice-grpc`）
2. **SessionPayload 消费器**：解析 5-block SessionPayload → OpenCode Config/Permission/MCP/SystemPrompt
3. **Hook 桥接器**：注册 Plugin hook，在 `tool.execute.before` 插入 EAASP scoped hook evaluation
4. **Telemetry 转发器**：订阅 `Bus` 事件 → L3 gRPC telemetry

### 风险

| 风险 | 级别 | 说明 |
|---|---|---|
| Bun 运行时兼容性 | 低 | gRPC 在 Bun 下需验证，可回退 Node.js |
| Effect-TS 学习曲线 | 中 | 需要理解 Service/Layer 模式才能正确注入 |
| 上游版本跟踪 | 低 | OpenCode 更新活跃但接口稳定 |

---

## 7. 关键文件路径索引

| 模块 | 路径 |
|---|---|
| MCP Client | `packages/opencode/src/mcp/index.ts` |
| Skill 加载 | `packages/opencode/src/skill/index.ts` |
| Skill Discovery | `packages/opencode/src/skill/discovery.ts` |
| Permission 系统 | `packages/opencode/src/permission/index.ts` |
| Permission 评估 | `packages/opencode/src/permission/evaluate.ts` |
| Plugin Hook 类型 | `packages/plugin/src/index.ts` |
| Agent 定义 | `packages/opencode/src/agent/agent.ts` |
| Tool 注册 | `packages/opencode/src/tool/registry.ts` |
| Session 处理器 | `packages/opencode/src/session/processor.ts` |
| LLM 调用层 | `packages/opencode/src/session/llm.ts` |
| Session Prompt | `packages/opencode/src/session/prompt.ts` |
| Config 系统 | `packages/opencode/src/config/config.ts` |
| Frontmatter 解析 | `packages/opencode/src/config/markdown.ts` |
| Provider 抽象 | `packages/opencode/src/provider/provider.ts` |
