# Octo-Server 开发路径规划

> **范围**: octo-server（单用户工作台服务）的安全加固、前端完善、可观测性增强
> **前置**: Phase AJ（多会话）完成，octo-engine 基座稳固（36 阶段，2476+ 测试）
> **不含**: octo-platform-server（多租户平台版），独立规划

---

## 当前基线

| 维度 | 状态 |
|------|------|
| Server API | 23 模块，WS 流式 + REST CRUD，认证/审计/限流已有 |
| 认证 | 3 模式（None/ApiKey/Full），HMAC-SHA256，4 级角色 |
| 多会话 | SessionRegistry + WS 路由 + REST 端点 + idle 回收 |
| Web 前端 | 8 页面（Chat/MCP/Memory/Tools/Debug/Schedule/Tasks/Collaboration）|
| 部署 | Dockerfile 多阶段 + docker-compose + Makefile 完整 |
| 测试 | 2476+ workspace 测试，5 个 server 集成测试文件 |

---

## Phase AK — Server 安全加固 + 生产就绪

> **目标**: 让 octo-server 可安全对外暴露，API 契约稳定

### G1: 安全响应头 + CORS 严格化（P1）

**AK-T1: 安全响应头中间件**
- 新增 `security_headers_middleware`，插入 router middleware 栈
- 添加: `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`, `Referrer-Policy: strict-origin-when-cross-origin`
- 条件添加: `Strict-Transport-Security`（仅 TLS 开启时）
- 不添加 CSP（API 服务不需要，留给反向代理/前端）
- 文件: `router.rs`
- 测试: 新增 `crates/octo-server/tests/api_security_headers.rs`

**AK-T2: CORS 生产模式严格化**
- `config.rs` 新增 `server.cors_strict: bool`（默认 false）
- 当 `cors_strict = true` 且 `cors_origins` 为空 → 启动时 warn 并拒绝 wildcard
- `router.rs` 读取配置，严格模式下要求显式 origin 列表
- 文件: `config.rs`, `router.rs`

### G2: API 版本化 + 健康检查（P1）

**AK-T3: API 统一 `/api/v1` 前缀**
- 当前状态: 部分端点在 `/api/*`，agent/skill 在 `/api/v1/*`，metrics 在 `/api/v1/metrics`
- 目标: 所有端点统一到 `/api/v1/*`
- 兼容: 保留 `/api/health` 不带版本（公共端点）
- 迁移策略:
  - `api/mod.rs` 的 `routes()` 返回值 nest 到 `/v1` 下
  - 去掉 `agents` 和 `skills` 的二次 `/v1` nest
  - 前端 `web/src/` 全局搜索替换 `/api/` → `/api/v1/`
  - Vite proxy 同步更新
- 文件: `router.rs`, `api/mod.rs`, `web/src/**/*.ts`, `web/vite.config.ts`
- 测试: 更新所有 server 集成测试的 URI

**AK-T4: 健康检查增强**
- `/api/health` 拆分为:
  - `liveness`: 进程存活（简单 200 OK，用于 k8s livenessProbe）
  - `readiness`: 依赖就绪（DB 连通 + provider 可达 + primary session 存在）
- 当前 `/api/health` 改为 readiness 语义
- 新增 `/api/health/live` 极简端点
- 文件: `router.rs`
- 测试: 更新 `api_health.rs`

### G3: 请求安全 + 优雅关闭（P2）

**AK-T5: 请求体大小限制 + 超时**
- Axum `DefaultBodyLimit` 设置为 10MB（可配置 `server.max_body_size`）
- 请求超时: `tower_http::timeout::TimeoutLayer` 30s（可配置 `server.request_timeout_secs`）
- 文件: `router.rs`, `config.rs`

**AK-T6: 优雅关闭（合并 AJ-D3）**
- 监听 SIGTERM/SIGINT（`tokio::signal`）
- 收到信号 → 停止接受新连接 → 等待活跃请求完成（grace period 30s）→ 停止所有 session → 关闭 DB
- `axum::serve(...).with_graceful_shutdown(shutdown_signal())`
- 文件: `main.rs`
- 测试: 手动验证（提供验证方案）

### G4: 测试（P1）

**AK-T7: 安全中间件测试**
- 安全头存在性检查
- CORS 严格模式测试
- API 版本路由测试
- 请求大小限制测试
- 文件: `crates/octo-server/tests/api_security.rs`

### 执行顺序

```
G1 (T1,T2) ← 可并行
   ↓
G2 (T3→T4) ← T3 是 breaking change，先做
   ↓
G3 (T5,T6) ← 可并行
   ↓
G4 (T7)
```

**预估**: 5-7 天

### Deferred

| ID | 描述 | 前置条件 | 状态 |
|----|------|---------|------|
| AK-D1 | Rate limiter 可配置化（每端点/每用户，非全局） | 有真实流量数据 | ⏳ |
| AK-D2 | WebSocket 认证增强（token 参数 vs header） | 前端多会话 UI | ✅ 已补 |
| AK-D3 | API Key 轮换机制（热更新，不重启） | 生产部署需求 | ⏳ |

---

## Phase AL — Web 前端完善 + 多会话 UI

> **目标**: web 端成为可日常使用的完整工作台
> **前置**: AK-T3（API 版本化）

### G1: 多会话 UI（P1，合并 AJ-D5）

**AL-T1: Session Tab 切换器**
- 顶部 Tab Bar 显示活跃 session 列表
- 新建 session 按钮（调用 `POST /api/v1/sessions/start`）
- 切换 session → WsManager 断开旧 WS → 重连 `/ws?session_id=xxx`
- 关闭 session 按钮（调用 `DELETE /api/v1/sessions/{id}/stop`）
- 状态: 新增 `activeSessionAtom`, `sessionsAtom` (Jotai)
- 文件: `web/src/components/SessionBar.tsx`, `web/src/atoms/session.ts`

**AL-T2: WS 连接状态指示器**
- 显示连接状态: 已连接(绿) / 重连中(黄) / 断开(红)
- 重连进度条（指数退避可视化）
- 断连后自动切换到最近的可用 session
- 文件: `web/src/components/ConnectionStatus.tsx`, `web/src/ws/manager.ts`

### G2: Chat 增强（P2）

**AL-T3: Markdown 渲染改进**
- 代码块语法高亮（Prism.js 或 Shiki）
- 表格渲染（支持 GFM 表格）
- 可折叠的长输出
- 文件: `web/src/components/chat/MessageBubble.tsx`

**AL-T4: 消息交互增强**
- 消息复制按钮
- Tool 调用结果折叠/展开
- Thinking 过程折叠显示
- 文件: `web/src/components/chat/`

### G3: 错误处理 + 边界（P1）

**AL-T5: Error Boundary**
- React ErrorBoundary 组件 → 友好错误页面
- WS 错误 → Toast 通知
- API 错误 → 统一 Toast 处理
- 文件: `web/src/components/ErrorBoundary.tsx`, `web/src/hooks/useApi.ts`

### G4: MCP + Memory 增强（P3）

**AL-T6: MCP Workbench 增强**
- 服务器日志实时流（SSE 或 WS 订阅）
- 工具测试面板（选择工具 → 填参数 → 执行 → 看结果）
- 文件: `web/src/pages/McpWorkbench.tsx`, `web/src/components/mcp/`

**AL-T7: Memory Explorer 增强（合并 AG-D8）**
- 时间线视图（按时间排列记忆条目）
- Session 过滤器（下拉选择 session → 只看该 session 的记忆）
- 文件: `web/src/pages/Memory.tsx`

### 执行顺序

```
G3 (T5) ← 先做 error boundary，后续开发受益
   ↓
G1 (T1→T2) ← 核心功能
   ↓
G2 (T3,T4) ← 可并行
   ↓
G4 (T6,T7) ← 增强，可选
```

**预估**: 7-10 天

### Deferred

| ID | 描述 | 前置条件 | 状态 |
|----|------|---------|------|
| AL-D1 | KG 可视化（力导向图） | AG-D6 KG 语义搜索 | ⏳ |
| AL-D2 | 深色/浅色主题切换 | 设计 token 完善 | ⏳ |
| AL-D3 | 移动端响应式适配 | 核心页面稳定 | ⏳ |
| AL-D4 | 国际化 (i18n) | 产品需求 | ⏳ |

---

## Phase AM — 可观测性 + 运维工具

> **目标**: 生产环境可监控、可排查
> **前置**: AK（安全加固）

### G1: 指标导出（P1）

**AM-T1: Prometheus 端点**
- `/api/v1/metrics/prometheus` 输出标准 Prometheus text format
- 核心指标:
  - `octo_active_sessions` (gauge)
  - `octo_request_duration_seconds` (histogram)
  - `octo_tool_invocations_total` (counter)
  - `octo_llm_tokens_used_total` (counter, label: model)
  - `octo_ws_connections_active` (gauge)
- 文件: `api/metrics.rs`

**AM-T2: 结构化日志**
- 生产模式: JSON 格式日志（`tracing_subscriber::fmt::json()`）
- 每个请求注入 `request_id` (UUID) → 全链路追踪
- 配置: `logging.format: "json" | "pretty"`（默认 pretty）
- 文件: `main.rs`, `config.rs`

### G2: 监控面板 API（P2）

**AM-T3: Session 监控端点**
- `GET /api/v1/sessions/metrics` → 活跃数、idle 分布、总创建数、平均存活时间
- 文件: `api/sessions.rs`

**AM-T4: Event Bus session 过滤（合并 AJ-D6）**
- `GET /api/v1/events/stream?session_id=xxx` → SSE 按 session 过滤
- 文件: `api/events.rs`

### G3: 崩溃恢复（P2，合并 AJ-D2）

**AM-T5: Session 状态序列化**
- Session metadata 持久化到 SQLite（session_id, user_id, agent_id, created_at, status）
- `AgentRuntime::save_session_state()` / `restore_sessions()`
- 启动时检测未正常关闭的 session → 恢复或标记为 crashed
- 文件: `runtime.rs`, `main.rs`
- 注: 不恢复 conversation history（已在 SessionStore），只恢复 registry 元数据

**AM-T6: Provider Chain 监控**
- `GET /api/v1/providers/status` → 各 provider 健康状态、延迟 P50/P99、failover 计数
- 文件: `api/providers.rs`

### 执行顺序

```
G1 (T1,T2) ← 可并行，基础设施
   ↓
G2 (T3,T4) ← 可并行
   ↓
G3 (T5,T6) ← 较复杂
```

**预估**: 7-10 天

### Deferred

| ID | 描述 | 前置条件 | 状态 |
|----|------|---------|------|
| AM-D1 | Grafana Dashboard 模板 | AM-T1 Prometheus 端点 | ✅ 已补 |
| AM-D2 | 告警规则（session 数异常、错误率飙升） | AM-T1 + 运维环境 | ✅ 已补 |
| AM-D3 | 分布式追踪（OpenTelemetry） | 多实例部署需求 | ⏳ |

---

## Deferred 归并总表

| 原 ID | 归入 Phase/Task | 状态 |
|--------|----------------|------|
| AJ-D2 崩溃恢复 | AM-T5 | 📋 |
| AJ-D3 优雅关闭 | AK-T6 | 📋 |
| AJ-D5 前端多会话 UI | AL-T1 | 📋 |
| AJ-D6 Event Bus 过滤 | AM-T4 | 📋 |
| AG-D8 Memory 前端增强 | AL-T7 | 📋 |
| AJ-D1 IPC 健康检查 | 不归入（平台版需求） | ⏳ |
| AJ-D7 KG scope 字段 | 不归入（需产品需求驱动） | ⏳ |
| AG-D2 语义合并 | 不归入（需充足数据验证） | ⏳ |
| AG-D6 KG 语义搜索 | 不归入（需稳定 embedding） | ⏳ |
| AD-D1~D6 容器增强 | 不归入（独立基础设施工作） | ⏳ |

---

## 总时间线

```
Phase AK (5-7d)  →  Phase AL + AM (7-10d, 并行)
                          ↓
                    Phase AN (2-3w, 独立规划)
```

**Phase AK → AL/AM 总计约 3 周**，覆盖 octo-server 从"可用"到"生产就绪"的关键路径。

---

## 验收标准

1. **AK 完成后**: `make verify-api` 全部通过，安全头存在，CORS 严格模式可用，优雅关闭工作
2. **AL 完成后**: Web 端可创建/切换/关闭 session，WS 断连有视觉反馈，无白屏崩溃
3. **AM 完成后**: Prometheus 端点可用，JSON 日志可接入 ELK，session 监控 API 可查

## Baseline

- **Tests**: 2476+ (from Phase AJ + D4)
- **Commit**: bfe795f (HEAD of main)
- **DB Version**: 12
