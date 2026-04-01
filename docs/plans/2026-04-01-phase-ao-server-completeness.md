# Phase AO — octo-server 功能完善

> **范围**: 将 octo-engine 已有但 octo-server 未暴露的能力通过 REST API 输出
> **前置**: Phase AK+AL+AM 完成（20/20 tasks + 3 deferred resolved）
> **目标**: 消除 engine/server 功能断层，让前端和外部集成方能完整使用 engine 能力

---

## 当前基线

| 维度 | 状态 |
|------|------|
| Tests | 2476+ |
| DB Version | 13 |
| HEAD | 3948530 (main) |
| Server API 模块 | 23 个（agents, audit, budget, collaboration, config, eval_sessions, events, executions, mcp_logs, mcp_servers, mcp_tools, memories, metrics, providers, scheduler, sessions, skills, sync, tasks, tools, user_context, error） |
| 未暴露 Engine 模块 | metering, memory/graph(KG), secret/vault, security/policy+tracker+ai_defence, sandbox/session_sandbox, hooks/registry+declarative+wasm, context/budget+manager |

---

## Phase AO-1 — Token 用量与费用（Metering API）

> **优先级**: P1 — 用户最关心"花了多少钱"
> **Engine 基础**: `Metering` (snapshot/reset), `MeteringStorage` (summary_by_session/model/global), `ModelPricing` (estimate_cost)

### AO-T1: Metering REST 端点

**端点设计**:
```
GET  /api/v1/metering/snapshot     → 实时 token 消耗（input/output/requests/errors/avg）
GET  /api/v1/metering/summary      → 按模型分组的累计用量 + 费用估算
GET  /api/v1/metering/by-session   → 按 session 分组的用量
POST /api/v1/metering/reset        → 重置实时计数器
```

**实现要点**:
- 新增 `crates/octo-server/src/api/metering.rs`
- `AppState` 需暴露 `Metering` 和 `MeteringStorage` 引用
- `snapshot` → 调用 `Metering::snapshot()` + `ModelPricing::estimate_cost()`
- `summary` → 调用 `MeteringStorage::summary_by_model()` + 逐模型 `estimate_cost()`
- `by-session` → 调用 `MeteringStorage::summary_by_session()`
- `reset` → 调用 `Metering::reset()`

**文件**: `api/metering.rs`, `api/mod.rs`, `state.rs`
**测试**: `crates/octo-server/tests/api_metering.rs`

---

## Phase AO-2 — 知识图谱 API（Knowledge Graph）

> **优先级**: P1 — 前端 KG 可视化 (AL-D1) 的前置
> **Engine 基础**: `KnowledgeGraph` (Entity CRUD, Relation, BFS, 路径搜索, stats), `GraphStore` (SQLite 持久化, FTS 搜索)

### AO-T2: Knowledge Graph REST 端点

**端点设计**:
```
GET    /api/v1/knowledge-graph/entities          → FTS 搜索 (?q=xxx&limit=50)
POST   /api/v1/knowledge-graph/entities          → 创建实体
GET    /api/v1/knowledge-graph/entities/:id       → 获取实体详情
DELETE /api/v1/knowledge-graph/entities/:id       → 删除实体（级联删除关系）
GET    /api/v1/knowledge-graph/entities/:id/relations  → 获取实体的出入关系
POST   /api/v1/knowledge-graph/relations          → 创建关系
GET    /api/v1/knowledge-graph/stats              → 图统计（entity_count, relation_count, types 分布）
GET    /api/v1/knowledge-graph/traverse           → BFS 遍历 (?start=X&depth=N)
GET    /api/v1/knowledge-graph/path               → 最短路径 (?from=X&to=Y)
```

**实现要点**:
- 新增 `crates/octo-server/src/api/knowledge_graph.rs`
- `AppState` 需暴露 `KnowledgeGraph` (内存) 和 `GraphStore` (SQLite) 引用
- 写操作同时更新内存图和 SQLite 持久化
- FTS 搜索走 `GraphStore::fts_search()`
- `traverse` 返回 `Vec<{id, entity, depth}>` 用于前端力导向图渲染

**文件**: `api/knowledge_graph.rs`, `api/mod.rs`, `state.rs`
**测试**: `crates/octo-server/tests/api_knowledge_graph.rs`

---

## Phase AO-3 — Hooks 管理 API

> **优先级**: P2 — 三层 hook 系统（编程式/策略/声明式）+ WASM 插件需要可视化管理
> **Engine 基础**: `HookRegistry` (register/execute/list), 声明式 hooks.yaml loader, WASM plugin loader

### AO-T3: Hooks REST 端点

**端点设计**:
```
GET    /api/v1/hooks                    → 已注册 hook 列表（按 HookPoint 分组，含 handler 数量）
GET    /api/v1/hooks/points             → 所有 HookPoint 枚举值
POST   /api/v1/hooks/reload             → 重载声明式 hooks.yaml（热更新）
GET    /api/v1/hooks/wasm               → WASM 插件列表（名称、状态、capabilities）
POST   /api/v1/hooks/wasm/:name/reload  → 重载单个 WASM 插件
```

**实现要点**:
- 新增 `crates/octo-server/src/api/hooks.rs`
- `HookRegistry` 需新增 `pub async fn list_all(&self) -> Vec<HookPointInfo>` 方法
- WASM 插件状态从 `WasmHookLoader` 获取
- `reload` 调用声明式 loader 重新解析 hooks.yaml

**文件**: `api/hooks.rs`, `api/mod.rs`, engine `hooks/registry.rs`
**测试**: `crates/octo-server/tests/api_hooks.rs`

---

## Phase AO-4 — 安全策略与 AI 防御 API

> **优先级**: P2 — 运行时可调自主级别，可查安全状态
> **Engine 基础**: `SecurityPolicy` (autonomy/command_risk/path_check), `ActionTracker`, `AiDefence` (injection/PII 检测)

### AO-T4: Security Policy REST 端点

**端点设计**:
```
GET    /api/v1/security/policy         → 当前策略（autonomy_level, allowed_dirs, blocked_commands）
PUT    /api/v1/security/policy         → 更新策略（仅允许调整 autonomy_level + allowed_dirs）
GET    /api/v1/security/tracker        → 操作计数器（窗口内操作数）
POST   /api/v1/security/check-command  → 命令风险评估（传入 command 返回 risk_level + requires_approval）
```

### AO-T5: AI Defence REST 端点

**端点设计**:
```
POST   /api/v1/security/scan           → 综合扫描（injection + PII 检测）
POST   /api/v1/security/pii/redact     → PII 脱敏
GET    /api/v1/security/defence/status  → AI Defence 启用状态
```

**实现要点**:
- 新增 `crates/octo-server/src/api/security.rs`
- Policy PUT 需验证权限（仅 Admin 角色）
- `check-command` 是只读操作，返回评估结果但不执行
- PII 扫描和脱敏面向前端消息预处理场景

**文件**: `api/security.rs`, `api/mod.rs`
**测试**: `crates/octo-server/tests/api_security_policy.rs`

---

## Phase AO-5 — Secret Vault API

> **优先级**: P2 — 安全凭证管理，替代 .env 手动配置
> **Engine 基础**: `CredentialVault` (AES-GCM 加密, set/get/encrypt/decrypt)

### AO-T6: Secret Vault REST 端点

**端点设计**:
```
GET    /api/v1/secrets                  → 列出所有 secret 名称（不返回值）
POST   /api/v1/secrets                  → 存储 secret（{name, value}）
DELETE /api/v1/secrets/:name            → 删除 secret
POST   /api/v1/secrets/verify           → 验证 vault 解锁状态
```

**实现要点**:
- 新增 `crates/octo-server/src/api/secrets.rs`
- **绝不返回 secret 值**，仅返回名称列表
- 需 Admin 角色才能写入/删除
- Vault 未解锁时返回 423 Locked

**文件**: `api/secrets.rs`, `api/mod.rs`
**测试**: `crates/octo-server/tests/api_secrets.rs`

---

## Phase AO-6 — Sandbox 管理 API

> **优先级**: P2 — 容器池可观察性
> **Engine 基础**: `SessionSandboxManager` (get_or_create/release/cleanup_idle/active_count/active_sessions)

### AO-T7: Sandbox REST 端点

**端点设计**:
```
GET    /api/v1/sandbox/status           → 活跃容器数、session 列表、配置摘要
GET    /api/v1/sandbox/sessions         → 活跃 sandbox session 详情列表
POST   /api/v1/sandbox/:session_id/release  → 手动释放容器
POST   /api/v1/sandbox/cleanup          → 触发 idle 清理
```

**实现要点**:
- 新增 `crates/octo-server/src/api/sandbox.rs`
- `AppState` 需暴露 `SessionSandboxManager` 引用（可选，仅当 SSM 初始化时）
- 如果 SSM 未初始化（Host 模式），返回 `{"mode": "host", "sandbox_available": false}`

**文件**: `api/sandbox.rs`, `api/mod.rs`, `state.rs`
**测试**: `crates/octo-server/tests/api_sandbox.rs`

---

## Phase AO-7 — 现有 API 补全 + Context 可观察性

> **优先级**: P2-P3 — 补齐现有端点的缺失操作

### AO-T8: Config 运行时更新

**端点设计**:
```
PUT    /api/v1/config                   → 运行时配置热更新（部分字段）
```

**可更新字段**（安全白名单）:
- `logging.format` (pretty/json)
- `server.cors_strict`
- `server.cors_origins`
- `provider.name` / `provider.model` — 切换 LLM provider
- `security.autonomy_level`

**不可更新**: port, host, db_path, tls — 需要重启

### AO-T9: Audit 增强

**端点设计**:
```
GET    /api/v1/audit/export             → 导出审计日志（JSON Lines, ?since=ISO8601&until=ISO8601）
DELETE /api/v1/audit                    → 清理过期审计日志（?before=ISO8601, 仅 Admin）
GET    /api/v1/audit/stats              → 审计统计（按操作类型/时间段分组）
```

### AO-T10: Context 可观察性

**端点设计**:
```
GET    /api/v1/context/snapshot         → 当前上下文预算快照（总窗口、已用、剩余、使用比率、降级等级）
GET    /api/v1/context/zones            → Zone A/B/C 分区使用明细
```

**文件**: `api/config.rs`, `api/audit.rs`, 新增 `api/context.rs`
**测试**: `crates/octo-server/tests/api_config_update.rs`

---

## 执行分组与依赖

```
AO-1 (T1 Metering)           ─┐
AO-2 (T2 KG)                 ─┼─ Wave 1（P1，可并行）
                               │
AO-3 (T3 Hooks)              ─┤
AO-4 (T4+T5 Security)        ─┼─ Wave 2（P2，可并行）
AO-5 (T6 Secrets)            ─┤
AO-6 (T7 Sandbox)            ─┘
                               │
AO-7 (T8+T9+T10 补全)        ─── Wave 3（P2-P3）
```

**Wave 1**: AO-1 + AO-2（2 tasks，独立，可并行）
**Wave 2**: AO-3 + AO-4 + AO-5 + AO-6（5 tasks，独立，可并行）
**Wave 3**: AO-7（3 tasks，依赖 Wave 1/2 的 AppState 扩展模式）

---

## 预估工作量

| Wave | Tasks | 预估 |
|------|-------|------|
| Wave 1 | T1 + T2 | 3-4 天 |
| Wave 2 | T3 + T4 + T5 + T6 + T7 | 4-5 天 |
| Wave 3 | T8 + T9 + T10 | 2-3 天 |
| **总计** | **10 tasks** | **~2 周** |

---

## 验收标准

1. **AO-1 完成后**: `GET /api/v1/metering/snapshot` 返回 token 用量 + 费用估算
2. **AO-2 完成后**: `GET /api/v1/knowledge-graph/entities?q=xxx` 返回搜索结果，`traverse` 返回 BFS 结果
3. **AO-3 完成后**: `GET /api/v1/hooks` 返回三层 hook 注册信息
4. **AO-4 完成后**: `POST /api/v1/security/check-command` 返回风险评估，`scan` 返回 PII 检测结果
5. **AO-5 完成后**: Secret 名称可列出但值不可读
6. **AO-6 完成后**: 容器池状态可查，手动释放可用
7. **AO-7 完成后**: Config 热更新、Audit 导出、Context 快照均可用
8. **全部完成后**: `make test` 全部通过（测试数 > 2476 + 新增）

---

## Deferred（Phase AO 不处理）

| ID | 描述 | 理由 |
|----|------|------|
| AO-D1 | WebSocket 订阅 metering 实时流 | 需前端配合，独立迭代 |
| AO-D2 | KG 图算法扩展（PageRank、社区检测） | 需明确产品场景 |
| AO-D3 | Hook 在线编辑（声明式 YAML 在线修改） | 安全风险高，需设计审批流 |
| AO-D4 | Secret rotation（自动轮换） | 需与 AK-D3 合并考虑 |
| Phase AN | octo-platform-server（多租户平台版） | 独立产品，独立规划 |

---

## Baseline

- **Tests**: 2476+
- **Commit**: 3948530 (HEAD of main)
- **DB Version**: 13
