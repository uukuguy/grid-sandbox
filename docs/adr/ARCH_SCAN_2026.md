# Octo-Engine 架构扫描报告 (2026-03-07)

> 本文档记录 octo-engine 架构全景和 ADR 文档更新状态

---

## 一、当前 ADR 状态

| ADR 文件 | 状态 | 内容 |
|----------|------|------|
| `ADR_AGENT_ARCHITECTURE.md` | 占位符 | 包含空存根 (ADR-013, ADR-014) |
| `ADR_MCP_INTEGRATION.md` | 占位符 | 包含空存根 |
| `ADR_MULTI_AGENT_ORCHESTRATION.md` | 完整 | ADR-006 到 ADR-012 |
| `ADR_SECURITY_REFACTORING.md` | 完整 | ADR-001 到 ADR-007 |

---

## 二、架构域清单 (22 个模块)

| # | 模块 | 关键组件 | ADR 状态 |
|---|------|---------|---------|
| 1 | **agent/** | AgentRuntime, AgentExecutor, AgentLoop, AgentCatalog, AgentRouter, AgentStore, ManifestLoader | 占位符 |
| 2 | **memory/** | WorkingMemory, SqliteMemoryStore, FtsStore, SemanticMemory, KnowledgeGraph, VectorIndex (HNSW), HybridQueryEngine | 缺失 |
| 3 | **mcp/** | McpManager, McpClient (stdio/SSE), McpToolBridge | 占位符 |
| 4 | **hooks/** | HookRegistry, HookHandler, HookPoint (11 个钩子点) | 缺失 |
| 5 | **event/** | EventBus, EventStore, ProjectionEngine, StateReconstructor | 缺失 |
| 6 | **security/** | AiDefence, InjectionDetector, PiiScanner, OutputValidator | 完整 |
| 7 | **secret/** | CredentialVault, EncryptedStore, Taint tracking | 缺失 |
| 8 | **context/** | SystemPromptBuilder, ContextPruner, TokenBudgetManager | 缺失 |
| 9 | **providers/** | Provider trait, Anthropic/OpenAI adapters, ProviderChain | 缺失 |
| 10 | **tools/** | ToolRegistry, Tool trait, Built-in tools | 缺失 |
| 11 | **skills/** | SkillLoader, SkillRegistry | 缺失 |
| 12 | **skill_runtime/** | SkillRuntime, SkillContext | 缺失 |
| 13 | **session/** | SessionStore (SQLite/InMemory) | 缺失 |
| 14 | **scheduler/** | Cron-based scheduler, SQLite storage | 缺失 |
| 15 | **auth/** | ApiKey, Roles, Middleware, HMAC | 完整(安全ADR) |
| 16 | **audit/** | AuditEvent, AuditRecord, AuditStorage | 缺失 |
| 17 | **metrics/** | MetricsRegistry, Counter, Gauge, Histogram | 缺失 |
| 18 | **sandbox/** | SandboxManager, Subprocess, WASM, Docker adapters | 缺失 |
| 19 | **db/** | SQLite wrapper with migrations | 缺失 |
| 20 | **extension/** | WASM plugin host with hostcall intercept | 缺失 |
| 21 | **logging/** | Structured logging (pretty/JSON) | 缺失 |
| 22 | **metering/** | Token usage metering, snapshots | 缺失 |

---

## 三、核心架构详解

### 3.1 Memory System (四层架构)

```
Layer 0 (Working)     → InMemoryWorkingMemory, SqliteWorkingMemory
Layer 1 (Session)      → SqliteSessionStore
Layer 2 (Persistent)   → SqliteMemoryStore, FtsStore, SemanticMemory
Layer 3 (Knowledge)    → KnowledgeGraph, GraphStore
```

**向量搜索**:
- `HnswIndex`: 基于 hnsw_rs 的近似最近邻搜索
- `HybridQueryEngine`: 融合向量 + FTS 混合查询

**预算管理**:
- `TokenBudgetManager`: 上下文预算控制

### 3.2 Hooks System (11 个钩子点)

| 钩子点 | 触发时机 |
|--------|---------|
| PreToolUse | 工具调用前 |
| PostToolUse | 工具调用后 |
| PreTask | 任务开始前 |
| PostTask | 任务完成后 |
| SessionStart | 会话开始 |
| SessionEnd | 会话结束 |
| ContextDegraded | 上下文降级 |
| LoopTurnStart | Agent 循环开始 |
| LoopTurnEnd | Agent 循环结束 |
| AgentRoute | Agent 路由决策 |
| Notify | 通知事件 |

### 3.3 Event Sourcing

- **EventBus**: broadcast channel + ring buffer
- **EventStore**: SQLite 持久化
- **ProjectionEngine**: 读模型构建，检查点线程安全
- **StateReconstructor**: 状态重放，事件回溯

### 3.4 Security System

- **AiDefence**:
  - `InjectionDetector`: 注入攻击检测 (24 关键词)
  - `PiiScanner`: PII 检测 (邮箱/电话/SSN)
  - `OutputValidator`: 输出验证
- **SecurityPolicy**:
  - `PathValidator`: 路径验证
  - `CommandRiskLevel`: 命令风险分级
  - `AutonomyLevel`: 自主级别
- **Secret**:
  - `CredentialVault`: AES-GCM 加密
  - `Taint tracking`: 污点追踪

---

## 四、ADR 更新建议

### 优先级 1 (关键 - 占位符)

1. **Agent Architecture ADR**: 补充
   - AgentRuntime → AgentExecutor → AgentLoop 流程
   - AgentCatalog, AgentStore 持久化
   - AgentRouter (route_task API)
   - ManifestLoader (YAML agents)

2. **MCP Integration ADR**: 补充
   - McpManager 生命周期
   - McpClient (stdio/SSE)
   - McpToolBridge

### 优先级 2 (高 - 缺失)

3. **Memory System ADR**: 新建
   - 四层内存架构
   - HNSW 向量搜索
   - 混合查询引擎

4. **Hooks System ADR**: 新建
   - 11 个钩子点
   - HookRegistry
   - HookHandler trait

5. **Event Sourcing ADR**: 新建
   - EventStore
   - Projections
   - StateReconstructor

6. **Secret Management ADR**: 新建
   - CredentialVault
   - Taint tracking

### 优先级 3 (中 - 缺失)

7. Context Engineering, Providers, Skills, Scheduler, Audit, Metrics, Sandbox, DB, Extension, Logging, Metering

---

## 五、已合并 PR 记录

| PR# | 标题 | 日期 |
|-----|------|------|
| #4 | test(memory): add unit tests for WorkingMemory and ContextInjector | 2026-03-07 |
| #3 | fix(p1): security hardening, architecture reliability | 2026-03-06 |
| #2 | feat: RuFlo Phase 1+2 | 2026-03-06 |
| #1 | feat: octo-engine P0 core capabilities | 2026-03-06 |
