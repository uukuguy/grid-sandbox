# octo-sandbox 下一会话指南

**最后更新**: 2026-03-04 GMT+8
**当前分支**: `octo-workbench`
**当前状态**: 🔄 v1.0 Release Sprint — Phase A（稳定地基）待启动

---

## 当前阶段进度

| 阶段 | 状态 | 说明 |
|------|------|------|
| Phase 1 核心引擎 | ✅ 完成 | 32 Rust + 16 TS 文件，E2E 验证通过 |
| Phase 2.1–2.11 | ✅ 完成 | 全部子阶段完成（含 AgentRegistry + AgentRuntime 重构） |
| v1.0 Release Sprint Phase A | 🔄 待启动 | A1-A5 待执行，A6 已确认完成 |
| v1.0 Release Sprint Phase B | ⏳ 未开始 | 依赖 Phase A |
| v1.0 Release Sprint Phase C | ⏳ 未开始 | 前端控制台 |
| v1.0 Release Sprint Phase D | ⏳ 未开始 | 集成验收 |

---

## v1.0 Release Sprint 任务清单

**计划文档**: `docs/plans/2026-03-04-v1.0-release-sprint-plan.md`

### Phase A — 稳定地基（~3天）

| Task | 内容 | 状态 |
|------|------|------|
| A1 | 修复 stop_primary（drop tx 语义） | ⏳ |
| A2 | ToolRegistry 版本化共享引用（MCP 热插拔） | ⏳ |
| A3 | 修复 Scheduler run_now 真实执行 | ⏳ |
| A4 | WorkingMemory per-session 隔离 | ⏳ |
| A5 | 优雅关机（MCP shutdown_all） | ⏳ |
| A6 | 确认 Provider 重试已实现 | ✅ 已完成 |

### Phase B — 后端能力（~4天）

| Task | 内容 | 状态 |
|------|------|------|
| B1 | 并行工具执行（enable_parallel 生效） | ⏳ |
| B2 | 后台任务 API（POST/GET /api/tasks） | ⏳ |
| B3 | 增强 /health 端点 | ⏳ |
| B4 | 补发 LoopTurnStarted 事件（修复 turns.total 指标） | ⏳ |
| B5 | JSON 日志格式支持 | ⏳ |
| B6 | 移除 Option<McpManager>（清理噪声） | ⏳ |

### Phase C — 前端控制台（~5天）

| Task | 内容 | 状态 |
|------|------|------|
| C1 | TabBar 扩展（添加新页面标签） | ⏳ |
| C2 | Tasks 页面 | ⏳ |
| C3 | Schedule 页面 | ⏳ |
| C4 | Tools 页面（MCP + Built-in + Skills） | ⏳ |
| C5 | Memory 页面 | ⏳ |
| C6 | Debug 页面 | ⏳ |
| C7 | Chat 页面完善 | ⏳ |

### Phase D — 集成验收（~2天）

| Task | 内容 | 状态 |
|------|------|------|
| D1 | 端到端测试脚本 | ⏳ |
| D2 | Docker Compose 一键启动 | ⏳ |
| D3 | 配置文档完善 | ⏳ |
| D4 | 发布 Checklist 验证 | ⏳ |

---

## 关键代码路径（Phase A 相关）

| 组件 | 路径 | Phase A 关联 |
|------|------|-------------|
| AgentRuntime | `crates/octo-engine/src/agent/runtime.rs` | A1 stop_primary, A2 ToolRegistry |
| AgentExecutor | `crates/octo-engine/src/agent/executor.rs` | A1 cancel 语义 |
| Scheduler | `crates/octo-engine/src/scheduler/mod.rs` | A3 run_now |
| WorkingMemory | `crates/octo-engine/src/memory/working.rs` | A4 per-session 隔离 |
| AppState | `crates/octo-server/src/state.rs` | A1 handle 类型 |
| main.rs | `crates/octo-server/src/main.rs` | A5 shutdown_all |

---

## 快速启动命令

```bash
# 构建验证
cargo check --workspace
cd web && npx tsc --noEmit && cd ..

# 运行测试
cargo test -p octo-engine

# 启动开发服务器
make dev
```

---

## 下一步操作

```bash
# 直接开始执行 Phase A
/executing-plans
# 或用子代理并行
/subagent-driven-development
```

---

## 重要记忆引用

| claude-mem ID | 内容 |
|---------------|------|
| #3044 | AgentRuntime 核心架构完整说明 |
| #3045 | AgentEntry / AgentManifest 数据模型 |
| #3043 | AgentExecutor 架构与生命周期 |
