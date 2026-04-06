# Grid Platform 下一会话指南

**最后更新**: 2026-04-06 22:00 GMT+8
**当前分支**: `Grid`
**当前状态**: Phase BF 启动 — L2 统一资产层 + L1 抽象机制

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
- [x] Phase BD — grid-runtime EAASP L1 (6/6, 37 tests @ ae4b337)
- [x] Phase BE W1-W3 — 协议层 + HookBridge + certifier (3/3, 54 tests @ 40a231e)
- [x] Phase BE W4-W6 — claude-code-runtime Python T1 Harness (3/3, 39 Python tests)
- [ ] **Phase BF** — L2 统一资产层 + L1 抽象机制 (0/7)

## Phase BF 当前进度

**状态**: 设计完成，实施计划已写入

**计划文档**: `docs/plans/2026-04-06-phase-bf-l2-asset-layer.md`

| Wave | 内容 | 状态 |
|------|------|------|
| W1 | 协议扩展（SessionPayload L2 字段） | ⏳ |
| W2 | L2 Skill Registry crate (REST + SQLite + Git) | ⏳ |
| W3 | L2 MCP Orchestrator crate (YAML + subprocess) | ⏳ |
| W4 | L1 Runtime L2 集成 (GridHarness → L2 REST) | ⏳ |
| W5 | Mock L3 RuntimeSelector + 运行时池 | ⏳ |
| W6 | 盲盒对比 (并行执行 + 匿名评分) | ⏳ |
| W7 | 集成验证 + 设计文档 + Makefile | ⏳ |

## 关键设计决策 (BF Brainstorming 产出)

| # | 决策 | 理由 |
|---|------|------|
| BF-KD1 | L2 存储：SQLite 元数据 + 文件系统 + Git 追溯 | 三层各司其职 |
| BF-KD2 | L1 ↔ L2 Skill 通信：REST（L1 拉取内容） | Agent 不直连 L2 |
| BF-KD3 | L2 三个独立服务（Skill/MCP/Ontology） | 资产类型本质不同 |
| BF-KD4 | RuntimeSelector 属于 L3，BF 在 certifier mock | L3 未来 Python/TS |
| BF-KD5 | 盲盒：用户主动开启，并行执行，匿名评分 | 实验性功能 |
| BF-KD6 | L2 Skill Registry = REST only（去掉 MCP 接口） | Agent 不直连 L2 |
| BF-KD7 | MCP Orchestrator 管理 MCP Server 运行 | L3 查询后下发给 L1 |
| BF-KD8 | L3 下发 skill_ids + skill_registry_url | L1 自行从 L2 拉取 |
| BF-KD9 | Agent 不需要 skill_search | L3 策略筛选子集 |

## 关键代码路径

| 组件 | 路径 |
|------|------|
| SessionPayload (proto) | `proto/eaasp/runtime/v1/runtime.proto` |
| SessionPayload (Rust) | `crates/grid-runtime/src/contract.rs` |
| GridHarness | `crates/grid-runtime/src/harness.rs` |
| gRPC service | `crates/grid-runtime/src/service.rs` |
| certifier CLI | `tools/eaasp-certifier/src/main.rs` |
| certifier mock L3 | `tools/eaasp-certifier/src/mock_l3.rs` |
| certifier verifier | `tools/eaasp-certifier/src/verifier.rs` |
| common.proto | `proto/eaasp/common/v1/common.proto` |
| hook.proto | `proto/eaasp/hook/v1/hook.proto` |
| HookBridge trait | `crates/grid-hook-bridge/src/traits.rs` |
| Python runtime | `lang/claude-code-runtime-python/` |

## 新增组件（BF 产出物）

| 组件 | 路径 | 说明 |
|------|------|------|
| Skill Registry | `tools/eaasp-skill-registry/` | L2 Skill 仓库 REST API |
| MCP Orchestrator | `tools/eaasp-mcp-orchestrator/` | L2 MCP Server 管理 |
| L2 Client | `crates/grid-runtime/src/l2_client.rs` | L1 从 L2 拉取 Skill |
| RuntimePool | `tools/eaasp-certifier/src/runtime_pool.rs` | 运行时池管理 |
| RuntimeSelector | `tools/eaasp-certifier/src/selector.rs` | Mock L3 选择策略 |
| Blindbox | `tools/eaasp-certifier/src/blindbox.rs` | 盲盒对比 |

## 建议下一步

1. 执行实施计划 W1-W7（推荐 subagent-driven 模式）
2. 或 `/dev-phase-manager:resume-plan` 如果已有 checkpoint
