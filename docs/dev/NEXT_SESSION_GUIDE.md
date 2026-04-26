# Grid Platform 下一会话指南

> ⚠️ **FROZEN 2026-04-26 — 项目转向 GSD 体系。**
>
> 此文件保留为 **EAASP v2.0 Phase 2 → 4a 的会话指南快照**(dev-phase-manager 时代,2026-04-14 至 2026-04-20)。
>
> **新会话上下文请读** `.planning/STATE.md`(由 `/gsd-new-project` 生成后)。
>
> 历史 phase 计划仍在 `docs/plans/2026-*-plan.md`(132 个,只读保留);
> 历史 phase stack 仍在 `docs/dev/.phase_stack.json`(14 archived,不再更新);
> WORK_LOG.md 继续 prepend 新条目(GSD 不接管这个);
> DEFERRED_LEDGER 继续作为跨阶段 D-item SSOT(GSD backlog 不取代它,见 PROJECT.md 例外)。
>
> 切换原因:GSD workstreams + resume-work + plan-checker + map-codebase 在多 workstream / brownfield 场景比 dev-phase-manager 更适合。

---

**最后更新**: 2026-04-20 GMT+8
**当前分支**: `main`（ahead `origin/main` by ~293 commits — Phase 3.5 遗留 + Phase 3.6 + Phase 4a）
**当前状态**: EAASP v2.0 **Phase 4a 🟢 COMPLETE 7/7** @ `8629505` (sign-off 2026-04-20) → **Phase 4 待规划(由 GSD 接管)**

## 本会话重点

**Phase 4a — Pre-Phase-4 Debt Cleanup 完成**：7 项跨阶段 Deferred 全部关闭（D148/D149/D151/D152/D153/D154/D155）。5 commits landed this session 顶部（T5 + T5-fix + T6 + T6-fix + T7），T1-T4 在 prior session。Debt 水位归零，Phase 4 起点干净。详见 `docs/dev/WORK_LOG.md` 顶部条目。

## 项目进度

- [x] Phase 2 — Memory and Evidence（23/23 @ `f4bf9ad`）
- [x] Phase 2.5 — Container + MCP Pool（25/25 @ `844664d`）
- [x] Phase 3 — L1 Runtime Functional Completeness（35/35 @ `8ee05fe`）
- [x] Phase 3.5 — chunk_type Unification（19/19 @ `5b13898`）
- [x] Phase 3.6 — Tech-debt Cleanup（5/5 @ `b81f455`）
- [x] **Phase 4a — Pre-Phase-4 Debt Cleanup（7/7 @ `8629505`）← 本次**
- [ ] Phase 4 — TBD（参考 ADR-V2-023 §P5 Leg A vs Leg B 决策 + `docs/plans/` backlog）

## 下一步优先级

1. **推 `origin/main`（人决定时机）**—— 累积 ~293 unpushed commits（Phase 3.5 遗留 275 + Phase 3.6 新增 11 + Phase 4a 新增 7 + end-phase docs commit 1）。先 `git log origin/main..main --oneline | wc -l` 确认数，再 `git push origin main`。Phase 4a 明确列为 out-of-scope 保留人工决策。
2. **Phase 4 产品范围讨论**（前置 ADR-V2-023）：
   - **Leg A（EAASP 集成）continuation**：`grid-runtime` 作为 EAASP L1 flagship 继续硬化 —— 可能方向：multi-tenant isolation、performance tuning、skill catalog 扩展。
   - **Leg B（Grid 独立产品）activation**：判断 ADR-V2-023 §P5 触发条件是否满足（当前 dormant crate `grid-platform` / `grid-server` / `grid-desktop` / `web*`）。
   - 选择后 → `/dev-phase-manager:start-phase "Phase 4 - <topic>"` + `/gsd-discuss` 做 Socratic ideation。
3. **可选清扫**（新 session 低优先）：
   - `pydantic_ai.OpenAIModel → OpenAIChatModel` upstream rename（10 DeprecationWarnings 在 pydantic-ai 测试中，源头 `provider.py:39`）。
   - claude-code-runtime `test_default_config` 预存 fail（`acceptEdits→bypassPermissions` drift from commit 6784994，MEMORY.md 已追踪）。
   - 上游 `protocolbuffers/protobuf#25319` 如果 merge，`_loosen_enum_stubs` 变 no-op 可删。

## 关键代码路径

- **Phase 4a 新增 scripts**:
  - `scripts/check-ccb-types-ts-sync.sh` — ccb TS 枚举同步 guard（name + wire-int）。
  - `scripts/check-pyright-prereqs.sh` — 9-venv 预检。
  - `scripts/gen_runtime_proto.py:_loosen_enum_stubs` — proto3 enum union post-process。
  - `scripts/gen_runtime_proto.py:--out-dir` — Dockerfile layout override flag。
- **Phase 4a 新增 CI**:
  - `.github/workflows/phase4a-ccb-types-sync.yml` — ccb 枚举 sync CI gate（轻量 bash workflow，~1s）。
- **Phase 4a 新增 tests**:
  - `crates/grid-engine/tests/harness_envelope_wiring_test.rs` — spy HookHandler/StopHook 回归（D151）。
  - `lang/pydantic-ai-runtime-python/tests/test_provider.py` + `test_session.py` — 18 new tests（D148）。
- **Proto stubs 已 widened**: 24 处 `_Union[EnumCls, str]` → `_Union[EnumCls, str, int]` 跨 4 Python 包。

## 关键文件

- **Deferred 账本（SSOT）**: `docs/design/EAASP/DEFERRED_LEDGER.md` — Phase 4a 关闭 D148/D149/D152（+ T1-T4 已关 D151/D153/D154/D155）。0 P1-active 剩余。
- **ADR governance**: `/adr:status` 会话启动仪表盘；`/adr:trace <path>` 反查约束；ADR-V2-023 是 Phase 4 前置决策。
- **Phase stack**: `docs/dev/.phase_stack.json`（end-phase 后 0 active 预期）。
- **归档 checkpoint**: `docs/plans/.checkpoint.archive.json`（Phase 4a 归档后覆盖 3.6 的）。
- **本阶段 plan**: `docs/plans/2026-04-20-phase4a-debt-cleanup.md`（Verification Checklist 全 ✅）。

## 启动 checklist

1. `/adr:status` — 看当前 ADR 健康度 + 近期变更（ADR-V2-023 是 Phase 4 方向选择的前置）。
2. `git log origin/main..main --oneline | wc -l` — 确认 unpushed commit 数（预期 ~293）。
3. `cat docs/dev/.phase_stack.json | python3 -c "import json,sys; d=json.load(sys.stdin); print('active:', len(d['active_phases']))"` — 应为 0。
4. Phase 4 规划：读 ADR-V2-023 §P5（Leg B 激活条件）+ `docs/plans/` 中待规划项（若有），然后 `/gsd-discuss` 或 `/dev-phase-manager:start-phase`。
