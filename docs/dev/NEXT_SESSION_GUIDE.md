# octo-sandbox 下一会话指南

**最后更新**: 2026-03-10 10:00 GMT+8
**当前分支**: `main`
**当前状态**: Octo-CLI 重新设计 — 计划已就绪，待执行

---

## 当前活跃阶段：Octo-CLI 设计与实现

### 计划文档

- **实施方案**: `docs/plans/2026-03-10-octo-cli-redesign.md` (34 tasks, 5 phases)
- **设计文档**: `docs/design/AGENT_CLI_DESIGN.md` (1,694 行，6 部分)
- **配色方案**: `docs/design/color_comparison.html` (12 种内置主题)
- **Checkpoint**: `docs/plans/.checkpoint.json` (status: READY)

### 技术决策

| 决策 | 选择 | 依据 |
|------|------|------|
| REPL 库 | rustyline v17 | IronClaw + ZeroClaw 验证 |
| TUI 框架 | Ratatui 0.29 | OpenFang fork |
| Web Dashboard | Deferred (Alpine.js) | Phase 4 完成后启动 |
| 配色方案 | 12 种内置主题 | 冷/暖/渐变/无色系全覆盖 |

### 需新增的 Engine API

| API | 位置 | 说明 |
|-----|------|------|
| `AgentRuntime::send_message_streaming()` | `agent/runtime.rs` | CLI 高层消息 API |
| `AgentRuntime::create_session_and_start()` | `agent/runtime.rs` | Session 创建+Agent 启动一步完成 |
| `SessionStore::delete_session()` | `session/mod.rs` | Session 删除 |
| `SessionStore::most_recent_session()` | `session/mod.rs` | `--continue` 恢复最近会话 |

### Phase 概览

| Phase | 任务 | 内容 | 状态 |
|-------|------|------|------|
| Phase 1 | R1-R8 | CLI 核心基础设施 | pending |
| Phase 2 | R9-R14 | REPL 交互模式 | pending |
| Phase 3 | R15-R20 | 管理子命令补全 | pending |
| Phase 4 | T1-T8 | TUI 全屏模式 | pending |
| Phase 5 | A1-A6 | 高级功能 | pending |

### 下一步行动

1. **开始 Phase 1** — 从 R1(命令结构) + R2(输出系统) + R3(UI 组件) + R5(Engine 接口) 并行开始
2. 执行命令: `/dev-phase-manager:resume-plan` 进入执行模式

---

## 已完成阶段

| 阶段 | 任务 | 测试 | Commit | 说明 |
|------|------|------|--------|------|
| pre-harness-refactor | 42/42 + 5 Deferred | 857 | 3117721 | 基础重构 |
| harness-implementation | 28/28 | 872 | 9ada808 | Agent Harness 核心 |
| harness-skills-completion | 34/34 | 904 | 71dc7fc | 类型统一、Skills集成、安全审批、Pipeline |
| octo-platform Phase 1 | P1+P2 | — | — | 多租户基础 |

---

## 挂起阶段

| 阶段 | 进度 | 说明 |
|------|------|------|
| Phase 2.11: AgentRegistry | 0% | 切换到 octo-platform 时挂起 |

---

## 设计文档索引

| 文档 | 状态 |
|------|------|
| `docs/design/AGENT_CLI_DESIGN.md` | 参考文档（6 部分，1,694 行） |
| `docs/design/AGENT_HARNESS_BEST_IMPLEMENTATION_DESIGN.md` | 已同步到 Phase 3 |
| `docs/design/AGENT_SKILLS_BEST_IMPLEMENTATION_DESIGN.md` | 已同步（7.5/10） |
| `docs/design/color_comparison.html` | 12 种配色方案预览 |
| `docs/plans/2026-03-10-octo-cli-redesign.md` | 实施方案（34 tasks, 5 phases） |

---

## 快速命令

```bash
cargo check --workspace
cargo test --workspace -- --test-threads=1
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
open docs/design/color_comparison.html    # 查看配色方案
```
