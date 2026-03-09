# Agent Harness / Runtime 行业研究报告 (2025-2026)

> 研究日期: 2026-03-09
> 数据来源: Tavily 搜索 (10 个主题)、DeepWiki (3 个仓库)、行业文章与论文

---

## 行业趋势总结

### 2025-2026 Agent Harness 核心趋势

1. **"Agent Harness" 概念正式确立** — Philipp Schmid (Hugging Face) 提出 "如果 2025 是 Agent 的开端, 2026 将围绕 Agent Harness 展开"。Harness 被定义为包裹模型的基础设施层, 管理 prompt 预设、tool 执行、生命周期钩子、记忆和恢复。类比: **LLM 是 CPU, Harness 是操作系统**。(来源: philschmid.de, hugo.im, OpenAI Codex 团队)

2. **Harness Engineering 成为新学科** — OpenAI 在 2026 年 3 月公开了 Codex App Server 的 "Harness Engineering" 实践: 仓库优先文档(AGENTS.md)、golden principles 编码到仓库、质量分级系统、执行计划作为一等公民。Anthropic 发表了 "Effective Harnesses for Long-Running Agents" 和 arxiv 论文 (2603.05344), 系统性描述了 scaffolding (脚手架) + harness (运行时编排) 的二分法。

3. **Rust 成为 Agent 执行层首选语言** — Goose (Block, 31.2k stars)、rig (0xPlaygrounds, v0.32)、ZeroClaw、AutoAgents、ADK-Rust、swarms-rs 等框架密集出现。行业共识: **Python 用于探索和编排, Rust 用于执行层** — 并发安全、零 GC、可预测延迟。商业 Rust 使用量 2021-2024 增长 68.75%。(来源: JetBrains State of Rust 2025, thenewstack.io)

4. **MCP 成为事实标准** — 2024.11 Anthropic 发布 MCP, 2025.03 OpenAI 采纳, 随后 Google、Microsoft、AWS 全面集成。截至 2025.11 一周年, MCP 已进入 "不仅连接模型到数据, 而是赋能全新类别 AI 应用" 的阶段。Gartner 预测到 2026 年, 75% API 网关厂商和 50% iPaaS 厂商将具备 MCP 功能。

5. **Context Engineering 取代 Prompt Engineering** — 从 "写好 prompt" 演进到 "工程化管理上下文": 渐进式压缩 (compaction)、结构化笔记、子 Agent 架构。Anthropic 的工程博客将其定义为: reduce (减少)、offload (卸载)、isolate (隔离)。上下文窗口不再追求更大, 而是追求更智能的利用。

6. **分层记忆架构成为标配** — 行业收敛到三层记忆模型: 工作记忆 (会话内)、长期记忆 (跨会话持久化)、结构化知识 (知识图谱/关系推理)。Mem0、Zep、Letta 等专业记忆层创业公司快速发展。图记忆 (Graph Memory) 成为 2026 年热点。

7. **Multi-Agent 编排从实验走向生产** — 多 Agent 编排市场预计 2026 年达 85 亿美元。关键模式: 事件流编排 (非调用栈)、handoff 协议 (A2A)、子 Agent 隔离、文件系统交接 (.goose/handoff/)。

8. **流式 Tool 执行与实时反馈** — Agent 不再等待完整响应: 流式验证 (streaming validation)、部分结果解析 (jsonrepair)、实时 tool 调用预览。Strands Agents (AWS) 和 Dyad 代表了这一趋势。

9. **安全与可观测性成为一等公民** — IDEsaster (2025.12) 暴露了 AI IDE 生态 30+ 漏洞, 100% 受测 IDE 存在 prompt 注入风险。Agent Harness 必须内置多层安全: 审批系统、危险命令检测、doom loop 检测、迭代上限、协作取消。

10. **Bet on Protocols, Not Frameworks** — TheNewStack 文章明确指出 Agent 框架战争类似 2014 年容器战争 (Docker vs rkt vs Mesos)。赢家不是单一框架, 而是协议 (MCP/A2A)。框架会不断重建 (Manus 重构 5 次), 但协议层持久。

---

### 顶级 Rust Agent 框架对比

| 框架 | Stars | 核心特点 | Agent Loop 设计 | Tool 系统 | MCP 支持 |
|------|-------|---------|----------------|----------|---------|
| **Goose** (Block) | 31.2k | 全栈 Agent (CLI+Desktop), 企业级, 60% Block 员工周活 | ReAct 循环: request -> LLM -> tool_call -> execute -> result -> LLM; 自动 compaction (80% 阈值) | Extension trait (name/description/instructions/tools/call_tool); MCP-first 设计 | 原生 MCP (自建 mcp-client/mcp-core/mcp-server crate); ACP 协议 |
| **rig** (0xPlaygrounds) | ~4.5k (crates.io 328k 下载) | 模块化 LLM 库, 20+ provider, 10+ vector store, WASM 兼容 | Agent<M,P> + PromptRequest 驱动; max_turns 控制多轮; PromptHook 生命周期回调 | Tool trait (name/definition/call); ToolSet 集合; ToolServer + ToolServerHandle 异步通信 | 通过 rmcp feature 集成; McpTool 包装 rmcp::model::Tool |
| **ADK-Rust** (zavora-ai) | 新兴 | 对标 Google ADK 的 Rust 实现, 模块化, 支持实时语音 Agent | 模仿 LangChain/ADK 模式 | 模块化组件 | 待确认 |
| **AutoAgents** | 新兴 | 多 Agent 框架, Cloud-Native + Edge-Native + Hybrid | 可替换执行器和通信后端 | 模块化架构, 可替换组件 | 待确认 |
| **swarms-rs** | 新兴 (v0.2.1) | Python swarms 的 Rust 实现 | Agent struct + workflow 抽象 | 宏驱动 (swarms_macro) | 待确认 |
| **ZeroClaw** | ~18k | 4MB 内存, <10ms 启动, 12MB 单二进制, 零 CVE, 30+ 通道 | Trait 驱动架构 | Rust 原生扩展 | 内置 |

> **关键观察**: Goose 和 rig 是目前最成熟的两个 Rust Agent 框架。Goose 偏"全栈产品"(CLI+Desktop+MCP生态), rig 偏"LLM库"(嵌入式集成, 类似 Rust 版 LangChain)。

---

### Agent Loop 最佳实践

从多个框架和行业文章中提炼的共识模式:

#### 1. 规范 ReAct 循环 (The Canonical While Loop)

```
while !done {
    response = call_llm(messages, tools)
    messages.push(response)
    if response.has_tool_calls() {
        for tc in response.tool_calls {
            result = execute_tool(tc)
            messages.push(tool_result(tc.id, result))
        }
    } else {
        done = true  // 或等待用户输入
    }
}
```

> "Agent 本质上就是一个带 tools 的 while 循环" — Braintrust 博客
> "同样的模式, 不同的框架" — Victor Dibia (AutoGen 核心作者)

#### 2. 迭代上限与终止条件

- **max_iterations / max_turns**: 所有生产框架都设置上限 (Goose 未公开硬编码, Dyad 25 步, rig 可配置 max_turns)
- **doom loop 检测**: 识别 Agent 陷入重复循环的模式 (arxiv 2603.05344 将其列为 harness 安全层)
- **协作取消 (cooperative cancellation)**: 通过 cancel token 允许外部中断 Agent 循环

#### 3. 六阶段扩展循环 (Composio 2026 指南)

现代生产环境已从 5 步演进到 6 步:
1. **Tool Discovery** (动态发现可用工具)
2. **Intent Recognition** (识别用户意图)
3. **Tool Selection** (选择合适工具)
4. **Parameter Construction** (构建参数, JSON schema)
5. **Execution & Result** (执行并返回结果)
6. **Reflection & Retry** (反思和重试)

#### 4. Context Revision (Goose 模式)

在每轮 tool 执行后, Agent 执行上下文修订 — 移除旧的或不相关的信息, 保持上下文窗口精简。这不是简单的截断, 而是智能裁剪。

#### 5. 子 Agent 隔离 (Goose Subagent 模式)

复杂任务拆分到独立 Agent 实例, 每个子 Agent 拥有自己的隔离执行上下文:
- 主 Agent 通过 `platform__create_task` / `platform__execute_tasks` 创建任务
- 子 Agent 在独立会话中执行
- 结果聚合回父 Agent
- 保持主对话上下文清洁

#### 6. 三阶段分离 (Goose 子 Agent 架构)

```
Planning (Orchestrator) -> Building (Builder) -> Validation (Validator)
```

每个阶段使用不同模型, 通过 `.goose/handoff/` 文件系统交接:
- `02-plan.json`: Orchestrator -> Builder
- `03-build.json`: Builder -> Validator
- `04-validation.json`: Validator -> Builder (失败时回环)

---

### Tool 系统最佳实践

#### 1. Trait 驱动设计

**Goose 模式** — Extension trait:
```rust
trait Extension {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn instructions(&self) -> &str;
    fn tools(&self) -> Vec<Tool>;
    fn status(&self) -> Status;
    async fn call_tool(&self, name: &str, args: Value) -> AgentResult<Value>;
}
```

**rig 模式** — Tool trait:
```rust
trait Tool {
    fn name(&self) -> &str;
    fn definition(&self) -> ToolDefinition;  // name + description + JSON schema
    async fn call(&self, args: Value) -> Result<Value>;
}
```

**共识**: Tool 必须声明 JSON Schema 参数定义, 返回结构化结果。名称 + 描述 + 参数 schema 是最小三件套。

#### 2. 注册机制

- **rig**: `ToolSet` (集合管理) + `ToolServer` (生命周期管理) + `ToolServerHandle` (异步通信句柄)
- **Goose**: `ExtensionManager` 管理所有 extension 的加载和生命周期
- **通用模式**: Registry pattern — 工具注册时提供 schema, 运行时通过 name 查找和调用

#### 3. 动态工具发现

rig 支持 "dynamic tools" — 从 VectorStoreIndex 在 prompt 时采样相关工具, 而非一次性加载所有工具。这对工具数量多的场景至关重要 (避免 token 浪费)。

Goose 通过 MCP 实现动态工具发现 — 连接 MCP server 时自动获取其工具列表。

#### 4. 工具注解 (Tool Annotations)

MCP 2025 更新引入了 tool annotations: 标记工具为 "read-only" vs "destructive action", 帮助 Agent 和 harness 做安全决策。

#### 5. 执行模型

- **同步执行**: 简单场景, 一个工具调一次
- **并行执行**: 多个独立 tool call 可并发执行 (Promise.all / tokio::join!)
- **流式执行**: 工具执行过程中流式返回部分结果
- **沙箱执行**: 不可信代码在隔离环境 (Docker/WASM/subprocess) 中执行

---

### Context Engineering 最佳实践

#### 1. Anthropic 三原则: Reduce, Offload, Isolate

- **Reduce (减少)**: 渐进式压缩 (compaction) — 当上下文达到阈值时, 摘要化旧内容
- **Offload (卸载)**: 结构化笔记 (note-taking) — Agent 将关键信息写入外部文件, 需要时再读取
- **Isolate (隔离)**: 子 Agent 架构 — 每个子任务在独立上下文中执行, 避免上下文污染

#### 2. 五阶段渐进式压缩 (arxiv 2603.05344)

论文描述了 Claude Code 的 harness 实现:
1. **Pre-check**: 检查上下文使用率
2. **Thinking**: LLM 思考阶段
3. **Self-critique**: 自我批评
4. **Action**: 选择工具
5. **Post-processing**: 压缩和清理

当上下文接近限制时, 应用渐进式压缩策略。

#### 3. 自动压缩 (Auto-Compaction)

Goose 的实现: 当对话达到上下文窗口的 80% 时自动触发摘要化, 保留关键信息同时减少 token 使用。阈值可通过 `GOOSE_AUTO_COMPACT_THRESHOLD` 配置。

#### 4. 串行位置优化 (Serial Position Effect)

LLM 对上下文开头和结尾的信息关注度高于中间部分 ("lost in the middle" 问题)。最佳实践:
- 重要信息放在上下文的开头和结尾
- 中间部分放不太关键的背景信息
- 使用 recency bias (近期信息权重更高)

#### 5. Observation Masking

JetBrains Research 提出: 选择性遮蔽过往轮次的工具输出 (observation), 保留 action 和 reasoning 历史。因为典型 SE Agent 的每轮输出中, observation (如文件内容、测试日志) 占绝大部分 token。

#### 6. Git-Context-Controller (GCC)

2025 年论文提出将 Agent 记忆形式化为类 Git 的版本化层次结构:
- COMMIT: 保存里程碑
- BRANCH: 实验性探索
- MERGE: 合并结果
- CONTEXT: 上下文切换

在软件工程和自我复制任务中达到 SOTA 结果。

#### 7. 仓库优先文档 (Repository-First Documentation)

OpenAI Codex 团队实践:
- 所有架构决策、命名规范、部署流程都在仓库中
- AGENTS.md 作为 Agent 的 "ground truth"
- 执行计划 (execution plans) 作为一等公民, 带进度和决策日志
- 质量文档对每个产品域和架构层评分

---

### Memory 架构最佳实践

#### 1. 三层记忆模型 (行业共识)

| 层级 | 名称 | 特点 | 实现方式 |
|------|------|------|---------|
| L0 | 工作记忆 (Working Memory) | 会话内, 易失, 类 RAM | 对话历史 / 消息列表 |
| L1 | 长期记忆 (Long-term Memory) | 跨会话持久化, 语义检索 | 向量数据库 (embedding + cosine similarity) |
| L2 | 结构化知识 (Structured Knowledge) | 实体关系图, 推理就绪 | 知识图谱 (entities + relationships + rules) |

> "向量数据库不等于记忆。有用的 Agent 需要: 情景记忆 (发生了什么)、语义记忆 (我知道什么)、程序记忆 (我怎么做)" — hugo.im

#### 2. Goose 的记忆实现

- **.goosehints**: 静态项目级上下文, 自动加载
- **Memory Extension**: 动态存储系统, 支持标签/关键词存取, 实现跨会话持久化
- **Auto-Compaction**: 自动摘要化长对话
- **Subagents**: 子 Agent 隔离执行, 保持主上下文清洁
- **Recipes**: 将完整任务设置打包为可复用配置

#### 3. 图记忆 (Graph Memory) — 2026 热点

- **Zep**: Temporal Knowledge Graph, 比基线检索系统准确率高 18.5%, 延迟降低 ~90%
- **Mem0**: 结构化摘要 + 冲突解决, 准确率提升 26%, token 成本大幅降低
- **Letta**: "文件系统记忆" (原始文本文件按时间戳索引) 在某些基准测试中超过专业系统
- **Graphiti**: 实时知识图谱构建 (23k stars)

#### 4. Constitutional Memory Architecture (CMA)

2026 年 arxiv 论文 (2603.04740) 提出: 随着 Agent 生命周期从分钟扩展到天/周/月, 需要 "受治理的记忆基础设施" — 包含身份持续性、记忆治理、跨模型升级的服务连续性。

#### 5. Redis 三层记忆架构

Redis 在 Agent 编排中的定位:
- **短期记忆**: 活跃会话的工作上下文 (sub-ms 访问)
- **长期记忆**: 用户画像和历史模式 (跨会话持久化)
- **情景记忆**: 通过语义检索回忆特定过去交互

统一向量搜索 + 状态协调 + 消息传递, 替代独立的向量数据库、缓存层和消息队列。

---

### MCP 集成最佳实践

#### 1. 传输层

- **Stdio**: 最基础的传输方式, 适合本地 MCP server (CLI 工具)
- **Streamable HTTP**: 2025 年更新, 支持低延迟实时交互, 替代旧的 SSE 模式
- **WebSocket**: 双向通信, 适合持久连接场景

#### 2. 认证与安全

- **OAuth 2.1**: MCP 2025 更新引入标准认证/授权
- **Tool Annotations**: read-only vs destructive 标记
- **Stytch Connected Apps**: 专门为 MCP server 提供 OAuth 认证解决方案

#### 3. Tool 桥接模式

**Goose 模式**: 自建 mcp-client/mcp-core/mcp-server 三个 crate
- `mcp-core`: MCP 协议共享定义
- `mcp-client`: MCP 客户端实现
- `mcp-server`: MCP 服务端实现
- `goose-mcp`: 具体 MCP extension 实现

**rig 模式**: 通过 `rmcp` feature 集成
- `McpTool` 包装 `rmcp::model::Tool` 定义
- 使用 `rmcp::service::ServerSink` 客户端通信
- 通过 `AgentBuilder::rmcp_tools()` 添加到 Agent

**octo-sandbox 模式** (本项目): 通过 `rmcp` 0.16 SDK
- `McpManager`: 运行时服务器生命周期
- `McpStorage`: SQLite 持久化
- `McpToolBridge`: 将 MCP 工具包装为统一 Tool trait

#### 4. 服务器管理

- 动态发现: 连接 MCP server 时自动获取工具列表
- 生命周期管理: start/stop/restart
- 日志隔离: 每个 MCP server 独立日志
- 状态持久化: 连接配置存储在数据库中

#### 5. 行业采纳情况

截至 2025.11 (MCP 一周年):
- **Anthropic**: Claude 原生支持
- **OpenAI**: ChatGPT、Agents SDK、Responses API 全面集成
- **Google**: Gemini、Gemini CLI、Google Maps MCP server
- **Microsoft**: Copilot Studio、Azure AI Agent Service、Windows 原生支持 (计划中)
- **AWS**: Amazon Bedrock、Kiro、Strands、AgentCore
- **生态**: Playwright MCP (11.6k stars)、AWS Labs MCP (3.7k)、Terraform MCP (575)

---

### 值得关注的新兴模式

#### 1. Agent-as-a-Service (AaaS)

Codex App Server 代表了这一趋势 — 将 Agent 暴露为可编程服务:
- Thread / Turn / Item 协议 (JSON-RPC 2.0 over JSONL)
- 实时事件流 (agent event streaming)
- 审批流 (approval flows)
- spawn_agent / send_input 指令

#### 2. 事件流编排 (Event-Stream Orchestration)

"停止将 Agent 视为函数, 开始将其视为事件流的参与者":
- 计划是事件
- 中间产物是事件
- 失败、重试、审批、修正都是事件
- 编排系统观察、解释、丰富和路由事件

#### 3. Durable Execution (持久执行)

Agent 必须能从失败中恢复:
- Temporal、LangGraph、Inngest、Trigger.dev 提供 checkpointing 和恢复
- 模式: 将 Agent 任务视为 durable workflow, 而非短暂函数调用
- Goose 通过 `temporal-service/` (Go 调度器) 实现

#### 4. 流式 Tool 预览

Dyad (Electron AI 编辑器) 的创新:
- 模型流式输出 tool 参数时, 实时解析部分 JSON (使用 jsonrepair)
- 渲染工具操作的实时预览
- 在工具完成前提供可见性

#### 5. Code-as-Tool (代码即工具)

Anthropic 工程博客提出: 直接 tool 调用会消耗大量上下文 (每个定义和结果)。更高效的方式是 **Agent 编写代码来调用工具**, 而非逐个调用。通过 MCP 暴露的工具可以被 Agent 组合成代码脚本执行。

#### 6. BAML: 类型安全的 Agent 开发

BoundaryML 的 BAML 提供了一种声明式方法:
- "Prompts are Functions" — 每个 LLM 交互定义为带类型输入/输出的函数
- Schema-Aligned Parsing (SAP) — 从任何模型提取结构化输出, 即使模型不原生支持 tool calling
- Rust 编译器生成目标语言的类型安全客户端代码
- 声明式 client 配置 (重试策略、fallback 机制)

#### 7. Multi-Provider Harness

不绑定单一模型提供商:
- Goose: 真正的 model-agnostic, 支持同时配置多个模型
- rig: 20+ provider 统一接口, Capable<T>/Nothing 能力声明
- OpenAI Codex 团队: "Provider-agnostic design means we can switch models without rebuilding"
- 三阶段可用不同模型: planning 用强模型, building 用快模型, validation 用另一个

#### 8. Agent 安全纵深防御

arxiv 2603.05344 描述的 Claude Code 安全系统包含多个独立层:
- Approval 系统 (用户确认高风险操作)
- 危险命令检测
- Hooks (pre/post 钩子)
- Stale-read 检测 (防止基于过时文件内容做决策)
- Plan mode 限制
- Doom loop 检测
- 迭代上限
- 协作取消

---

## 对 octo-sandbox 的启示

基于以上行业研究, 以下是对 octo-sandbox 项目的具体建议:

### 已经做对的

1. **Rust 执行层** — 与行业趋势完全一致
2. **MCP 集成** (McpManager / McpToolBridge / McpStorage) — 架构正确
3. **分层记忆** (L0 WorkingMemory / L1 SessionMemory / L2 MemoryStore / KnowledgeGraph) — 与行业三层模型对齐
4. **Context Engineering** (SystemPromptBuilder / ContextBudgetManager / ContextPruner) — 对应 Anthropic 三原则
5. **Agent 三层架构** (AgentRuntime -> AgentExecutor -> AgentLoop) — 与 Goose 的 Agent + Extension + Provider 类似
6. **安全系统** (SecurityPolicy / CommandRiskLevel / AutonomyLevel) — 与行业安全纵深趋势一致

### 可以改进的方向

1. **Auto-Compaction**: 参考 Goose 的 80% 阈值自动摘要化机制
2. **Subagent 隔离**: 参考 Goose 的子 Agent 模式, 在独立上下文中执行子任务
3. **Tool Annotations**: 为工具添加 read-only / destructive 标注
4. **Streaming Tool Preview**: 参考 Dyad 的实时 tool 参数解析预览
5. **Repository-First Documentation**: 参考 OpenAI Codex 的 AGENTS.md 模式
6. **Durable Execution**: 考虑任务 checkpointing 和失败恢复
7. **Graph Memory**: 考虑在 KnowledgeGraph 基础上增强关系推理能力
8. **Dynamic Tool Discovery**: 参考 rig 的 VectorStoreIndex 动态工具采样
9. **Provider Chain 增强**: 考虑三阶段不同模型策略 (planning/building/validation)
10. **Event-Stream 编排**: 考虑将 Agent 执行模型从 "调用栈" 演进为 "事件流"

---

## 参考来源

### 核心文章
- Philipp Schmid, "The importance of Agent Harness in 2026" (philschmid.de)
- Hugo Nogueira, "The Agent Harness: Why 2026 is About Infrastructure, Not Intelligence" (hugo.im)
- OpenAI, "Harness engineering: leveraging Codex in an agent-first world" (openai.com)
- Anthropic, "Effective harnesses for long-running agents" (anthropic.com, 2025.11)
- Anthropic, "Effective context engineering for AI agents" (anthropic.com)
- Anthropic, "Code execution with MCP" (anthropic.com, 2025.11)
- arxiv 2603.05344, "Building AI Coding Agents for the Terminal: Scaffolding, Harness..."
- arxiv 2603.04740, "Constitutional Memory Architecture" (2026.03)
- Braintrust, "The canonical agent architecture: A while loop with tools"
- Victor Dibia, "The Agent Execution Loop: How to Build an AI Agent From Scratch"
- JetBrains, "State of Rust Ecosystem 2025" (2026.02)
- JetBrains Research, "Efficient Context Management" (2025.12)
- NxCode, "Harness Engineering: The Complete Guide" (2026.03)
- Ian Bull, "2026 Prediction - The Year Agents Get S#&t Done"

### 框架文档
- block/goose (GitHub, AGENTS.md, HOWTOAI.md, DeepWiki)
- 0xPlaygrounds/rig (GitHub, crates.io rig-core v0.32, DeepWiki)
- BoundaryML/baml (DeepWiki)
- swarms-rs (docs.rs)
- ZeroClaw (zeroclaws.io)

### MCP 生态
- MCP 官方博客, "One Year of MCP" (2025.11)
- Figma, "What is Model Context Protocol (MCP)?"
- Gartner 2025 Software Engineering Survey (via onereach.ai)
