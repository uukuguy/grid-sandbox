# Grid — Roadmap

> **Milestone:** v3.1 Phase 5 — Engine Hardening (grid-cli + grid-server)
> **Brownfield context:** Second GSD-managed milestone after Phase 4 (v3.0) closed 2026-04-28 with ADR-V2-024 Accepted (双轴模型 supersedes ADR-V2-023). Phase 4 (4.0/4.1/4.2) 历史保留只读, this roadmap covers ONLY milestone v3.1 (Phase 5.0 → 5.5, 6 phases).
> **Granularity:** standard (6 phases — 在 5-8 区间内偏低端, 匹配 Phase 4 单 plan/phase 节奏 + watchlist-spread 策略避免单独 watchlist phase 阻塞主线)。
> **Done condition for milestone:** 6 phases 全 ✅; 23 REQ-ID 全 ✅ traceability; ADR-V2-025 (runtime tier strategy) + ADR-V2-026 (engine vs data/integration boundary) Accepted; ADR-V2-019 (L1 deployment) 翻 Accepted; contract-v1.2.0 升级 + 主力档 7 runtime 全 PASS; watchlist 8 项 (D109 / D120 / D134 / D136 / D142 / D143 / NEW-D2 / NEW-E2 / NEW-E3) 全部 closed-or-resolved; PROJECT.md / STATE.md milestone close cascade 完成。

## Milestones

- ✅ **v3.0 Phase 4 — Product Scope Decision** — Phases 4.0/4.1/4.2 (shipped 2026-04-28, ADR-V2-024 Accepted)
- 🚧 **v3.1 Phase 5 — Engine Hardening (grid-cli + grid-server)** — Phases 5.0/5.1/5.2/5.3/5.4/5.5 (in progress)

## Phases

<details>
<summary>✅ v3.0 Phase 4 — Product Scope Decision (Phases 4.0/4.1/4.2) — SHIPPED 2026-04-28</summary>

3 phases shipped under previous milestone. See git log + ADR-V2-024 + previous ROADMAP.md (commit before 2026-04-29 milestone-restart) for details. All 10 REQ-IDs (CLEANUP-01..04 / DECIDE-01..03 / GOVERNANCE-01..03) traceability ✅.

- [x] **Phase 4.0: Bootstrap & Cleanup** — Complete 2026-04-27 (5/5 SC, 7/7 must-haves)
- [x] **Phase 4.1: Discuss & Audit** — Complete 2026-04-27 (14/15 must-haves, GOVERNANCE-03 deferred to 4.2)
- [x] **Phase 4.2: Decide & Document** — Complete 2026-04-28 (5/5 SC, ADR-V2-024 Accepted, milestone closed)

</details>

### 🚧 v3.1 Phase 5 — Engine Hardening (grid-cli + grid-server) (In Progress)

**Milestone Goal:** 在 ADR-V2-024 双轴模型下推进 engine 接入面 (grid-cli + grid-server 优先发力组合) 的硬化, 同时把 cross-milestone watchlist P1 项分散到相关 phase 顺手解决, 并定义 data/integration 横切层接入面 ADR-only 规约。

- [ ] **Phase 5.0: Hook Envelope Baseline** — D120 (HookContext schema 补全) + D134 (shipped skill hooks key-path 改正), engine 侧 hook envelope baseline 健康度补丁解锁 5.3/5.4
- [x] **Phase 5.1: Runtime Tier ADR + Contract Test Parametrization** — ADR-V2-025 Accepted (主力/样板/参考/冻结四档执行强度 × 7 runtime) + test_chunk_type_contract.py 7-runtime 参数化 (✅ 2026-05-02)
- [ ] **Phase 5.2: CLI Hardening** — `grid` 命令树 / streaming output / error+exit code / TUI 拆分 / session lifecycle / `grid doctor` 6 项 grid-cli 硬化
- [ ] **Phase 5.3: Contract Evolution** — ChunkType + Hook event 扩展 (升级 contract-v1.2.0 主力档强制), 顺带 D109 + D136 收尾
- [ ] **Phase 5.4: Server Hardening** — WebSocket / L1 gRPC 集成 / session+L2 持久化 / auth+audit / config hot-reload 5 项 grid-server 硬化, 顺带 D142+D143 + ADR-V2-019 → Accepted
- [ ] **Phase 5.5: Interface ADR + Milestone Close** — ADR-V2-026 候选 (engine vs data/integration boundary contract) + NEW-E2 advisory sweep + milestone close cascade

## Phase Details

### Phase 5.0: Hook Envelope Baseline

**Goal**: Hook envelope baseline (ADR-V2-006 §2-§3) 健康度补丁 — Rust HookContext schema 补齐至与 Python 一致 (D120), shipped skill hooks 的 nested key path 改正为 ADR-V2-006 §2.3 top-level (D134); 这是 Phase 5 后续 CONTRACT (5.3) 与 SERVER (5.4) phase 改 hook fire 点 / 跨 runtime parity test 的前置基线。
**Depends on**: Nothing (milestone 第一个 phase, 但需 Phase 4.2 milestone close 已经完成的 ADR-V2-024 双轴 framework 上下文)
**Requirements**: WATCH-00, WATCH-02
**Success Criteria** (what must be TRUE):
  1. `crates/grid-engine/src/hooks/context.rs` (or 等价路径) `HookContext::to_json` / `to_env_vars` 输出包含 `event` + `skill_id` 字段, 进程 env 包含 `GRID_EVENT` + `GRID_SKILL_ID`, 且 `tests/contract/test_hook_envelope_parity.rs` (或 `hook_envelope_parity_test.rs`) 跨 Rust ↔ Python 两侧 envelope shape 比对全 PASS (无 missing-field skip)
  2. `examples/skills/*/hooks/*.sh` 7 个 example skill hook 脚本中所有读 `.payload.output.*` 的位置改为读 ADR-V2-006 §2.3 top-level `.output.*`; grep `.payload.output` 在 `examples/skills/` 路径下 0 hit (lock-in regression test)
  3. D120 + D134 在 `docs/design/EAASP/DEFERRED_LEDGER.md` 标 ✅ CLOSED 并附 commit hash, ledger row-edit-on-close convention 遵守 (per Phase 4.0 CLEANUP-02 precedent)
  4. Phase 5.0 后跑 `make verify-runtime` (或 7 runtime contract test 子集 covering HookContext envelope 字段) 不引入新 xfail; D136 (probe turn 不 fire hook 的 3 contract xfails) 保持原状, 留给 Phase 5.3 处理
**Plans**: 1 plan
- [x] 05.0-01-PLAN.md — Hook envelope baseline: fix GA1 dead code, confirm D120 parity, close D134
**UI hint**: no

### Phase 5.1: Runtime Tier ADR + Contract Test Parametrization

**Goal**: 在 ADR-V2-017 三轨产品策略 (主力 / 样板 / 对比) 基础上增加契约执行强度差异化策略 — 起草 ADR-V2-025 (`type: strategy`), 评估 7 runtime 现状 + 划分主力档 / 样板档 / 参考档 / 冻结档, 同时把 `tests/contract/cases/test_chunk_type_contract.py` 从现 3 tests 扩展为 7-runtime 参数化, 与 tier review 同步用 (NEW-D2)。
**Depends on**: Phase 5.0 (hook envelope baseline 健康对 contract test 主力档 PASS 判定有影响, 不让 D120 missing field 污染 tier 划分依据)
**Requirements**: CONTRACT-00, WATCH-05
**Success Criteria** (what must be TRUE):
  1. `docs/design/EAASP/adrs/ADR-V2-025-l1-runtime-contract-tier-strategy.md` 状态 Accepted (F1-F4 lint exit 0); Decision 段包含 4 档执行强度表格 (主力 / 样板 / 参考 / 冻结) × 7 runtime (grid / claude-code / nanobot / pydantic-ai / goose / claw-code / ccb) 显式分配 + hermes (frozen)
  2. ADR-V2-025 §References 引用 ADR-V2-017 (产品三轨) + ADR-V2-021 (chunk_type contract) + ADR-V2-024 (双轴 framework) + Phase 4a project review 中 NEW-D2 议题
  3. `tests/contract/cases/test_chunk_type_contract.py` 改为 pytest parametrize 跑 7 runtime; CI workflow `.github/workflows/phase3-contract.yml` (或 phase5 后续) 7-runtime matrix run 全部按 ADR-V2-025 tier 设定 (主力档 must PASS / 样板档 PASS-or-xfail / 参考档 v1.1 baseline / 冻结档 skip) 行为正确
  4. NEW-D2 在 DEFERRED_LEDGER 标 ✅ CLOSED 并附 commit hash; `pytest tests/contract/cases/test_chunk_type_contract.py -v` 输出 7 runtime case 数 ≥ 21 (3 cases × 7 runtime)
**Plans**: 1 plan
- [x] 05.1-01-PLAN.md — Runtime Tier ADR + Contract Test Parametrization
**UI hint**: no

### Phase 5.2: CLI Hardening

**Goal**: `grid-cli` (binary `grid`) 用户面硬化 — 命令树梳理 / streaming output UX / 错误信息+exit code 一致性 / TUI key_handler.rs 拆分 / session lifecycle 命令打磨 / `grid doctor` 检查清单扩展 6 项, 让用户敲 `grid --help` 与 `grid cli-ask "..."` 体验对齐 Phase 4 后预期成熟度。
**Depends on**: Phase 5.0 (hook envelope baseline; CLI session lifecycle 命令需要 hook fire 路径稳定; 不阻塞 5.1, 但概念上 CLI hardening 在主力档 contract test 健康后做更稳)
**Requirements**: CLI-01, CLI-02, CLI-03, CLI-04, CLI-05, CLI-06
**Success Criteria** (what must be TRUE):
  1. 用户运行 `grid --help` 看到统一分类的 subcommand 树 (cli-ask / cli-session / cli-config / cli-doctor / studio-tui / studio-dashboard 命名一致, help 输出按 group 分类), 并且 6 个 subcommand 各自的 `grid <cmd> --help` 都包含 Examples 段 + exit code 说明
  2. 用户运行 `grid cli-ask "hello world"` 看到 ChunkType 流式渲染 (TEXT_DELTA 打字机效果 + TOOL_START 折叠 + TOOL_RESULT 摘要 + ERROR 高亮); 跨 ChunkType (含 WORKFLOW_CONTINUATION) 渲染无丢字 / 错排
  3. CLI exit code 在 4 类典型故障场景 (provider 错误 / network timeout / config 缺失 / API key invalid) 输出一致格式错误信息且 exit code 符合 sysexits.h (EX_USAGE=64 / EX_NOINPUT=66 / EX_UNAVAILABLE=69 / EX_SOFTWARE=70 / EX_CONFIG=78); `cargo test -p grid-cli` 包含至少 4 个 exit-code 断言测试
  4. `crates/grid-cli/src/tui/key_handler.rs` 拆分为 dispatcher + per-mode handler 子模块 (NEW-C2); 文件行数从拆分前 P1 阈值 (~500+ LOC) 降到合理区间 (<300 LOC per file); `cargo test -p grid-cli` 新增的 mode-handler 单元测试 ≥ 6 个 PASS
  5. 用户运行 `grid session list / resume / kill` 端到端可用 — 列表显示 SQLite session record 关键字段, resume 能 turn-by-turn 恢复, kill 清理孤儿 record; `grid doctor` 输出包含 EAASP_DEPLOYMENT_MODE / GRID_HOOKS_FILE / hook bundle 健康度 / L2 memory engine 可达性 / L1 runtime gRPC 可达性 5 个新增检查项
**Plans**: TBD by `/gsd-plan-phase 5.2` (推测 1-2 plans, CLI-01..03 + CLI-04..06 二分组, 取决于 plan-phase 评估)
**UI hint**: yes

### Phase 5.3: Contract Evolution

**Goal**: L1 runtime contract 中等扩展 — ChunkType 新增 1-2 个 enum 值 (CONTRACT-01) + Hook event 新增 1-2 个 event (CONTRACT-02), 走完 proto + codegen + 主力档 runtime 实现 + L4 mapper + CLI whitelist + cross-runtime parity test 全链路, 升级到 contract-v1.2.0 (主力档强制 / 样板+参考档按 ADR-V2-025 策略); 顺带 D109 (workflow.required_tools 不变量文档化) + D136 (grid-runtime probe turn 不 fire hook 的 3 contract xfails 修正) 收尾。
**Depends on**: Phase 5.1 (ADR-V2-025 tier strategy 必须先 Accepted, 用以判定 contract-v1.2.0 升级时哪些 runtime 必须 PASS / 哪些可降级)
**Requirements**: CONTRACT-01, CONTRACT-02, WATCH-01, WATCH-03
**Success Criteria** (what must be TRUE):
  1. `proto/eaasp/runtime/v2/runtime.proto` ChunkType 枚举新增 1-2 个 wire 值 (例 `THINKING_TRACE` / `ATTACHMENT_REF`, 由 plan-phase 决定具体语义), `proto/eaasp/runtime/v2/hook.proto` Hook event oneof 新增 1-2 个 event (例 `SubagentStart` / `TaskCheckpoint`); proto codegen 在 7 runtime 全部 regen 且 `cargo build` + `make build-eaasp-all` 全 PASS; ADR-V2-021 + ADR-V2-006 增量更新 commit + 新 ADR (V2-XXX) covering hook event 扩展 Accepted
  2. 主力档 (grid-runtime) 与样板档 (claude-code / nanobot / pydantic-ai) 4 runtime 在 contract-v1.2.0 全 PASS; 参考档 (goose / claw-code / ccb) 按 tier 策略允许 v1.1 baseline 或 selective xfail; `make v2-phase3-e2e` (或 phase5 后续) 输出反映 tier 设定结果
  3. CLI `_ALLOWED_CHUNK_TYPES` whitelist + L4 `_chunk_type_to_wire` mapper + `tests/contract/cases/test_chunk_type_contract.py` 三处全部覆盖新增 ChunkType wire 值; new hook event fire site 在主力档 runtime 至少有一个端到端 trigger test (mock LLM, 不打 live LLM)
  4. D109 (workflow.required_tools 不变量) 在 `docs/design/EAASP/L1_RUNTIME_ADAPTATION_GUIDE.md` (or 等价 spec doc) 显式文档化; D136 (grid-runtime hook 在 probe turn 不触发) 修正 — 原 3 contract xfails 改为 PASS, ADR-V2-016 capability matrix probe turn 章节注明协同; 两项 D109 + D136 在 DEFERRED_LEDGER 标 ✅ CLOSED 并附 commit hash
**Plans**: TBD by `/gsd-plan-phase 5.3` (推测 2-3 plans, CONTRACT-01 + CONTRACT-02 各一 + watchlist 顺手 plan)
**UI hint**: no

### Phase 5.4: Server Hardening

**Goal**: `grid-server` (Axum 0.8 + WebSocket + tokio-rusqlite) 单用户 workbench 服务端硬化 — WebSocket 流式 / L1 runtime gRPC 集成 / session+L2 内存持久化 / auth+audit 路径 / config hot-reload 5 项, 让 grid-server 在 `:3001` 端口端到端服务用户能撑住 ChunkType 流式 + 多 turn session + L2 内存读写 + HMAC auth; 顺带 D142+D143 (EAASP_DEPLOYMENT_MODE 接入 grid-runtime + claude-code-runtime) + ADR-V2-019 → Accepted (NEW-E3, blocks on D142+D143 关闭后顺接)。
**Depends on**: Phase 5.3 (contract-v1.2.0 + hook event 扩展 必须先就位, server WebSocket 流式 + session 持久化 hook 写 trajectory 到 L2 才有正确的 ChunkType+event 集合可序列化)
**Requirements**: SERVER-01, SERVER-02, SERVER-03, SERVER-04, SERVER-05, WATCH-04, WATCH-07
**Success Criteria** (what must be TRUE):
  1. WebSocket endpoint `ws://127.0.0.1:3001/v1/sessions/{id}/stream` 支持 ChunkType stream 端到端流式输出 + backpressure (1000-msg load test 0 dropped chunks) + reconnect (client reconnect 续传 from last chunk_id) + message ordering (顺序保持); 集成测试 `tests/integration/test_websocket_stream.rs` (或等价路径) ≥ 5 case PASS
  2. `grid-server` (`:3001`) 调 `grid-runtime` (`:50051`) 16-method RuntimeService gRPC 端到端跑通 — Initialize + SendResponse + Terminate 三大主路径 + chunk relay + hook envelope 透传, `make verify-dual-runtime` 在 grid-server 启动时 pass; session id 在 server-side state 与 runtime-side state 一致
  3. SQLite (`data/grid.db`) schema 新增 / 演进 covering session record + turn record + L2 memory FTS5+HNSW+time-decay 索引 (与 `tools/eaasp-l2-memory-engine` schema 兼容); Stop hook fire 时 server 写 trajectory 到 L2 memory engine 的端到端单元测试 ≥ 2 case PASS; tokio-rusqlite migration 跑通且幂等
  4. `GRID_AUTH_MODE` / `GRID_API_KEY` / `GRID_API_KEY_USER` / HMAC (ADR-003) 端到端验证: 4 种 auth 模式各有 ≥ 1 个集成测试覆盖 + audit log 写入 + rate limit 基础生效 (per-key 简单计数即可); config hot-reload 在 4 类 hot-reloadable 字段上 (GRID_HOOKS_FILE / GRID_POLICIES_FILE / GRID_LOG / GRID_CORS_ORIGINS) 不重启生效, GRID_HOST / GRID_PORT 列入 require-restart 白名单且 hot-reload 触发时报清晰错误
  5. D142 + D143 closed (`grid-runtime` + `claude-code-runtime` 都读 EAASP_DEPLOYMENT_MODE + 实施 max_sessions=1 gate per ADR-V2-019); NEW-E3 closed (ADR-V2-019 status flip Proposed → Accepted, F1-F4 lint exit 0); 三项 D142 / D143 / NEW-E3 在 DEFERRED_LEDGER + ADR audit doc 标 ✅ CLOSED 并附 commit hash
**Plans**: TBD by `/gsd-plan-phase 5.4` (推测 2-3 plans, SERVER-01..03 主路径一组 + SERVER-04+05+watchlist 一组)
**UI hint**: yes

### Phase 5.5: Interface ADR + Milestone Close

**Goal**: 起草 ADR-V2-026 候选 (`type: strategy 或 contract 待定`) 描述 engine (user 60%+30% 工时) 与 data/integration 横切层 (他人 10%) 的 boundary contract — customer data ingestion / SSO / third-party API gateway / EAASP+Grid 双产品 boundary 在代码层的 enforcement (crate boundaries + proto package separation); ADR-only, **不写 trait / proto skeleton** (per 用户 Phase 5 决策, 留给 v3.2+ INTERFACE-02/03); 顺带 NEW-E2 (F3 ADR enforcement.trace 29 missing items, advisory) sweep, 然后跑 milestone close cascade (PROJECT.md §Active flip + ROADMAP 状态 + STATE.md frontmatter)。
**Depends on**: Phase 5.4 (ADR-V2-019 已 Accepted, NEW-E3 已 closed, 让 NEW-E2 sweep 时 ADR audit baseline 干净; 同时 server hardening 已落地, ADR-V2-026 boundary contract 可参考 grid-server runtime 实证 boundary 而非纸面臆测)
**Requirements**: INTERFACE-01, WATCH-06
**Success Criteria** (what must be TRUE):
  1. `docs/design/EAASP/adrs/ADR-V2-026-engine-data-integration-boundary.md` 状态 Accepted (F1-F4 lint exit 0); Decision 段描述双轴 (engine vs data/integration) 在代码层的 boundary — engine = `crates/grid-*` + `tools/eaasp-l[2-4]-*` + `proto/eaasp/runtime/v2/`, data/integration = customer-specific adapters / SSO / third-party API / WORM 存储 / 信创 LLM 适配 等横切层 + 各自 hook-out 接入面 (但具体 trait / proto skeleton **不在本 ADR 范围**)
  2. ADR-V2-026 §References 引用 ADR-V2-024 §1 双轴模型 + ADR-V2-023 P1 shared core 规则 (retained under V2-024) + Phase 4.1 audit doc 5+3 字段切分; ADR-V2-026 标记 future ADR 候选 V2-XXX (代码层 enforcement implementation) + V2-YYY (Rust trait + gRPC service skeleton) 待 v3.2+
  3. NEW-E2 sweep 完成 — 29 missing `enforcement.trace` items 在相关 ADR 中按 advisory level 补全或显式标记 "trace=[] is intentional for strategic ADRs"; F3 lint 输出从 29 missing 降至 ≤ 5 (剩余项明示 strategic / strategic-with-rationale); NEW-E2 在 DEFERRED_LEDGER 标 ✅ CLOSED 并附 commit hash
  4. Milestone close cascade ✓ — PROJECT.md §Active "Phase 5 milestone (v3.1)" 行划掉移入 §Validated 引用 ADR-V2-025+026 commit hash; ROADMAP.md 全 6 phase Status=Complete 含完成日期; STATE.md frontmatter `status: milestone-complete` + progress 6/6=100%; debt water-line 无新增 P0/P1-active 项
  5. 全 milestone 23 REQ-ID traceability ✅ (所有 6 phase 的 SC 在自身 phase verify-phase 阶段过, 且 Phase 5.5 close 时 ROADMAP.md Coverage 表无 ❌); 8 watchlist 项 (D109 / D120 / D134 / D136 / D142 / D143 / NEW-D2 / NEW-E2 / NEW-E3 — 实际 9 项含 NEW-E3, requirements WATCHLIST 数为 8 是因 NEW-D2 + NEW-E2 + NEW-E3 算 3 项 advisory) 全 closed-or-resolved 在 DEFERRED_LEDGER 中
**Plans**: TBD by `/gsd-plan-phase 5.5` (推测 1 plan, INTERFACE-01 ADR 起草 + NEW-E2 sweep + milestone close 并入)
**UI hint**: no

## Phase 之外的 milestone 关闭后续

> 这些不是本 milestone 的 phase, 只作为 traceability 提示。

- **下一个 milestone (v3.2 候选)** 由 `/gsd-new-milestone` 启动, 内容由 Phase 5 milestone close 时累积的 Future Requirements 决定, 候选方向:
  - **CONTRACT-03**: 新 RPC method (Probe / Capabilities / MemorySync) — defer 到 v3.2+
  - **CONTRACT-04**: SubAgent first-class 协议 — defer 至 second consumer 出现
  - **INTERFACE-02**: Rust trait + gRPC service skeleton 实现 (在 ADR-V2-026 boundary 锁定后)
  - **INTERFACE-03**: EAASP / Grid 双产品 boundary 代码层 enforcement (in-place)
  - **NEW-C1 / C3**: harness.rs / grid-eval 大文件拆分 (defer until second consumer per Phase 4a project review; CLI-04 NEW-C2 在 Phase 5.2 已处理)
  - **D-batch (~40 P3 / housekeeping items 跨 D8..D80)** — 单日 batch sweep window 待安排
  - **EAASP 与 Grid 分仓** — 时点由 v3.2+ milestone 决定
  - **grid-platform / grid-desktop / web* 增量功能** — dormant per ADR-V2-024 双轴 framework, 激活由 next milestone audit 决定
- **不属于本 milestone 但仍需追踪的项**: 见 PROJECT.md §Out of Scope; `grid-sandbox` 仓库改名 / `git push origin main` push 时机 / Phase 0–2.5 历史 sign_off_commit retrofit / 132 历史 plan + 14 archived phase 迁入 GSD ROADMAP / F4 lint 52 module-overlap 警告 reconcile / WORK_LOG.md vs STATE.md / DEFERRED_LEDGER 迁入 GSD backlog 全部 acknowledged 不动。

## Progress

**Execution Order:**
Phases execute in numeric order: 5.0 → 5.1 → 5.2 → 5.3 → 5.4 → 5.5

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 5.0 Hook Envelope Baseline | 0/1 | Not started | - |
| 5.1 Runtime Tier ADR + Contract Test Parametrization | 0/1 | Not started | - |
| 5.2 CLI Hardening | 0/TBD | Not started | - |
| 5.3 Contract Evolution | 0/TBD | Not started | - |
| 5.4 Server Hardening | 0/TBD | Not started | - |
| 5.5 Interface ADR + Milestone Close | 0/TBD | Not started | - |

## Coverage

| REQ-ID | Phase | Notes |
|--------|-------|-------|
| CLI-01 | 5.2 | grid 命令树整理 + help 输出 |
| CLI-02 | 5.2 | streaming output ChunkType 渲染 UX |
| CLI-03 | 5.2 | error message + sysexits.h exit code 一致性 |
| CLI-04 | 5.2 | TUI key_handler.rs 拆分 (NEW-C2) |
| CLI-05 | 5.2 | session lifecycle list/resume/kill 端到端 |
| CLI-06 | 5.2 | grid doctor 检查清单扩展 (5 新增项) |
| SERVER-01 | 5.4 | WebSocket ChunkType stream + backpressure + reconnect |
| SERVER-02 | 5.4 | L1 runtime gRPC 16-method 端到端集成 |
| SERVER-03 | 5.4 | session+L2 内存持久化 + Stop hook 写 trajectory |
| SERVER-04 | 5.4 | auth (HMAC ADR-003) + audit log + rate limit 基础 |
| SERVER-05 | 5.4 | config hot-reload + require-restart 字段白名单 |
| CONTRACT-00 | 5.1 | ADR-V2-025 候选 — runtime tier strategy (主力/样板/参考/冻结) |
| CONTRACT-01 | 5.3 | ChunkType 1-2 新 enum 值, contract-v1.2.0 升级 |
| CONTRACT-02 | 5.3 | Hook event 1-2 新 event, ADR (V2-XXX) 候选 |
| WATCH-00 | 5.0 | D120 — Rust HookContext schema 补全 (D134 前置) |
| WATCH-01 | 5.3 | D109 — workflow.required_tools 不变量文档化 (CONTRACT phase 顺手) |
| WATCH-02 | 5.0 | D134 — shipped skill hooks .payload.output.X → .output.X must-fix |
| WATCH-03 | 5.3 | D136 — grid-runtime probe turn hook 不触发 (3 contract xfails) 修正 |
| WATCH-04 | 5.4 | D142 + D143 — EAASP_DEPLOYMENT_MODE 接入 + max_sessions=1 gate |
| WATCH-05 | 5.1 | NEW-D2 — test_chunk_type_contract.py 7-runtime 参数化 |
| WATCH-06 | 5.5 | NEW-E2 — F3 ADR enforcement.trace 29 missing items advisory sweep |
| WATCH-07 | 5.4 | NEW-E3 — ADR-V2-019 Proposed → Accepted (D142+D143 关闭后顺接) |
| INTERFACE-01 | 5.5 | ADR-V2-026 候选 — engine vs data/integration boundary contract (ADR-only) |

**Total v3.1 requirements:** 23 (CLI 6 + SERVER 5 + CONTRACT 3 + WATCHLIST 8 + INTERFACE 1)
**Mapped:** 23/23 ✓
**Orphans:** 0
**Double-mapped:** 0

> 注: REQUIREMENTS.md §Total 标 22 是计数 typo (CLI 6 + SERVER 5 + CONTRACT 3 + WATCHLIST 8 + INTERFACE 1 = 23), Traceability 表实际 23 行, 此处以 23 为准。

## Granularity 备注

本 milestone 选 6 phase (在 standard 设定 5-8 区间内偏低端) 是**有意为之**:
- ADR-V2-024 Open Item #3 优先发力组合 grid-cli + grid-server 是本 milestone 主线, 自然落两个独立 phase (5.2 CLI + 5.4 SERVER); 其他 4 phase (5.0 baseline / 5.1 tier ADR / 5.3 contract evolution / 5.5 interface ADR) 都是为主线提供 prerequisite or close-out
- watchlist-spread 策略 (per 用户 Phase 5 决策) 把 8 watchlist 项分散到 5 个 phase (5.0/5.1/5.3/5.4/5.5) 而非单独 watchlist phase, 避免单独 phase 阻塞主线; 顺手解决降低工时分散风险
- Phase 5.0 单独成 phase 是因 D120 (HookContext schema) 是 D134 修正的 hard prerequisite + Phase 5.3/5.4 的 hook fire 路径前置, 不可与 CLI/SERVER 主线并行 (会撞 baseline 不一致)
- Phase 5.1 单独成 phase 是因 ADR-V2-025 tier strategy 必须在 Phase 5.3 contract-v1.2.0 升级前 Accepted (用以判定哪些 runtime 必须 PASS / 哪些可降级); 与 NEW-D2 7-runtime 参数化同一 plan 写作工时复用
- Phase 5.5 单独成 phase 是因 INTERFACE-01 是 ADR-only deliverable + milestone close cascade 工作量都是文档+脚本性质, 不混入 SERVER hardening (避免 Phase 5.4 边界模糊)

如 plan-phase 阶段发现某个 phase task 多于 5 个 plan, 可由 plan-phase 自行考虑微拆 (例 Phase 5.2 CLI 6 项可能拆 2 plan, Phase 5.4 SERVER 5+watchlist 7 项可能拆 2-3 plan), 但 ROADMAP 阶段不预拆。

---

*Roadmap created 2026-04-29 by `/gsd-roadmapper` (Step 10 of `/gsd-new-milestone` v3.1). Source: REQUIREMENTS.md (commit `ed864fb`) + PROJECT.md §Current Milestone v3.1 + ADR-V2-024 §1 双轴 framework + Phase 4 milestone close decisions. Phase 5.0 ready to enter `/gsd-plan-phase 5.0`.*
