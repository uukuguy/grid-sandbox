# Grid — Requirements

> **Brownfield context**: 14 archived phases (Phase BA → Phase 4a) under dev-phase-manager already shipped EAASP v2.0 functional baseline. Phase 4 milestone v3.0 (3 phases: 4.0 / 4.1 / 4.2) closed 2026-04-28 with ADR-V2-024 Accepted (双轴模型 supersedes ADR-V2-023). This REQUIREMENTS.md scopes **milestone v3.1 — Phase 5 Engine Hardening (grid-cli + grid-server)**.

---

## v3.1 Requirements (Milestone: Phase 5 — Engine Hardening)

> **Per ADR-V2-024 §1 双轴模型 + Open Item #3**: 优先发力组合 grid-cli + grid-server; 其余 (grid-platform / grid-desktop / web*) 保持 dormant. 工时 baseline: Grid 全栈 ≈60% / EAASP 引擎 ≈30% / 元工作 ≈10%.

### A. CLI — grid-cli 硬化

- [ ] **CLI-01**: `grid` 命令树整理 — 统一 subcommand 命名 / help 输出 / exit code, 现有 `cli` / `cli-ask` / `cli-session` / `cli-config` / `cli-doctor` / `studio-tui` / `studio-dashboard` UX 一致性提升, 用户敲 `grid --help` 能看到清晰的命令分类。
- [ ] **CLI-02**: Streaming output 改善 — `grid cli-ask` 渲染 ChunkType (TEXT_DELTA / TOOL_START / TOOL_RESULT / WORKFLOW_CONTINUATION / 等) 的 UX 优化, 流式打字机效果 + tool call 折叠 + error 高亮。
- [ ] **CLI-03**: Error message + exit code 一致性 — provider 错误 / network timeout / config 缺失 / API key invalid 等场景的错误信息有统一格式, exit code 符合 sysexits.h 约定 (EX_USAGE=64 / EX_NOINPUT=66 / EX_UNAVAILABLE=69 / EX_SOFTWARE=70 / EX_CONFIG=78)。
- [ ] **CLI-04**: TUI key_handler.rs 拆分 — Phase 4a NEW-C2 deferred 项, key_handler.rs 单文件过大, 拆分为 dispatcher + per-mode handler 子模块以便后续扩展 (Phase 5 提优先级到本 milestone)。
- [ ] **CLI-05**: Session lifecycle 命令端到端打磨 — `grid session list / resume / kill` 的 UX 验证, 包括 SQLite session record 显示 / 恢复 turn-by-turn / 清理孤儿 session record。
- [ ] **CLI-06**: `grid doctor` 检查清单扩展 — 添加 EAASP_DEPLOYMENT_MODE / GRID_HOOKS_FILE / hook bundle 健康度 / L2 memory engine 可达性 / L1 runtime gRPC 可达性 等检查项。

### B. SERVER — grid-server 硬化

- [ ] **SERVER-01**: WebSocket 流式渲染成熟度 — Axum 0.8 + axum-extra 0.10 WebSocket 端点对 ChunkType stream 的端到端流式输出, 含 backpressure / reconnect / message ordering 验证。
- [ ] **SERVER-02**: L1 runtime gRPC 集成端到端验证 — grid-server (`:3001`) 调 grid-runtime (`:50051`) 16-method RuntimeService 全部 RPC 端到端跑通 (Initialize / SendResponse / Terminate / etc), 含 session id 传递 / chunk relay / hook envelope 透传。
- [ ] **SERVER-03**: Session 持久化 + L2 内存集成 — `data/grid.db` SQLite + tokio-rusqlite schema 演进, session record + turn record + L2 memory FTS5+HNSW+time-decay 索引, Stop hook fire 时把 trajectory 写入 L2 memory engine。
- [ ] **SERVER-04**: Auth 路径打磨 — GRID_AUTH_MODE / GRID_API_KEY / GRID_API_KEY_USER / HMAC (ADR-003) 端到端验证 + audit log + rate limit 基础。
- [ ] **SERVER-05**: Config hot-reload — config.yaml 改动 + GRID_* env vars 在 server 运行时优雅 reload (排除 GRID_HOST / GRID_PORT 必须重启的字段), 含 hot-reloadable vs require-restart 字段白名单。

### C. CONTRACT — L1 runtime contract 中等扩展

- [ ] **CONTRACT-00**: 7 runtime 分级 review + 契约执行强度策略 ADR — 新 ADR 候选 `ADR-V2-025-l1-runtime-contract-tier-strategy.md` (type: strategy), 评估 7 runtime 现状, 划分 **主力档** (强制 v1.2 全 PASS) / **样板档** (鼓励但允许 xfail) / **参考档** (可降级到 v1.1 baseline) / **冻结档** (免审, 现 hermes-runtime); ADR-V2-017 三轨产品策略基础上增加契约执行强度差异化策略。
- [ ] **CONTRACT-01**: ChunkType 扩展点 (1-2 新 enum 值) — 评估并落地新 ChunkType (例如 `THINKING_TRACE` / `ATTACHMENT_REF`), 走完 proto 改动 + codegen + 主力档 runtime 实现 + L4 mapper + CLI whitelist + contract test + ADR-V2-021 增量更新流程; 升级到 contract-v1.2.0 (主力档强制, 样板/参考档按 ADR-V2-025 策略)。
- [ ] **CONTRACT-02**: Hook event 扩展 (1-2 新 event) — 例如 `SubagentStart` / `TaskCheckpoint` (对接 Phase 4.1 §F.Q3 audit ⚫ 6 项接入位 ADR 候选之一), 走完 proto 改动 + envelope schema + 主力档 runtime fire site + L3 governance trigger + cross-runtime parity test + 新 ADR (V2-026 候选)。

### D. WATCHLIST — 分散到相关 phase 顺手解决 (spread strategy)

- [ ] **WATCH-00**: D120 Rust HookContext schema 补全 — 现 `HookContext::to_json/to_env_vars` 缺 `event` / `skill_id` 字段 + 缺 `GRID_EVENT` / `GRID_SKILL_ID` env, ADR-V2-006 §2-§3 envelope shape 不完整。Phase 5 早期 phase 必修 (D134 修的前置)。
- [ ] **WATCH-01**: D109 — workflow.required_tools 不变量文档化 (CONTRACT phase 顺手)。
- [ ] **WATCH-02**: D134 — Shipped skill hooks `.payload.output.X` → `.output.X` 改正 (per ADR-V2-006 §2.3 top-level), 修 7 个 example skill hook 脚本; 锁定 must-fix per 用户 Phase 5 决策 (D120 修完后顺接)。
- [ ] **WATCH-03**: D136 — grid-runtime hook 在 probe turn 不触发 (3 contract xfails) 修正 (CONTRACT phase 顺手, 跟 ADR-V2-016 capability matrix probe turn 协同)。
- [ ] **WATCH-04**: D142 + D143 — grid-runtime + claude-code-runtime EAASP_DEPLOYMENT_MODE 接入 + max_sessions=1 gate (~20 LOC each, SERVER phase 顺手)。
- [ ] **WATCH-05**: NEW-D2 — test_chunk_type_contract.py 7-runtime 参数化 (现 仅 3 tests, CONTRACT phase 顺手, 与 CONTRACT-00 runtime 分级 review 同步用)。
- [ ] **WATCH-06**: NEW-E2 — F3 ADR enforcement.trace 29 missing items 补 (advisory, 任一 phase 顺手)。
- [ ] **WATCH-07**: NEW-E3 — ADR-V2-019 Proposed → Accepted (blocks on D142+D143; closes after WATCH-04, SERVER phase 收尾)。

### E. INTERFACE — Data/integration 横切层接入面规约 (ADR-only)

- [ ] **INTERFACE-01**: data/integration boundary contract ADR 起草 — 新 ADR 候选 `ADR-V2-026-engine-data-integration-boundary.md` (type: strategy / contract 待定), 描述 engine (user 60%+30%) 与 data/integration 横切层 (他人 10%) 之间的 boundary contract: customer data ingestion endpoints / SSO contracts / third-party API gateway 接入面 / EAASP / Grid 双产品 boundary 在代码层的 enforcement (crate boundaries + proto package separation); ADR-only, **不写 trait / proto skeleton** (per 用户 Phase 5 决策, 留给 next milestone)。

---

## Future Requirements (deferred to v3.2+)

- **CONTRACT-03**: 新 RPC method (Probe / Capabilities / MemorySync) — defer to next milestone, premature 风险高
- **CONTRACT-04**: SubAgent first-class 协议 — defer 至 second consumer 出现
- **INTERFACE-02**: Rust trait + gRPC service skeleton — defer 待 INTERFACE-01 ADR 锁定 boundary 后
- **INTERFACE-03**: EAASP / Grid 双产品 boundary 代码层 enforcement — 与 INTERFACE-01 部分重叠, 由 INTERFACE-01 一处 anchor 起步
- **NEW-C1 / C3**: harness.rs / grid-eval 大文件拆分 — defer until second consumer (per Phase 4a project review)
- **D-batch (~40 P3 / housekeeping items 跨 D8..D80)** — defer batch sweep window
- **EAASP 与 Grid 分仓** — 分仓时点由后续 milestone 决定, Phase 5 不动
- **grid-platform / grid-desktop / web* 增量功能开发** — dormant per ADR-V2-024 双轴 framework

---

## Out of Scope

- **`grid-sandbox` 仓库改名** — per ADR-V2-023 §P6, Grid 独立产品 (原 Leg B, see ADR-V2-024) 激活前不动
- **`git push origin main` 累积 push 控制** — Phase 4.2 期间已 push, Phase 5 期间继续累积, push 时机由人决策
- **Phase 0–2.5 历史 sign_off_commit retrofit** — 历史不完美接受, git history 为准
- **132 个历史 plan + 14 archived phase 迁入 GSD ROADMAP.md** — 冻结只读历史
- **F4 lint 52 module-overlap 警告 reconcile** — advisory-only 接受
- **`docs/dev/WORK_LOG.md` 替换为 STATE.md** — 二者并存 (GSD 例外)
- **DEFERRED_LEDGER 迁入 GSD backlog** — ledger SSOT 保留 (GSD 例外)
- **新 RPC method 与 SubAgent 协议** — defer 到 v3.2+ (CONTRACT-03/04 已 defer)
- **Data/integration 真实现** — Phase 5 仅 ADR (INTERFACE-01), trait/skeleton/code 全部 defer

---

## Traceability

> Filled by `/gsd-roadmapper` after Step 10 (✅ filled 2026-04-29 via `/gsd-roadmapper`). 每条 REQ-ID 1-to-1 映射到 ROADMAP.md `Phase Details` 中一个 phase。

| REQ-ID | Phase | Notes |
|--------|-------|-------|
| CLI-01 | 5.2 | |
| CLI-02 | 5.2 | |
| CLI-03 | 5.2 | |
| CLI-04 | 5.2 | |
| CLI-05 | 5.2 | |
| CLI-06 | 5.2 | |
| SERVER-01 | 5.4 | |
| SERVER-02 | 5.4 | |
| SERVER-03 | 5.4 | |
| SERVER-04 | 5.4 | |
| SERVER-05 | 5.4 | |
| CONTRACT-00 | 5.1 | (ADR-V2-025 候选) |
| CONTRACT-01 | 5.3 | (ChunkType 扩展, contract-v1.2.0 升级) |
| CONTRACT-02 | 5.3 | (Hook event 扩展, ADR-V2-026 候选) |
| WATCH-00 | 5.0 | (D120 — Phase 5 早期 phase 必修, D134 前置) |
| WATCH-01 | 5.3 | (D109) |
| WATCH-02 | 5.0 | (D134 — must fix, D120 修完后顺接) |
| WATCH-03 | 5.3 | (D136) |
| WATCH-04 | 5.4 | (D142 + D143) |
| WATCH-05 | 5.1 | (NEW-D2) |
| WATCH-06 | 5.5 | (NEW-E2) |
| WATCH-07 | 5.4 | (NEW-E3 — D142/D143 关闭后顺接) |
| INTERFACE-01 | 5.5 | (ADR-only, 新 ADR-V2-026 待定 ID) |

**Total v3.1 requirements:** 23 REQ-IDs (CLI 6 + SERVER 5 + CONTRACT 3 + WATCHLIST 8 + INTERFACE 1)
**Granularity:** 6 phases (Phase 5.0 → 5.5)
**Mapping density:** ~3.8 REQ/phase (23 REQ / 6 phase, 在 GSD standard 3-5 plans/phase 范围)
**Watchlist strategy:** spread (8 watchlist items 分散到相关 phase 顺手解决)

---

*Requirements 来源: Phase 4 milestone close + ADR-V2-024 §1 双轴 framework + Open Item #2/#3 工时 baseline + 优先发力组合 + 用户 Phase 5 决策 (CONTRACT 中等选项 + 7 runtime 分级 review + D134 must fix + INTERFACE ADR-only). Defined 2026-04-29 via `/gsd-new-milestone` Step 9 conversation-mode (no research).*
