# Grid Platform 下一会话指南

**最后更新**: 2026-04-20 GMT+8
**当前分支**: `main`（ahead `origin/main` by 275+11 commits — Phase 3.5 遗留 + Phase 3.6 新增）
**当前状态**: EAASP v2.0 **Phase 3.6 🟢 COMPLETE 5/5** @ `b81f455` (sign-off 2026-04-20) → **Phase 4 待规划**

## 本会话重点

**Phase 3.6 — Tech-debt Cleanup 完成**：5 项 Phase 3.5 审查遗留的 ready Deferred 全部关闭（D140/D145/D146/D147/D150），1–2 天工期、全低风险。详见 `docs/dev/WORK_LOG.md` 顶部条目。

## 项目进度

- [x] Phase 2 — Memory and Evidence（23/23 @ `f4bf9ad`）
- [x] Phase 2.5 — Container + MCP Pool（25/25 @ `844664d`）
- [x] Phase 3 — L1 Runtime Functional Completeness（35/35 @ `8ee05fe`）
- [x] Phase 3.5 — chunk_type Unification（19/19 @ `5b13898`）
- [x] **Phase 3.6 — Tech-debt Cleanup（5/5 @ `b81f455`）← 本次**
- [ ] Phase 4 — TBD（参考 ADR-V2-023 §P5 Leg B 激活条件 + `docs/plans/` backlog）

## 下一步优先级

1. **推 `origin/main`** —— 累积 286 unpushed commits（Phase 3.5 遗留 275 + Phase 3.6 新增 11）。先跑 `git log origin/main..main --oneline | wc -l` 确认数；再 `git push origin main`。
2. **Phase 3.6 后续清单**（5 项新 Deferred，全 🧹 tech-debt，Phase 4 前完成）：
   - **D151** — `crates/grid-engine/tests/harness_envelope_wiring_test.rs`：spy HookHandler/StopHook 断言 `ctx.event` 字段（~50 LOC）。阻止 D136 xfail 掩码掩盖 `.with_event(...)` 意外删除。
   - **D152** — 上游 `grpcio-tools` / `mypy-protobuf` int-accepting stubs 跟踪；或 `scripts/gen_runtime_proto.py` 加 post-process `.pyi` 脚本。目前 12 处 `# type: ignore[arg-type]`。
   - **D153** — `scripts/gen_runtime_proto.py` 加 `--out-dir` override flag（5 LOC）+ `lang/claude-code-runtime-python/Dockerfile` 去 symlink（-8 LOC）。Phase 4 runtime Dockerfile 增殖前完成。
   - **D154** — `pyrightconfig.json` per-env `pythonVersion` 统一为 `"3.12"`（pyproject 声明的 floor）或完全去掉 per-env version 让顶层 3.12 fallback 接管。
   - **D155** — `scripts/check-pyright-prereqs.sh` 预检 9 个 `.venv` 存在；或 `make setup` 覆盖全 9 包 `uv sync`。
3. **Phase 4 前必清**（P1-active，跨阶段）：
   - **D148** — pydantic-ai-runtime test bench 加厚（补 sdk_wrapper 等价测试 + agent loop 覆盖）。
   - **D149** — `lang/ccb-runtime-ts/src/proto/types.ts` SoT 同步保障（@bufbuild/protoc-gen-es 或 proto guard 注释 + CI grep）。

## 关键代码路径

- **Hook envelope (ADR-V2-006)**: `crates/grid-engine/src/hooks/context.rs`（struct/serialize）+ `crates/grid-engine/src/agent/harness.rs:1766/2236/2390`（3 个 `.with_event(...)` dispatch 位点）。
- **Session orchestrator**: `tools/eaasp-l4-orchestration/src/eaasp_l4_orchestration/session_orchestrator.py:334-384`（`_accumulate_delta` + `_record_coalesced_deltas` helpers）+ `send_message` L438/L442/L468 / `stream_message` L535/L541/L567。
- **Proto 生成 SSOT**: `scripts/gen_runtime_proto.py`（registry dict 4 行 + `--package-name` + `--proto-files`）+ Makefile `claude-runtime-proto` / `nanobot-runtime-proto` / `pydantic-ai-runtime-proto` / `l4-proto-gen` targets。
- **Pyright 配置**: `pyrightconfig.json`（10 executionEnvironments，exclude hermes/archive，strict off）。

## 关键文件

- **Deferred 账本（SSOT）**: `docs/design/EAASP/DEFERRED_LEDGER.md` — 11 项 Phase 3.5-onward 新增（4 CLOSED + 2 P1-active + 5 tech-debt）
- **ADR governance**: `/adr:status` 会话启动仪表盘；`/adr:trace <path>` 反查约束
- **Phase stack**: `docs/dev/.phase_stack.json`（0 active 预期）
- **归档 checkpoint**: `docs/plans/.checkpoint.archive.json`（Phase 3.6 被 end-phase 归档后覆盖 3.5 的）

## ⚠️ Deferred 未清项（下次 session 启动时必查）

> 本次 Phase 3.6 新增 5 项全部 🧹 tech-debt；加上 Phase 3.5 遗留的 D148/D149 P1-active，共 7 项待清。

| 来源 Phase | ID | 摘要 | 类别 |
|-----------|----|------|------|
| Phase 3.5 S1.T6 | D148 | pydantic-ai-runtime 测试密度 | 🟡 P1-active |
| Phase 3.5 S1.T7 | D149 | ccb TS enum SoT 同步 | 🟡 P1-active |
| Phase 3.6 T1 | D151 | harness envelope call-site 回归测试 | 🧹 tech-debt |
| Phase 3.6 T3 | D152 | grpcio-tools int-accepting stubs 上游跟踪 | 🧹 tech-debt |
| Phase 3.6 T4 | D153 | Dockerfile symlink → `--out-dir` override | 🧹 tech-debt |
| Phase 3.6 T5 | D154 | pyrightconfig pythonVersion 对齐 pyproject floor | 🧹 tech-debt |
| Phase 3.6 T5 | D155 | fresh-clone pyright prereq 检查 | 🧹 tech-debt |

## 启动 checklist

1. `/adr:status` — 看当前 ADR 健康度 + 近期变更
2. `git log origin/main..main --oneline | wc -l` — 确认 unpushed commit 数（预期 ~286）
3. `cat docs/dev/.phase_stack.json | python3 -c "import json,sys; d=json.load(sys.stdin); print('active:', len(d['active_phases']))"` — 应为 0
4. Phase 4 规划：参考 ADR-V2-023 §P5（Leg B 激活条件）+ `docs/plans/` 中待规划项
