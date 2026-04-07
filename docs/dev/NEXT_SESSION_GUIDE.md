# Grid Platform 下一会话指南

**最后更新**: 2026-04-07 23:30 GMT+8
**当前分支**: `main`
**当前状态**: Phase BH-MVP 完成 — EAASP v1.8 架构蓝图已产出

---

## 完成清单

- [x] Phase A-Z — Core Engine + Eval + TUI + Skills
- [x] Phase AA-AF — Sandbox/Config/Workspace architecture
- [x] Phase AG-AI — Memory/Hooks/WASM enhancement
- [x] Phase AJ-AO — 多会话/安全/前端/服务器
- [x] Phase AP-AV — 追赶 CC-OSS + 安全对齐
- [x] Phase AW-AY — 工具/Agent/SubAgent 体系
- [x] Phase AZ — Cleanup/Transcript/Completion
- [x] Phase BA — Octo to Grid 重命名 + TUI 完善
- [x] Phase BB-BC — TUI 视觉升级 + Deferred 补齐
- [x] Phase BD — grid-runtime EAASP L1 (6/6, 37 tests)
- [x] Phase BE — EAASP 协议层 + claude-code-runtime (6/6, 93 tests)
- [x] Phase BF — L2 统一资产层 + L1 抽象机制 (7/7, 30 tests)
- [x] Phase BG — Enterprise SDK 基石 (6/6, 107 tests)
- [x] Phase BH-MVP — E2E 全流程验证 (7/7+D3/D5/D10, 71 tests)
- [ ] **v1.8 Phase 2** — 事件引擎 + 事件室基础

## Phase BH-MVP 成果总结

### 新增组件

| 组件 | 位置 | 端口 | 语言 |
|------|------|------|------|
| L3 Governance Service | `tools/eaasp-governance/` | :8083 | Python FastAPI |
| L4 Session Manager | `tools/eaasp-session-manager/` | :8084 | Python FastAPI |
| SDK `eaasp run` | `sdk/python/src/eaasp/cli/run_cmd.py` | CLI | Python Click |
| PlatformClient | `sdk/python/src/eaasp/client/platform_client.py` | — | Python httpx |
| E2E Tests | `tests/e2e/` | — | Python pytest |
| HR Example (完善) | `sdk/examples/hr-onboarding/` | — | YAML/Python |

### 测试统计

| 组件 | 测试数 | 运行命令 |
|------|--------|---------|
| L3 governance | 33 | `cd tools/eaasp-governance && .venv/bin/python -m pytest tests/ -xvs` |
| L4 session-manager | 10 | `cd tools/eaasp-session-manager && .venv/bin/python -m pytest tests/ -xvs` |
| SDK run_cmd | 8 | `cd sdk/python && .venv/bin/python -m pytest tests/test_run_cmd.py -xvs` |
| E2E | 20 | `tools/eaasp-governance/.venv/bin/python -m pytest tests/e2e/ -xvs` |
| **总计** | **71** | |

### Makefile 新增目标

```bash
make l3-setup / l3-start / l3-test
make l4-setup / l4-start / l4-test
make e2e-setup / e2e-run / e2e-test / e2e-teardown / e2e-full
```

## EAASP v1.8 架构蓝图

**设计文档**: `docs/design/Grid/EAASP_ARCHITECTURE_v1.8.md`

### 五层架构

```
L5  协作层  Cowork Layer         人与 Agent 的协作空间
L4  编排层  Orchestration Layer  事件驱动 + 会话编排 + A2A 协调
L3  治理层  Governance Layer     策略 + 审批 + 审计 + 校核
L2  资产层  Asset Layer          Skill + MCP + Memory Engine
L1  执行层  Execution Layer      Agent Runtime
```

### 核心升级（vs v1.7）

1. **L5 协作层新增** — 事件室 + 四卡置顶（事件卡/证据包/行动卡/审批卡）
2. **L4 从会话管理→事件编排** — 事件引擎 + A2A 路由 + 状态机
3. **L2 新增 Memory Engine** — 证据锚点库 + 文件化记忆 + 混合检索索引
4. **三纵向机制** — Hook管线 + 数据流管线 + 会话控制管线

### v1.8 实施路径

```
Phase 1 ✅ 已完成（BH-MVP）— 基础管道验证
Phase 2 → 事件引擎 + 事件室数据模型（下一步）
Phase 3 → Memory Engine + 证据索引
Phase 4 → 审批闸门 + 确定性校核
Phase 5 → A2A 并行互审
Phase 6 → 完整四卡 + IM 集成
```

## 下一步优先级

1. **设计 v1.8 Phase 2 实施计划** — 事件引擎（接入→聚合→事件对象→状态机）+ 事件室 API
2. 决策：事件引擎放在 L4 现有 `eaasp-session-manager` 还是独立新服务
3. 决策：事件源接入方式（Webhook vs Kafka vs 内嵌模拟）

## 关键代码路径

| 组件 | 路径 |
|------|------|
| L3 Governance | `tools/eaasp-governance/src/eaasp_governance/` |
| L4 Session Manager | `tools/eaasp-session-manager/src/eaasp_session/` |
| L1 Grid Runtime | `crates/grid-runtime/` |
| L1 Claude Code Runtime | `lang/claude-code-runtime-python/` |
| L2 Skill Registry | `tools/eaasp-skill-registry/` |
| SDK | `sdk/python/src/eaasp/` |
| Proto | `proto/eaasp/` |
| E2E Tests | `tests/e2e/` |

## ⚠️ Deferred 未清项（下次 session 启动时必查）

> 以下暂缓项来自 Phase BH-MVP，前置条件尚未满足。

| 来源 | ID | 内容 | 前置条件 |
|-----|----|------|---------|
| BH-MVP | BH-D1 | RBAC 访问控制 | 用户身份管理 |
| BH-MVP | BH-D2 | 审批闸门 | L4 审批 UI (v1.8 Phase 4) |
| BH-MVP | BH-D4 | MCP 注册中心 | L2 MCP 扩展 |
| BH-MVP | BH-D6 | L4 管理控制台 UI | Web 框架 (v1.8 Phase 6) |
| BH-MVP | BH-D7 | L4 事件总线 | 消息队列 (v1.8 Phase 2) |
| BH-MVP | BH-D8 | L4 可观测性枢纽 | Grafana/Prometheus |
| BH-MVP | BH-D9 | 多租户 | PostgreSQL |
| BH-MVP | BH-D11 | 成本治理 | Cost Ledger |
| BH-MVP | BH-D12 | T2/T3 HookBridge 验证 | 非 T1 运行时 |
