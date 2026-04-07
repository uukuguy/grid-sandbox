# Grid Platform 下一会话指南

**最后更新**: 2026-04-07 05:30 GMT+8
**当前分支**: `Grid`
**当前状态**: Phase BG 设计完成 — 准备实施

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
- [x] Phase BE — EAASP 协议层 + claude-code-runtime (6/6, 93 tests)
- [x] Phase BF — L2 统一资产层 + L1 抽象机制 (7/7, 30 new tests)
- [ ] **Phase BG** — Enterprise SDK 基石 (0/6, 设计已完成)

## Phase BG 概览

**目标**: 构建 EAASP Enterprise SDK 的基石层，让企业开发者可以创作、校验、推演 Skill

**设计蓝图**: `docs/design/Grid/EAASP_SDK_DESIGN.md`
**实施计划**: `docs/plans/2026-04-07-phase-bg-enterprise-sdk.md`

| Wave | 内容 | 状态 |
|------|------|------|
| W1 | specs/ JSON Schema + Python Pydantic 模型 | ⏳ pending |
| W2 | authoring 创作工具链 (parser + validator + scaffold + hook) | ⏳ pending |
| W3 | sandbox 核心 + GridCliSandbox | ⏳ pending |
| W4 | RuntimeSandbox + MultiRuntimeSandbox (gRPC + 对比) | ⏳ pending |
| W5 | CLI + submit + HR 入职示例 | ⏳ pending |
| W6 | 文档收尾 + Makefile + ROADMAP 更新 | ⏳ pending |

## SDK 长期演进路线

| 阶段 | Phase | 内容 | 状态 |
|------|-------|------|------|
| S1: 基石 | BG | specs + models + authoring + sandbox + CLI | **当前** |
| S2: 推演增强 | BG-D/BH | GridServerSandbox + test 报告 | 后续 |
| S3: 治理 | BH | Policy DSL + L3 对接 | 后续 |
| S4: 编排 | BH/BI | Playbook DSL + 事件触发 | 后续 |
| S5: 客户端 | BI | 5 REST API + PlatformSandbox | 后续 |
| S6: TypeScript | BI/BJ | TS SDK | 后续 |
| S7: 生态 | BJ+ | MCP Tool + Java/Go | 后续 |

## 关键代码路径

| 组件 | 路径 |
|------|------|
| SDK 设计蓝图 | `docs/design/Grid/EAASP_SDK_DESIGN.md` |
| BG 实施计划 | `docs/plans/2026-04-07-phase-bg-enterprise-sdk.md` |
| SDK 源码（待创建） | `sdk/python/src/eaasp/` |
| JSON Schema（待创建） | `sdk/specs/` |
| 示例 Skill（待创建） | `sdk/examples/` |
| Proto 定义 | `proto/eaasp/runtime/v1/runtime.proto` |
| L2 Skill Registry | `tools/eaasp-skill-registry/` |
| grid-runtime gRPC | `crates/grid-runtime/src/service.rs` |
| claude-code-runtime | `lang/claude-code-runtime-python/` |
| certifier blindbox | `tools/eaasp-certifier/src/blindbox.rs` |

## 建议下一步

1. 执行 W1: 创建 `sdk/specs/` + `sdk/python/` 项目骨架 + Pydantic 模型
2. 依次推进 W2-W5
3. W6 收尾后提交
