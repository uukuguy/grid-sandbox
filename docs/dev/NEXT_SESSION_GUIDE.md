# octo-sandbox 下一会话指南

**最后更新**: 2026-04-01 19:30 GMT+8
**当前分支**: `main`
**当前状态**: Phase AO 完成，无活跃 Phase

---

## 项目状态

### 已完成 Phases
- Phase A-H: Core Engine + Eval 基础
- Phase I-R: 外部 Benchmark + 标准测试 + 评估
- Phase S: Agent Capability Boost
- Phase T: TUI OpenDev 整合 (24 tasks)
- Phase U: TUI Production Hardening (10 tasks)
- Phase V: Agent Skills (11 tasks)
- Phase W-Z: OctoRoot + TUI + Playbook + Landmine
- Phase AA-AF: 部署配置 + 沙箱容器 + Workspace + SSM
- Phase AG: 记忆和上下文机制增强 (11 tasks + 5 deferred)
- Phase AH: Hook 系统增强 (15 tasks + 3 deferred)
- Phase AI: WASM Component Model Hook 插件 (11 tasks + 4 deferred)
- Phase AJ: 多会话复用 (13 tasks + D4 resolved)
- Phase AK: Server 安全加固 (7 tasks)
- Phase AL: Web 前端完善 (7 tasks)
- Phase AM: 可观测性 (6 tasks)
- **Phase AO: octo-server 功能完善 (10 tasks + 2 stubs) @ 39159aa**

### 最新提交
```
39159aa feat(server): resolve 2 NOT_IMPLEMENTED stubs
a660366 feat(server): Phase AO Wave 3 — Config Update + Audit + Context
9b3075b feat(server): Phase AO Wave 2 — Hooks + Security + Secrets + Sandbox
757ddc8 feat(server): Phase AO Wave 1 — Metering API + Knowledge Graph API
```

### 测试基线
- 2476+ tests passing（建议跑全量确认）
- DB Version: 13
- 新增 36 个 E2E 测试覆盖 Phase AO API

---

## 下一步优先级

1. **全量测试验证** — `cargo test --workspace -- --test-threads=1` 确认无回归
2. **Deferred 清理** — 跨 Phase 暂缓项（见下方列表）
3. **Phase AN** — octo-platform-server 多租户平台（独立产品线）
4. **前端集成** — 基于 AO 新增 API 扩展 web/ 功能（KG 可视化等）

---

## ⚠️ Deferred 未清项

### Phase AO
| ID | 内容 | 前置条件 |
|----|------|---------|
| AO-D1 | WebSocket 订阅 metering 实时流 | 需前端配合 |
| AO-D2 | KG 图算法扩展（PageRank、社区检测） | 需产品场景明确 |
| AO-D3 | Hook 在线编辑（YAML 在线修改） | 需设计审批流 |
| AO-D4 | Secret rotation（自动轮换） | 需与 AK-D3 合并 |

### 历史遗留
| Phase | ID | 内容 |
|-------|-----|------|
| AK | D1 | Rate limiter 精细化 |
| AK | D3 | API Key rotation |
| AL | D1-D4 | KG 可视化、主题、响应式、i18n |
| AM | D3 | OpenTelemetry 集成 |

---

## 关键代码路径

- **Server API**: `crates/octo-server/src/api/` (28 模块)
- **Engine Core**: `crates/octo-engine/src/` (21 模块)
- **Frontend**: `web/src/` (React + Jotai)
- **测试**: `crates/octo-server/tests/` (force-add, .gitignore)

## 备注

- octo-server 零 NOT_IMPLEMENTED stub
- RuntimeConfigOverrides 模式用于运行时配置和安全策略覆盖
- Phase AO 计划文档: `docs/plans/2026-04-01-phase-ao-server-completeness.md`
