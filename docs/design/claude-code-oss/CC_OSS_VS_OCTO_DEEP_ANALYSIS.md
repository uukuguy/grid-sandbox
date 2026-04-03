# CC-OSS vs Octo 深度代码级比较分析（修订版）

> 基于 CC-OSS 源码（383K 行 TypeScript，43 个模块）和 Octo 源码（~80K 行 Rust + ~15K 行 TypeScript）的**双向逐文件阅读**。
> 生成日期: 2026-04-03 | 修订: 基于 Octo 8 轮 agent 探索的实际代码分析

---

## 一、总览对比

| 维度 | CC-OSS | Octo |
|------|--------|------|
| **语言** | TypeScript (Bun runtime) | Rust (Tokio) + TypeScript/React |
| **代码量** | ~383K 行 TS | ~80K Rust + ~15K TS |
| **架构风格** | 单体 CLI (Node/Ink TUI) | 分层 Monorepo (Engine + Server + CLI + Web) |
| **Agent Loop** | `async function* query()` 异步生成器 | `run_agent_loop_inner()` 1300+ 行单循环 (harness.rs) |
| **上下文工程** | 5 级压缩管线 | **6 级降级策略** + CompactionPipeline + ObservationMasker + Collapse |
| **工具系统** | ~50+ 内建 + 并发分区 | ~25 内建 + **已有并发执行** (parallel.rs, Semaphore) |
| **权限系统** | 7 层配置源 + 25 Hook 事件 + Rule DSL | **6 层权限引擎** + **Permission Rule DSL** (glob) + 31 种 AgentEvent |
| **MCP** | 8 种传输 + OAuth | stdio + SSE + **OAuth 2.1 PKCE** + Tool Annotations 映射 |
| **TUI** | Ink fork + Vim 模式 + 147 组件 | Ratatui + **30+ 快捷键** + **Vim Normal/Insert/Visual** |
| **记忆系统** | Memdir (4 类型文件) | **5 层** (L0-L2 + KG + FTS5) + MemoryInjector + ObservationMasker |
| **多 Agent** | Coordinator Mode + Teammate/Dream | AgentCatalog + SubAgent + **Team (Leader/Worker)** + **Autonomous Trigger** |
| **安全** | Workspace Trust + Hook 信任检查 | **4 层 SafetyPipeline** (注入检测 + PII + Canary + 凭证清洗) + AIDefence |
| **Provider** | Anthropic + Bedrock + Vertex + Foundry | Anthropic + OpenAI + **8 种错误分类** + ProviderChain (健康检查 + P50/P99) |

---

## 二、核心维度深度对比（修订版）

### 2.1 Agent Loop 架构

#### CC-OSS 实现 (`src/query.ts`, 1732 行)

```
query() [async generator wrapper]
  └─> queryLoop() [while(true)]
       1. Memory Prefetch (非阻塞)
       2. Tool Result Budgeting
       3. History Snipping
       4. Microcompaction (缓存友好)
       5. Context Collapse
       6. Autocompaction
       7. API 调用 (流式)
       8. StreamingToolExecutor (并发/串行分区)
       9. Continue/Stop 决策
```

#### Octo 实现 (`harness.rs`, 1949 行 — **实际远比之前认为的复杂**)

```
run_agent_loop_inner() [loop {}]
  每次迭代:
  1. LLM 流式调用 + PTL Recovery (压缩 → 截断回退)
  2. 流消费 + 流错误恢复 (MAX_STREAM_ERROR_RETRIES=2)
  3. 文本工具调用恢复 (parse_tool_calls_from_text)
  4. 畸形工具调用检测 + 重试 (MAX_MALFORMED_RETRIES=2)
  5. Token 升级 (AR-T1, escalate max_tokens)
  6. 自动续写 (ContinuationTracker, 最多 3 次, 120K 字符上限)
  7. AIDefence + SafetyPipeline 输出检查
  8. 工具执行 → 两条路径:
     A. 并发执行 (parallel.rs, Semaphore, max_parallel=8)
     B. 顺序执行 + PreToolUse/PostToolUse Hooks
  9. Tool Result 预算裁剪 (soft_trim, 15%/30% context window)
  10. Loop Guard 防无限循环
```

#### **修正后的差距分析**

| 能力 | CC-OSS | Octo | 实际状态 |
|------|--------|------|---------|
| 工具并发执行 | ✅ isConcurrencySafe 分区 | ✅ **已有** `execute_parallel()` + Semaphore(max=8) | **Octo 已实现**，但默认关闭 (`enable_parallel=false`) |
| 流式工具执行（边收 API 边执行） | ✅ StreamingToolExecutor | ❌ 工具等待完整 API 响应后批量执行 | **真正差距** |
| 并发安全标记 | ✅ `isConcurrencySafe(input)` | ✅ **已有** `is_concurrency_safe()` trait 方法 | **Octo 已有**，但 harness 未利用做分区 |
| PTL Recovery | ✅ reactive compact + retry | ✅ **已有** `is_prompt_too_long()` → CompactionPipeline → 截断回退 | **双方都有** |
| Max-Tokens Recovery | ✅ 增大窗口 + retry | ✅ **已有** Token Escalation (AR-T1) + ContinuationTracker (3 次) | **双方都有** |
| 流错误恢复 | ✅ tombstone + retry | ✅ **已有** MAX_STREAM_ERROR_RETRIES=2 | **双方都有** |
| 畸形工具调用恢复 | ❌ 依赖 Zod 验证 | ✅ **Octo 更强** `detect_malformed_tool_call()` + retry 提示 | **Octo 独有** |
| Token Budget 自动续写 | ✅ taskBudget + diminishing returns | ✅ **已有** ContinuationTracker (3 次, 120K 上限) | **双方都有**（CC-OSS 有 diminishing returns 检测，Octo 用固定上限） |
| Loop Guard | ❌ 无显式防护 | ✅ **Octo 独有** 检测重复调用、参数循环 | **Octo 独有** |
| Tool Interceptor | ❌ | ✅ **Octo 独有** P1-3 工具调用拦截器 | **Octo 独有** |

**结论**: 之前报告严重低估了 Octo 的 Agent Loop 能力。Octo 的 harness.rs **已经实现了 CC-OSS 的大部分核心 recovery 机制**，且有 CC-OSS 没有的 Loop Guard 和 Tool Interceptor。

---

### 2.2 上下文工程

#### CC-OSS 的 5 级压缩管线

1. Tool Result Budget → 2. History Snipping → 3. Microcompact → 4. Context Collapse → 5. Autocompact

#### Octo 的上下文管理（**实际有 6 级降级 + 4 个独立组件**）

**6 级渐进降级策略** (`budget.rs`):

| 级别 | 使用率 | 策略 |
|------|--------|------|
| None | < 60% | 无动作 |
| SoftTrim | 60-70% | 工具结果头尾裁剪 |
| AutoCompaction | 70-90% | 保留最近 10 条消息 |
| OverflowCompaction | > 90% | 保留最近 4 条 + Memory Flush |
| ToolResultTruncation | 压缩后仍超 | 截断至 8KB |
| FinalError | 全部失效 | 终止 |

**4 个独立组件**:
1. **CompactionPipeline** (689 行): 6 步 LLM 驱动压缩 + PTL 自重试 + Zone B/B+/B++ 状态重建
2. **ContextCollapser** (340 行): 消息重要性评分 (0-100) + 一行摘要折叠
3. **ObservationMasker** (326 行): 等价于 CC-OSS 的 Microcompact — 旧轮工具输出掩码 + 启发式摘要
4. **ContextPruner** (520 行): 3 种策略 (Truncate/Summarize/MoveToWorkspace)

**额外**: Tool Result 动态预算 (`tool_result_budget()`): soft = 15% context window (8K-50K), hard = 30% (30K-200K)

#### **修正后的差距分析**

| 能力 | CC-OSS | Octo | 实际状态 |
|------|--------|------|---------|
| Tool Result Budget | ✅ `applyToolResultBudget()` | ✅ **已有** `soft_trim_tool_result()` + 动态预算 (15%/30% context) | **双方都有**，Octo 按 context window 比例计算更灵活 |
| Microcompact | ✅ 缓存友好增量压缩 | ✅ **ObservationMasker 等价** — 旧轮工具输出掩码 + 启发式摘要 | **双方都有** |
| History Snipping | ✅ 自动裁剪旧段 | ✅ **已有** `[SNIP]` 标记 + CompactionPipeline Snip 模式 | Octo 是手动触发，CC-OSS 自动 |
| Context Collapse | ✅ 折叠视图投影 | ✅ **已有** ContextCollapser (AP-T9) — 消息评分 + 一行摘要 | **双方都有** |
| Autocompact | ✅ LLM 摘要 | ✅ **已有** CompactionPipeline 6 步 LLM 压缩 + PTL 自重试 | **双方都有** |
| 6 级降级策略 | ❌ 按需触发各级 | ✅ **Octo 独有** 基于使用率的渐进降级 | **Octo 更系统化** |
| Prompt Caching 集成 | ✅ Beta headers + cache_control | ⚠️ `build_separated()` 支持分离但 **API 层未实现** | **真正差距** |
| Zone B/B+/B++ 状态重建 | ❌ | ✅ **Octo 独有** 压缩后重注入记忆 + 技能 + 会话摘要 | **Octo 独有** |

**结论**: Octo 的上下文工程实际上**不弱于 CC-OSS**，甚至在降级策略和状态重建方面更系统化。唯一真正差距是 **Prompt Caching API 层集成**。

---

### 2.3 工具系统

#### Octo Tool Trait（**实际比之前认为的丰富得多**）

```rust
pub trait Tool: Send + Sync {
    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<ToolOutput>;
    async fn execute_with_progress(&self, params: Value, ctx: &ToolContext,
                                    on_progress: Option<ProgressCallback>) -> Result<ToolOutput>;
    fn is_concurrency_safe(&self) -> bool;     // ✅ 已有！
    fn execution_timeout(&self) -> Duration;    // ✅ 每工具超时
    fn rate_limit(&self) -> u32;               // ✅ 每分钟限速
    fn approval(&self) -> ApprovalRequirement; // ✅ 批准要求
    fn risk_level(&self) -> RiskLevel;         // ✅ 风险等级
    fn is_destructive(&self) -> bool;          // ✅ 破坏性标记
    fn is_read_only(&self) -> bool;            // ✅ 只读标记
    fn validate_input(&self) -> Result<()>;    // ✅ 输入验证
}
```

#### **修正后的差距分析**

| 能力 | CC-OSS | Octo | 实际状态 |
|------|--------|------|---------|
| 并发安全标记 | ✅ `isConcurrencySafe(input)` | ✅ `is_concurrency_safe()` | **双方都有** |
| 并行执行框架 | ✅ | ✅ `execute_parallel()` + Semaphore(max=8) | **双方都有**，Octo 默认关闭 |
| **基于 concurrency_safe 的分区** | ✅ 按标记自动分区 | ❌ harness 未利用 `is_concurrency_safe()` 做分区 | **差距**: Octo 并发时不区分安全/不安全 |
| 流式工具执行 | ✅ 边收 API 边执行 | ❌ 等待完整响应后执行 | **真正差距** |
| 进度回调 | ✅ AsyncGenerator yield | ✅ `execute_with_progress()` + ProgressCallback | **双方都有** |
| 每工具超时 | ⚠️ 全局超时 | ✅ **Octo 更强** `execution_timeout()` 每工具独立 | **Octo 更好** |
| 速率限制 | ❌ | ✅ **Octo 独有** `rate_limit()` 60 秒滑动窗口 | **Octo 独有** |
| 输入验证 | ✅ Zod schema | ✅ `validate_input()` | **双方都有** |
| 工具搜索 | ✅ EXPERIMENTAL_TOOL_SEARCH | ✅ ToolSearchIndex (hybrid) | **双方都有** |
| Context Modifier | ✅ 工具修改后续上下文 | ❌ | CC-OSS 独有（使用频率低） |

---

### 2.4 Hooks & 权限系统

#### Octo 权限系统（**实际远比之前认为的完善**）

**6 层优先级权限引擎** (`permission_engine.rs`):
```
Platform (1) > Tenant (2) > Project (3) > User (4) > Session (5) > ToolDefault (6)
```

**Permission Rule DSL** (`permission_rule.rs`) — **已实现！**:
```
bash(git *)              — 匹配 git 开头的命令
file_edit(src/**/*.rs)   — 匹配 Rust 文件编辑
web_fetch(*.example.com) — 匹配特定域名
```

**31 种 AgentEvent** (`events.rs`):
- 流: TextDelta/Complete, ThinkingDelta/Complete
- 工具: ToolStart, ToolResult, ToolExecution, ToolProgress
- 安全: ApprovalRequired, SecurityBlocked, InteractionRequested
- 上下文: ContextDegraded, ContextCompacted, MemoryFlushed
- 迭代: IterationStart/End, RetryingMalformedToolCall
- 自治: AutonomousResumed/Sleeping/Tick/Paused/Exhausted
- 生命周期: PlanUpdate, SubAgentEvent, Error, Done, Completed

**4 层 SafetyPipeline** (`pipeline.rs`) — **CC-OSS 没有的**:
1. InjectionDetector — prompt injection 检测
2. PiiScanner — PII 清理
3. CanaryGuard — 系统提示泄露检测（带轮次轮换）
4. CredentialScrubber — 凭证模式检测 (sk-ant-, AKIA, ghp_, PEM)

**InteractionGate** — 3 种交互 (Question/Select/Confirm) + 60 秒超时

#### **修正后的差距分析**

| 能力 | CC-OSS | Octo | 实际状态 |
|------|--------|------|---------|
| Permission Rule DSL | ✅ `Bash(git *)` | ✅ **已有** `bash(git *)` glob 模式 | **双方都有** |
| 多层权限引擎 | ✅ 7 层 | ✅ **6 层** (Platform→Tenant→Project→User→Session→ToolDefault) | **基本等价** |
| Hook 事件数量 | ✅ 25 种 | ✅ **31 种 AgentEvent** | **Octo 事件更多** |
| PreToolUse/PostToolUse Hook | ✅ 完整 Hook 执行管线 | ✅ **已有** `HookPoint::PreToolUse/PostToolUse` (顺序执行路径) | **双方都有** |
| 5 种 Hook 执行机制 | ✅ Command/Prompt/Agent/HTTP/Function | ⚠️ 主要 Command + Function callback | **差距缩小** |
| Workspace Trust | ✅ 信任对话框 | ⚠️ `workspace_only` + `forbidden_paths` | **Octo 通过路径限制实现安全** |
| SafetyPipeline (注入检测等) | ❌ | ✅ **Octo 独有** 4 层安全管线 | **Octo 独有优势** |
| AIDefence | ❌ | ✅ **Octo 独有** AI 操控防御 | **Octo 独有优势** |
| InteractionGate | ✅ PermissionRequest | ✅ **已有** Question/Select/Confirm + 超时 | **双方都有** |

---

### 2.5 MCP 集成

#### Octo MCP（**实际比之前认为的完善得多**）

- **传输**: stdio + SSE (**不是缺少，rmcp 0.16 支持**)
- **OAuth 2.1**: **已实现！** OAuthConfig + PKCE + 自动令牌刷新 (`oauth.rs`)
- **Tool Annotations**: **已映射！** `readOnly→ReadOnly`, `destructive→Destructive`, `openWorld→HighRisk` (`bridge.rs`)
- **配置**: YAML + Claude Code 兼容格式 + 环境变量展开 (`${VAR:-default}`)
- **Session Ownership**: 跟踪哪个 session 安装了 MCP 服务器 (AJ-T4)
- **资源 URI 验证**: 防路径遍历 + 拒绝私网 IP (MCP-01)
- **反向 MCP Server**: Octo 可作为 MCP 服务器向外暴露 ToolRegistry

#### **修正后的差距分析**

| 能力 | CC-OSS | Octo | 实际状态 |
|------|--------|------|---------|
| OAuth 认证 | ✅ OAuth + 401 刷新 | ✅ **已有** OAuth 2.1 + PKCE + 自动刷新 | **双方都有** |
| Tool Annotations | ✅ readOnly/destructive→behavior | ✅ **已有** 映射到 RiskLevel | **双方都有** |
| HTTP/WS 传输 | ✅ 4 种额外传输 | ❌ 仅 stdio + SSE | **CC-OSS 更多** |
| Session 过期检测 | ✅ HTTP 404 + JSON-RPC -32001 | ❌ | **差距** |
| 资源 URI 安全验证 | ❌ | ✅ **Octo 独有** 防路径遍历 + 拒绝私网 | **Octo 独有** |
| 反向 MCP Server | ❌ | ✅ **Octo 独有** 向外暴露工具 | **Octo 独有** |
| 多层配置合并 | ✅ 5 层 | ⚠️ 2 层 (YAML + CC 兼容) | CC-OSS 更多层 |

---

### 2.6 TUI/CLI

#### Octo TUI（**实际远比之前认为的丰富**）

- **key_handler.rs** (1530 行): **30+ 快捷键**
- **Vim 模式**: **已有！** Normal/Insert/Visual 三种模式
- **工具循环**: Ctrl+O (单次) / Ctrl+Shift+O (全局)
- **Slash 命令**: **41 个**
- **外部编辑器**: 支持
- **StatusBar** (654 行): 品牌、模型、Token、时间、Git 状态、呼吸动画
- **CLI 命令**: **17 种** (run, ask, agent, session, memory, tool, mcp, config, auth, skill, root, init, eval, completions, doctor, dashboard, sandbox)

#### **修正后的差距分析**

| 能力 | CC-OSS | Octo | 实际状态 |
|------|--------|------|---------|
| Vim 模式 | ✅ 完整 (10 种命令状态, dot-repeat, registers) | ✅ **已有** Normal/Insert/Visual | **Octo 有基础 Vim**，CC-OSS 更完整 |
| 快捷键数量 | ✅ 50+ | ✅ **30+** | **基本等价** |
| 用户可配置快捷键 | ✅ `~/.claude/keybindings.json` | ❌ 硬编码 | **差距** |
| StatusBar | ✅ 基础 | ✅ **多行** 品牌+模型+Token+Git+动画 | **Octo 更丰富** |
| Slash 命令 | ✅ 100+ | ✅ **41 个** | CC-OSS 更多 |
| IDE Bridge | ✅ 36 文件 | ❌ | **不同定位** |
| Output Styles | ✅ 可自定义 | ❌ | **差距** |
| CLI 命令种类 | ✅ ~10 | ✅ **17 种** (含 eval, doctor, dashboard, sandbox) | **Octo 更丰富** |

---

### 2.7 多 Agent 协调

#### Octo 多 Agent（**实际比之前认为的丰富**）

- **AgentCatalog** (167 行): DashMap 多索引并发存储 (by_id/name/tag/tenant_id)
- **AgentStore** (125 行): SQLite 持久化
- **SubAgent** (150+ 行): 递归执行 + 深度/并发限制 + SubAgentContext
- **Team** (150+ 行): Leader/Worker 角色 + 团队管理
- **AutonomousScheduler** (125 行): 会话注册 + 生命周期管理
- **AutonomousTrigger**: TriggerSource trait + Channel + Polling + **Redis Streams** (feature-gated)
- **Autonomous Config**: idle/active sleep, 预算 (rounds/duration/cost), 4 种触发类型 (Manual/Cron/Webhook/MessageQueue)

#### **修正后的差距分析**

| 能力 | CC-OSS | Octo | 实际状态 |
|------|--------|------|---------|
| Coordinator Mode | ✅ 专用编排者提示 | ❌ | **真正差距** — 最值得引入 |
| Team 管理 | ❌ | ✅ **Octo 独有** Leader/Worker 角色 | **Octo 独有** |
| 自治触发 | ❌ | ✅ **Octo 独有** Cron/Webhook/Redis/Manual | **Octo 独有** |
| Dream Task | ✅ 后台长期任务 | ❌ | CC-OSS 独有 |
| Teammate (同进程) | ✅ | ❌ | CC-OSS 独有 |
| Agent 持久化 | ❌ 仅进程内 | ✅ **AgentStore SQLite** | **Octo 独有** |
| Agent 多索引查询 | ❌ | ✅ **DashMap by_id/name/tag/tenant** | **Octo 独有** |

---

### 2.8 Provider & 流式处理

#### Octo Provider（**实际有成熟的故障恢复**）

**8 种 LLM 错误分类** (`retry.rs`):

| 分类 | HTTP | 可重试 | 故障转移 | 路由策略 |
|------|------|--------|---------|---------|
| RateLimit | 429 | ✓ | ✗ | Retry |
| Overloaded | 529 | ✓ | ✓ | Retry |
| Timeout | 408/504 | ✓ | ✓ | Failover |
| ServiceError | 500/502/503 | ✓ | ✓ | Failover |
| BillingError | 402 | ✗ | ✗ | Fail |
| AuthError | 401/403 | ✗ | ✗ | Fail |
| ContextOverflow | - | ✗ | ✗ | CompactAndRetry |
| Unknown | - | ✓ | ✓ | Retry |

**ProviderChain** (`chain.rs`): 多实例 + 健康检查(30s) + P50/P99 延迟 + 故障转移跟踪

**ResponseCache** (`response_cache.rs`): SHA-256 LRU 缓存 (128 条目, TTL 300s)

#### **修正后的差距分析**

| 能力 | CC-OSS | Octo | 实际状态 |
|------|--------|------|---------|
| 429 vs 529 区分 | ✅ | ✅ **已有** 8 种错误分类 | **Octo 分类更精细** |
| Failover 链 | ✅ Primary→Fallback | ✅ **ProviderChain** 多实例 + 健康检查 | **Octo 更强** |
| Unattended 无限重试 | ✅ 5 分钟上限 | ❌ 固定 max_retries=3 | **差距** |
| Prompt Caching | ✅ Beta headers | ❌ **API 层未实现** | **真正差距** |
| 响应缓存 | ❌ | ✅ **Octo 独有** SHA-256 LRU | **Octo 独有** |
| P50/P99 监控 | ❌ | ✅ **Octo 独有** 延迟百分位跟踪 | **Octo 独有** |

---

## 三、修正后的真正差距（从 CC-OSS 引入的价值项）

之前的报告列了 25 项差距，经过 Octo 实际代码审查后，**大量"差距"实际已被 Octo 实现**。真正值得引入的项目大幅减少：

### P0 — 高价值引入

#### 1. 流式工具执行 (Streaming Tool Execution)

**这是唯一真正的 P0 差距**。

CC-OSS 的 `StreamingToolExecutor` 在 API 流式返回时，`tool_use` block 一出现就入队执行。Octo 当前等待完整 API 响应后才开始工具执行。

**引入方案**: 在 `consume_stream()` 阶段解析到 `tool_use` content block 时，立即用 `tokio::spawn` 启动 read-only 工具执行。

#### 2. 基于 `is_concurrency_safe()` 的自动分区

Octo 已有 `is_concurrency_safe()` trait 方法和 `execute_parallel()`，但 harness **没有利用 `is_concurrency_safe()` 来决定分区**。应该在并发执行路径中：
- `is_concurrency_safe() == true` → 并行 batch
- `is_concurrency_safe() == false` → 串行执行

---

### P1 — 值得引入

#### 3. Prompt Caching API 集成

Octo 的 `SystemPromptBuilder.build_separated()` 已支持静态/动态分离，但 Anthropic provider **未发送 `cache_control` 标记**。这是成本优化的低挂果实。

**引入方案**: Anthropic provider 请求中为 system prompt 的静态部分添加 `cache_control: { type: "ephemeral" }`。

#### 4. 自动 History Snipping

Octo 的 Snip 是手动触发（`[SNIP]` 标记），CC-OSS 自动在每轮前检测并裁剪。

**引入方案**: 在 budget 60-70% (SoftTrim 级别) 时自动触发 snip。

#### 5. Coordinator Mode

CC-OSS 的 Coordinator 系统提示 + Worker 工具子集限制 + 标准化通知格式是一个完整的编排模式。Octo 有 Team (Leader/Worker) 但无专用编排者提示。

**引入方案**: 当 AgentManifest 设置 `coordinator: true` 时，注入编排者系统提示，限制 worker 可用工具。

#### 6. Unattended 无限重试模式

CC-OSS 在无人值守场景（CI/CD、后台任务）支持无限重试 + 心跳。Octo 的 `max_retries=3` 在长时间运行的自治 agent 中可能不够。

**引入方案**: `RetryPolicy` 增加 `unattended` 模式，配合 Autonomous Agent 使用。

---

### P2 — 中期可选

| # | 功能 | 说明 |
|---|------|------|
| 7 | Hook 执行机制扩展 (Prompt/HTTP) | Octo 的 Hook 主要是 Function callback，增加 LLM prompt 和 HTTP webhook 评估 |
| 8 | 用户可配置快捷键 | CC-OSS `~/.claude/keybindings.json`，Octo 快捷键硬编码 |
| 9 | MCP Session 过期检测 | HTTP 404 + JSON-RPC -32001 自动重连 |
| 10 | Output Styles | Markdown frontmatter 自定义输出风格 |
| 11 | Vim dot-repeat/registers/text-objects | Octo 有基础 Vim，CC-OSS 的更完整 |
| 12 | MCP 多层配置合并 | 5 层 vs 2 层 |

---

## 四、Octo 独有优势（CC-OSS 完全没有的）

| 维度 | Octo 独有能力 | 说明 |
|------|-------------|------|
| **4 层 SafetyPipeline** | 注入检测 + PII 清洗 + Canary 泄露检测 + 凭证清洗 | CC-OSS 无类似机制 |
| **AIDefence** | AI 操控防御系统 | CC-OSS 无 |
| **6 级渐进降级** | 基于使用率的自动降级策略 | CC-OSS 按需触发各级 |
| **Loop Guard** | 检测无限循环、重复调用、参数循环 | CC-OSS 无 |
| **Tool Interceptor** | 工具调用拦截器 | CC-OSS 无 |
| **畸形工具调用恢复** | 从文本解析 + 智能重试 | CC-OSS 依赖 Zod |
| **Token Escalation** | 动态升级 max_tokens（不浪费续写配额） | CC-OSS 直接续写 |
| **每工具超时 + 速率限制** | `execution_timeout()` + `rate_limit()` | CC-OSS 全局超时 |
| **Agent 持久化** | AgentStore SQLite + DashMap 多索引 | CC-OSS 仅进程内 |
| **自治触发系统** | Cron + Webhook + Redis Streams + Manual | CC-OSS 无 |
| **Team 管理** | Leader/Worker 角色 | CC-OSS 无 |
| **安全沙箱** | Docker + WASM + SessionSandboxManager | CC-OSS 无容器沙箱 |
| **评估框架** | octo-eval (suites, scorers, benchmarks) | CC-OSS 无 |
| **多租户** | octo-platform-server (JWT + tenant isolation) | CC-OSS 单用户 |
| **响应缓存** | SHA-256 LRU (128 条目, TTL 300s) | CC-OSS 无 |
| **P50/P99 监控** | ProviderChain 延迟百分位跟踪 | CC-OSS 仅 analytics |
| **Zone B/B+/B++ 重建** | 压缩后重注入记忆 + 技能 + 会话摘要 | CC-OSS 无 |
| **反向 MCP Server** | Octo 可向外暴露 ToolRegistry | CC-OSS 无 |
| **资源 URI 安全** | 防路径遍历 + 拒绝私网 IP | CC-OSS 无 |
| **Rust 性能** | 编译时安全 + 零成本抽象 + 无 GC | Node.js runtime |

---

## 五、修正后的引入路线图

```
Phase AV (近期):
├── T1: 流式工具执行 (P0) — 解析 tool_use 时立即 spawn
├── T2: 并发安全分区 (P0) — harness 利用 is_concurrency_safe() 分区
├── T3: Prompt Caching (P1) — Anthropic provider cache_control
└── T4: 自动 History Snipping (P1) — SoftTrim 级别自动触发

Phase AW (中期):
├── T5: Coordinator Mode (P1) — 编排者系统提示 + 工具子集
├── T6: Unattended Retry (P1) — Autonomous Agent 无限重试
└── T7: Hook Prompt/HTTP 机制 (P2) — 扩展 Hook 执行方式
```

---

## 六、修正后的结论

### 之前报告的主要错误

1. **严重低估了 Octo 的 Agent Loop**: 实际有 PTL Recovery、Token Escalation、ContinuationTracker、畸形工具调用恢复、Loop Guard — 大部分 CC-OSS 的 recovery 机制都已实现
2. **误判"工具全部顺序执行"**: Octo 已有 `execute_parallel()` + `is_concurrency_safe()` trait，只是默认关闭且未做分区
3. **误判"无 Tool Result Budget"**: 已有 `soft_trim_tool_result()` + 动态预算 (15%/30% context window)
4. **误判"无 Permission Rule DSL"**: 已有 `bash(git *)` glob 模式匹配
5. **误判"无 OAuth"**: 已有 OAuth 2.1 + PKCE
6. **误判"无 MCP Tool Annotations 映射"**: bridge.rs 已实现 readOnly/destructive→RiskLevel
7. **误判"无 Vim 模式"**: 已有 Normal/Insert/Visual 三种模式
8. **误判"无上下文降级策略"**: 已有 6 级渐进降级，比 CC-OSS 更系统化

### 修正后的核心建议

**真正的差距只有 2 个 P0 + 4 个 P1**：

1. **P0: 流式工具执行** — 唯一能显著提升响应速度的改动
2. **P0: 并发安全分区** — 已有基础设施，只需 harness 连接
3. **P1: Prompt Caching** — 已有 `build_separated()`，只需 API 层集成
4. **P1: 自动 History Snipping** — 小改动大收益
5. **P1: Coordinator Mode** — 多 Agent 编排的缺失
6. **P1: Unattended Retry** — 自治 Agent 场景需要

**Octo 在安全 (SafetyPipeline)、记忆 (5 层)、自治 (Trigger)、评估 (octo-eval)、沙箱 (Docker/WASM)、多租户等方面远超 CC-OSS**。之前的报告因未读 Octo 当前代码而严重低估了项目的成熟度。
