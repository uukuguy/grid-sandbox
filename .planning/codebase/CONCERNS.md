# Codebase Concerns

**Analysis Date:** 2026-04-26
**Scope:** Full repo (Grid sandbox, post-Phase-4a archival commit `3df969e`)
**Phase context:** Phase 4a closed (sign-off `8629505`); 7/7 tasks DONE; ledger water-line at 0 closed-ledger debt for Phase 4a items. Several known cross-phase items still open and several NEW concerns surfaced by this 2026-04-26 scan.

**Severity legend:**
- 🔴 **Critical** — block work / will mislead implementers
- 🟠 **Important** — fix before next milestone (Phase 4 / 4b)
- 🟡 **Minor** — watch / parking-lot
- 🟢 **Resolved** — already closed but listed for traceability

---

## A. Technical debt — open ledger items (`docs/design/EAASP/DEFERRED_LEDGER.md`)

**Phase 4a closure status (verified 2026-04-26):**
The ledger's "状态变更日志" rows confirm D148 / D149 / D151 / D152 / D153 / D154 / D155 are **all ✅ CLOSED** (lines 364-370 of `docs/design/EAASP/DEFERRED_LEDGER.md`). Phase 4a end-phase claim is accurate.

**D120 ledger-state ambiguity (RESOLVED in audit):**
The prompt flagged D120 as a candidate for "ledger says P1-defer but phase_stack shows Phase 2.5 100% complete." Actual ledger state:
- Line 326 (新增 row, 2026-04-15): `🟡 P1-defer` — opening entry.
- Line 339 (closing row, 2026-04-16): `🟡 P1-defer → ✅ closed @ 7e083c7`.
- Aggregate table (line 386): D120 listed under `✅ closed`.
- **Conclusion**: D120 is correctly marked **CLOSED**; ambiguity comes from reading only the opening row, not the close-out row. Recommend ledger formatter: when closing a D-item, retro-edit the opening row from `**新增** 🟡 P1-defer` to `**新增→CLOSED**` so a single-row scan tells the truth. → See concern **NEW-A1** below.

### A.1 Open P1/P2 functional gaps (still active)

| ID | Title | Files | Severity | Origin | Action |
|----|-------|-------|----------|--------|--------|
| **D90** | `ServerMessage::ToolResult` WS schema lacks `tool_name` (grid-server + grid-platform) | `crates/grid-server/`, `crates/grid-platform/` | 🟡 | Phase 2 S1.T4 derivative | Phase 4b — frontend UI prerequisite (Leg B trigger). Defer until grid-server / grid-platform UI activates. |
| **D93** | `embed_batch` is sequential `for text in texts: await embed(text)` | `tools/eaasp-l2-memory-engine/src/.../embed.py` | 🟡 | S2.T1 review | Phase 5 — perf pass. Both Ollama and TEI support batched POST. |
| **D102** | `AgentLoopConfig.compaction` field not wired to YAML config layer | `crates/grid-server/src/api/agents.rs`, engine agent factory | 🟡 | S3.T1 coder | Phase 5 — config harden sweep. |
| **D105** | `HookPoint::ContextDegraded` string-alias retained for backward-compat | `crates/grid-engine/src/.../runtime.rs:1899` | 🟡 | S3.T1 coder | Phase 5 — breaking version cycle (deprecate `"ContextDegraded"`, keep `"PostCompact"`). |
| **D109** | `workflow.required_tools` invariant (only list tools agent really invokes) is not documented or enforced | `crates/grid-engine/src/skills/skill_parser.rs`, ADR-V2-016 | 🟠 | S3.T2 design | Phase 5 — ADR-V2-016 amendment + parse-time WARN. **Risk**: bad authors will trip D87 `tool_choice=Specific(next)` lockout. |
| **D125** | L4 `events/stream` poll cap 500/s — burst overflow silently lags | `tools/eaasp-l4-orchestration/src/.../api.py` | 🟡 | S4.T2 reviewer | Phase 5 — only matters if L1 emits >1k events/sec. Add overflow log + adaptive poll interval. |
| **D134** | Shipped skill hooks read `.output.evidence_anchor_id` / `.output.draft_memory_id` (nested) but ADR-V2-006 §2.3 defines them as top-level | `examples/skills/threshold-calibration/hooks/check_output_anchor.sh`, `examples/skills/skill-extraction/hooks/check_final_output.sh` | 🟠 | Phase 2.5 S0.T3 | Phase 4 / 4b — when Phase 4 wires `with_event("Stop")` in production runtimes, fix the shipped hooks (or document top-level fields) **before** the wiring lands. |
| **D136** | Pre/PostToolUse hooks not firing on probe turn (grid-runtime) | `crates/grid-runtime/`, `tests/contract/harness/mock_openai_server.py` | 🟠 | Phase 2.5 S0.T4 | Phase 4 — root cause is mock OpenAI tool_calls JSON shape vs Rust adapter parser; 3 contract xfails (`test_hook_envelope.py --runtime=grid` Pre/Post scope) blocked on this. |
| **D137** | Multi-turn observability + MCP bridge live + PRE_COMPACT threshold trigger (10 contract xfails) | `tests/contract/contract_v1/test_event_type.py`, `test_proto_shape.py`, `test_mcp_bridge.py` | 🟡 | Phase 2.5 S0.T4/T5 | Phase 5 / 6 — needs harness extension (multi-turn replay framework + MCP subprocess fixture + scriptable PreCompact threshold mock). |
| **D138** | Skill-workflow enforcement tests need scriptable deny-path mock LLM | `tests/contract/harness/mock_openai_server.py`, `mock_anthropic_server.py` | 🟡 | Phase 2.5 S0.T4 | Phase 5 — `tools/eaasp-l4-orchestration/` mocks need `tool_choice` + scenario-routed deny fixtures. 5 contract xfails (`test_skill_workflow.py`). |
| **D142** | grid-runtime does NOT read `EAASP_DEPLOYMENT_MODE` env (per ADR-V2-019 D2) | `crates/grid-runtime/src/main.rs`, `service.rs:181` | 🟡 | ADR-V2-019 audit | Phase 4 — ~20 LOC; map env → existing `DeploymentMode::{Shared, PerSession}` enum. |
| **D143** | claude-code-runtime does NOT read `EAASP_DEPLOYMENT_MODE` env, no max_sessions=1 gate | `lang/claude-code-runtime-python/src/claude_code_runtime/service.py` | 🟡 | ADR-V2-019 audit | Phase 4 — ~20 LOC; on `per_session` mode reject `CreateSession` with `RESOURCE_EXHAUSTED` once `len(_sessions) >= 1`. |

### A.2 Open P3 / nice-to-have items (count: 22 across D92, D96, D97, D99, D100, D101, D103, D104, D106, D107, D110, D118, D119, D121, D122, D123, D126, D127, D128, D129, D135, D139)

All low-impact polish / edge-case / test-ergonomics items. Not action-blocking for Phase 4. See ledger §D-detail rows for individual descriptions. Recommendation: **batch-clean** in a future Phase X.5 tech-debt sprint similar to Phase 3.6.

### A.3 Phase-3-gated items (9): D8, D9, D34, D38, D41, D46, D62, D63, D64
All depend on Phase 3 identity / tenant model (not yet started in Grid). Park until tenant model lands.

### A.4 Long-term (11): D21, D25, D32, D36, D56, D73, D75, D76, D77, D79, D80
Phase 4/5/6 capacity / observability / clustering work. No action needed now.

### A.5 Tech-debt batch (18): D10, D11, D13, D14, D15, D16, D17, D18, D19, D20, D22, D23, D24, D26, D28, D29, D30, D31, D33, D39, D42, D43, D44, D45, D48, D55, D57, D58, D59, D61, D65
Phase 0 / Phase 1 era housekeeping. Many can be cleared with a single SQLite-helper extraction (`eaasp_common.connect()` for D17/D18/D29/D30) and a global FastAPI exception handler (D22/D28). Schedule as a 2-day Phase X.5 batch.

### A.6 Frozen (2): D66, D88
hermes-runtime is permanently frozen per ADR-V2-017. Do not touch.

---

## B. Phase 4a project-review findings (NEW concerns — 2026-04-26)

### NEW-B1 🔴 STALE chunk_type wire values in L1 adaptation guide

**File:** `docs/design/EAASP/L1_RUNTIME_ADAPTATION_GUIDE.md` §4 (lines 92-105)

**Problem:** Section 4 "Send 事件类型语义" still documents **legacy free-string values** that contradict the ADR-V2-021 enum freeze:

| Doc says (stale) | ADR-V2-021 enum value (current truth) |
|------------------|----------------------------------------|
| `"text"` | `"text_delta"` (CHUNK_TYPE_TEXT_DELTA = 1) |
| `"tool_call"` | `"tool_start"` (CHUNK_TYPE_TOOL_START = 3) |
| `"hook_fired"` | (no equivalent — see proto enum) |
| `"pre_compact"` | (no equivalent — superseded by PreCompactHook event flow per ADR-V2-018) |

Line 472 and 492 in the same doc also use the legacy strings inside a TypeScript example.

**Impact:** New L1 runtime authors (Phase 4 grid-runtime hardening, Phase 4b nanobot/pydantic-ai upstream syncs, plus any external integrator who picks up the guide) will emit wire values that fail `tests/contract/cases/test_chunk_type_contract.py` and the 7-runtime CI matrix. They will discover the contract violation only at certification time — wasting hours of integration work.

**Severity:** 🔴 — this is a single-doc documentation bug that will mislead every future runtime author.

**Action:** Phase 4 first task — sweep `L1_RUNTIME_ADAPTATION_GUIDE.md` §4 plus any TypeScript examples and rewrite to ADR-V2-021 canonical values. Add a 1-line `<!-- @chunk-type-sync ADR-V2-021 -->` provenance marker so future contract guards can cross-check (mirror the D149 grep guard pattern for `ccb-runtime-ts/src/proto/types.ts`).

### NEW-B2 🟠 ADR-V2-023 references `docs/reviews/strategy-grid-two-leg-checklist.md` but file does not exist

**Files:**
- `docs/design/EAASP/adrs/ADR-V2-023-grid-two-leg-product-strategy.md` line 17 (`review_checklist:` field), line 207 (R2 mitigation), line 218 (Enforcement bullet), line 222 ("未来创建" placeholder), line 268 (per-phase end review).
- Expected (missing): `docs/reviews/strategy-grid-two-leg-checklist.md`

**Problem:** The ADR's Enforcement section names a checklist that the strategy depends on for review-gate enforcement. The file is currently a placeholder reference only — no real PR-review aid exists. Reviewers cannot mechanically check Leg-B dormancy violations.

**Impact:** Per ADR-V2-023 P2 (Leg B dormancy), any PR touching `grid-server` / `grid-platform` / `grid-desktop` / `web/` / `web-platform/` should be reviewed against the checklist. Without the file, reviewers fall back to memory — high probability of false acceptance for "preparing leg B" pre-warming code (the very R3/R4 risks the ADR calls out).

**Severity:** 🟠 — governance gate is unwired; combined with NEW-B3 below the dormancy rule has no enforcement.

**Action:** Phase 4 — author `docs/reviews/strategy-grid-two-leg-checklist.md` with the 5 questions already drafted in ADR-V2-023 §Enforcement (lines 222-228). Wire to PR template via `.github/PULL_REQUEST_TEMPLATE.md` if/when one exists, or reference from CODEOWNERS rule (NEW-B3).

### NEW-B3 🟠 `.github/CODEOWNERS` does not exist

**Files:** Expected `.github/CODEOWNERS` — confirmed missing 2026-04-26.

**Problem:** ADR-V2-023 P2 specifies that PRs touching dormant Leg-B components require explicit reviewer attention to verify "is this really necessary now?". Without CODEOWNERS, GitHub does not auto-request reviewers for those paths. Any drive-by PR can land Leg-B feature work without the dormancy challenge.

**Severity:** 🟠 — combined with NEW-B2 the dormancy enforcement is paper-only.

**Action:** Phase 4 — add `.github/CODEOWNERS` with at minimum:
```
# Leg-B dormant components — require explicit reviewer
/crates/grid-server/      @<owner>
/crates/grid-platform/    @<owner>
/crates/grid-desktop/     @<owner>
/web/                     @<owner>
/web-platform/            @<owner>
```
(Replace `@<owner>` with the actual GitHub handle once the team naming is fixed.)

### NEW-B4 🟡 60+ pre-EAASP-v2 legacy `docs/design/*.md` root-level files

**Files:** 61 files at `docs/design/*.md` (verified 2026-04-26). Examples: `AGENT_CAPABILITY_BOOST_DESIGN.md`, `AGENT_HARNESS_INDUSTRY_RESEARCH_2025_2026.md`, `AGENT_RUNTIME_ARCHITECTURE_AUDIT.md`, `ARCHITECTURE_DESIGN.md`, etc.

**Problem:** These predate the EAASP v2 pivot. Many describe a product framing that has been replaced by ADR-V2-001 through ADR-V2-023. CLAUDE.md notes (line ~13, ~337, ~377) say "code trumps stale docs" and the EAASP/ subdirectory is now authoritative, but the root-level files themselves are **not annotated** with their stale-status — a new contributor or external reviewer reading `docs/design/AGENT_RUNTIME_DESIGN.md` will treat it as current.

**Severity:** 🟡 — confusing but not blocking. Code wins over docs in practice.

**Action:** Phase 4b — either (a) move all 61 files to `docs/design/legacy/` with a 1-line `STATUS.md` index ("these predate EAASP v2 — see `docs/design/EAASP/adrs/` for current"), or (b) prepend a stale-banner block to each via a one-shot script. Option (a) preferred — single move + index, no per-file edit.

---

## C. Code-level concerns (NEW + observed)

### NEW-C1 🟡 `crates/grid-engine/src/agent/harness.rs` is 3551 LOC monolithic

**File:** `crates/grid-engine/src/agent/harness.rs` (3551 lines)

**Problem:** Single file owns the entire agent execution lifecycle: round dispatch, hook envelope wiring (3 sites at L1755 / L2236 / L2390 per D151 close-out), retry policy, cancellation, compaction trigger, budget tracking, post-task hooks. Phase 4a Rust review identified extraction candidates: `handle_compaction_and_budgets`, `execute_round_hooks`, `call_llm_with_retries`.

**Severity:** 🟡 — currently cohesive (one read keeps the whole loop in head). Extraction without reuse would create artificial module boundaries.

**Action:** Defer until a real second consumer needs the same code (e.g., a parallel batch evaluation runner in `grid-eval`). Document the extraction candidates in a new `crates/grid-engine/REFACTOR_NOTES.md` or as a 🧹 ledger D-item so the deferral is tracked.

### NEW-C2 🟡 `crates/grid-cli/src/tui/key_handler.rs` is 1556 LOC

**File:** `crates/grid-cli/src/tui/key_handler.rs` (1556 lines)

**Problem:** Single file handles all of: key parsing, dispatch table, state mutation (mode transitions, focus changes, completion popup wiring). Phase 4a TUI review flagged this as a candidate for extraction into `parse.rs` / `dispatch.rs` / `mutate.rs` — but the dispatch table itself is intrinsically large.

**Severity:** 🟡 — TUI key tables grow naturally with feature count.

**Action:** Phase 5 — split when adding the next major TUI feature (e.g., split-pane mode) so the refactor pays for itself.

### NEW-C3 🟡 `grid-eval` orchestrator manual code

**Files:**
- `crates/grid-eval/src/runner.rs` (1487 lines)
- `crates/grid-eval/src/scorer.rs` (2011 lines)
- Combined 3498 LOC.

**Problem:** Manual orchestration where an `EvalRunner` struct + trait-based scorer plug-in would reduce repetition and let scorers be added without touching `runner.rs`. Phase 4a review noted but did not propose mandatory refactor.

**Severity:** 🟡 — works correctly, just not extensible.

**Action:** Phase 5 — refactor when adding the 3rd or 4th scorer type. Until then leave alone.

### NEW-C4 🟢 (RESOLVED-IN-ANALYSIS) `unwrap()` count in grid-engine

**Files:** `crates/grid-engine/src/**/*.rs` — 1001 `.unwrap()` occurrences across 125 source files.

**Original concern (Phase 4a review):** "977 `.unwrap()` in grid-engine — production paths use `?` per code review."

**Verification 2026-04-26:** Spot check of three of the heaviest files (`skills/loader.rs`: 105 unwraps, `session/thread_store.rs`: 38, `commands.rs`: 33) confirms **all 100% live inside `#[cfg(test)] mod tests { ... }` blocks** — production count = 0 in all three samples. Spread of `.unwrap()` is consistent with test-code idiom.

**Severity:** 🟢 — not a real concern.

**Action:** No action. Document this conclusion so future scans don't re-flag.

### NEW-C5 🟡 `lang/hermes-runtime-python` toolchain divergence

**File:** `lang/hermes-runtime-python/pyproject.toml`

**Findings (verified 2026-04-26):**
- `[build-system] requires = ["setuptools>=61.0"]` — every other Python package in the repo uses `hatchling`.
- `requires-python = ">=3.11"` — every other Python package floors at `>=3.12` (per pyrightconfig.json after Phase 4a T2).

**Severity:** 🟡 — frozen per ADR-V2-017 (replaced by goose-runtime + nanobot-runtime).

**Action:** Do not touch. Document in CONCERNS as "intentionally divergent — frozen". If hermes is ever retired entirely (delete the directory), update `pyrightconfig.json` to drop the `exclude` entry.

### NEW-C6 🟠 Ledger row-edit-on-close convention not enforced

**File:** `docs/design/EAASP/DEFERRED_LEDGER.md`

**Problem:** D-items like D120 have a row at the time of opening (`新增 P1-defer`) and a separate row at time of closing (`P1-defer → ✅ closed`). When grep-scanning the ledger, the opening row reads as still-active. This caused the Phase 4a planning prompt to flag D120 (which is in fact closed) as ambiguous.

**Severity:** 🟠 — corrupts every audit / scan that doesn't read the closing row.

**Action:** Phase 4 — add a small format rule to the ledger preamble:

> **When closing a D-item**, retro-edit the original `**新增**` row to `**新增→CLOSED <commit>**` so a single grep tells the truth. The original closing-row entry remains for chronology.

Alternatively, mechanize: a `scripts/ledger-link-close-rows.sh` that walks the changelog and back-references row 1 from row N where state transitions to ✅.

---

## D. Contract / proto concerns

### NEW-D1 🟡 `tests/contract/conftest.py:47-50` cross-crate `sys.path` injection layering smell

**File:** `tests/contract/conftest.py` (lines 39-55)

**Verbatim from the source:**
> "The current cross-crate import is fine for MVP contract work but is a layering smell."

**Problem:** conftest `sys.path`-injects `lang/claude-code-runtime-python/src` to reuse generated proto stubs. Two known fix paths in the comment block: (a) ship dedicated `tests/contract/harness/_proto/` with its own regen Make target, or (b) expose existing stubs via a real Python package.

**Severity:** 🟡 — works correctly; non-blocking.

**Action:** Phase 5 — when adding a 3rd python consumer of the proto stubs, refactor to option (a).

### NEW-D2 🟠 `test_chunk_type_contract.py` has only 3 tests, not "58 cases × 7 runtimes"

**File:** `tests/contract/cases/test_chunk_type_contract.py` (197 LOC, 3 `def test_` functions)

**Problem:** Phase 3.5 / 3.6 plan and conftest documentation reference the contract matrix as "58 cases × 7 runtimes". The actual file at HEAD has 3 guard tests, **no per-runtime parametrize**. The 7-runtime certification matrix runs via `.github/workflows/phase3-contract.yml` (per-runtime CI matrix) but the local pytest file does not parametrize over runtimes.

**Severity:** 🟠 — spec/code drift in conftest docs; can mislead a future contributor expecting a parametric matrix locally.

**Action:** Phase 4 — either (a) expand `test_chunk_type_contract.py` to true parametric `@pytest.mark.parametrize("runtime", [...])` matching the CI matrix, or (b) update the conftest comment + ADR-V2-021 frontmatter `trace` field to clarify that local pytest covers the proto-level guards while the per-runtime crossing is CI-only.

### NEW-D3 🟡 `_loosen_enum_stubs` post-process is a workaround for upstream stalled PR

**Files:**
- `scripts/gen_runtime_proto.py` — `_loosen_enum_stubs(out_dir)` regex post-process
- Upstream: `protocolbuffers/protobuf#25319` (OPEN since 2026-01-14, REVIEW_REQUIRED, 3+ months stalled)

**Problem:** D152 close-out installed a regex that rewrites `_Union[<EnumCls>, str]` → `_Union[<EnumCls>, str, int]` in generated `.pyi` stubs because grpcio-tools-generated stubs reject `int` enum values that runtime accepts (proto3 enums on the wire are int). This works but is a workaround. If upstream merges, the post-process should be removed.

**Severity:** 🟡 — works idempotently and correctly; just maintenance overhead.

**Action:** Park. Add a CI-side cron / quarterly check for upstream PR status; remove the post-process when upstream lands.

---

## E. ADR governance concerns

### NEW-E1 🟡 F4 module-overlap warnings are noisy (52 advisory)

**Source:** `docs/design/EAASP/adrs/AUDIT-2026-04-19.md` + per session-04-26 audit notes.

**Problem:** F4 lint reports 52 module-overlap warnings between ADRs touching the same `affected_modules` paths. Phase 4a session-04-26 audit found ZERO genuine Decision-text contradictions — F4 was demoted from hard-fail to warning per S2476 2026-04-20.

**Severity:** 🟡 — noise but informative.

**Action:** Phase 4b — improve F4 logic so it ignores overlaps where one ADR is `record` type or where the affected_modules is a parent path of another ADR's narrower path (e.g., ADR-A claims `crates/grid-engine/` and ADR-B claims `crates/grid-engine/src/agent/harness.rs` — that's parent/child, not real overlap).

### NEW-E2 🟡 F3 reports 29 missing `enforcement.trace` items

**Source:** `docs/design/EAASP/adrs/AUDIT-2026-04-19.md` per-ADR rows ADR-V2-001 through ADR-V2-018 (`WARN: missing trace`).

**Problem:** Most contract ADRs have `enforcement.level: contract-test` but no concrete `trace: [...]` array pointing at test files. ADR-V2-020 is the gold standard (3 test files traced); ADR-V2-021 declares `trace` but the file did not exist at audit time (now does, after Phase 3.5 / 3.6).

**Severity:** 🟡 — F3 is advisory.

**Action:** Phase 4 — sweep all `contract`-typed ADRs (V2-001/002/003/005/006/018) and add `trace` arrays. ADR-V2-006 (Hook Envelope) should reference `tests/contract/contract_v1/test_hook_envelope.py` once D136 unblocks `--runtime=grid` from xfail.

### NEW-E3 🟡 F5 reports 1 stale ADR (ADR-V2-019 still Proposed)

**Source:** `AUDIT-2026-04-19.md` per-ADR row, plus ADR audit notes.

**Problem:** ADR-V2-019 (L1 Runtime Deployment Model) is `Proposed` since 2026-04-16; should graduate to `Accepted` once D142/D143 close. Currently both still 🟡 P1-defer.

**Severity:** 🟡 — proposed-state ADRs are not enforced.

**Action:** Phase 4 — close D142 + D143, then `/adr:accept ADR-V2-019`.

### NEW-E4 🟡 14 archived phases + 132 historical phase plans, no automated migration to GSD ROADMAP

**Files:**
- `docs/dev/.phase_stack.json` — 14 archived phases (Phase 0 → Phase 4a)
- `docs/plans/*.md` — 132 plan files
- Expected (missing): `.planning/ROADMAP.md`

**Problem:** The repo has switched to GSD planning convention but the historical phase work has not been migrated. Per the FROZEN-as-historical strategy adopted 2026-04-26, that's deliberate, but the ROADMAP.md is not yet authored.

**Severity:** 🟡 — historical record is fully intact; just not GSD-formatted.

**Action:** Phase 4 first hour — author `.planning/ROADMAP.md` referencing `docs/dev/.phase_stack.json` as the immutable historical archive and listing the live Phase 4 + Phase 4b roadmap entries.

---

## F. Build / CI concerns

### NEW-F1 🟡 15 commits ahead of `origin/main`

**Verification 2026-04-26:** `git rev-list --count HEAD ^origin/main` = 15.

**Includes:**
- Phase 3.5 (chunk_type unification, ADR-V2-021)
- Phase 3.6 (tech-debt cleanup D140/D145/D146/D147/D150)
- Phase 4a (debt cleanup D148/D149/D151-D155)
- Cutover prep commits

**Severity:** 🟡 — not a code issue. Per ADR-V2-023 out-of-scope-rules and `CLAUDE.md` "user-decided push timing", these are intentional.

**Action:** None. User pushes when ready.

### NEW-F2 🟡 Makefile has 130 targets — discoverability concern

**Problem:** Per `CLAUDE.md`, Grid Sandbox-specific targets prefix with `v2-` / `eaasp-` / `claude-runtime-*` / `goose-runtime-*` / `hermes-runtime-*`. New contributor hits `make help` to find them but the target list is overwhelming.

**Severity:** 🟡 — works correctly; just a UX concern.

**Action:** Phase 5 — split into `make help-eaasp` / `make help-runtime` / `make help-dev` sub-helps if it becomes a frequent friction point.

### NEW-F3 🟡 Container build workflow requires manual trigger

**Per `CLAUDE.md` behavioral rules:** Do NOT run full test suites or container builds autonomously.

**Severity:** 🟡 — by design.

**Action:** None.

### NEW-F4 🟡 `.github/workflows/phase3-contract.yml` 7-runtime matrix CI-only enforcement

Per ADR-V2-021 trace, the per-runtime contract matrix runs **only** in CI, not via `make`. Local devs cannot reproduce a per-runtime cross-check without running `make v2-phase3-e2e` and a separate per-runtime invocation.

**Action:** Phase 4 — add `make v2-phase3-contract-matrix` that runs the 7-runtime parametric loop locally (skip-if-runtime-binary-absent, mirroring the goose / claude-code skip pattern).

---

## G. Security

**Phase 4a verification — 2026-04-26 — no CVE-class issues found.**

| Area | Current state | Mechanism | File / ref |
|------|---------------|-----------|------------|
| API key handling | Loaded from `.env` only (gitignored). HMAC for API key auth. | `crates/grid-engine/src/security/`, `ADR-003-API_KEY_HMAC.md` | `.env*` are in `.gitignore` (verified) |
| Password hashing | Argon2 | `Cargo.toml` deps | — |
| Symmetric encryption | AES-GCM | `Cargo.toml` deps | — |
| Signing / zeroize | `ed25519-dalek` + `zeroize` (Phase 4a verified) | `Cargo.toml` deps | — |
| Tool execution autonomy | `SecurityPolicy` + `CommandRiskLevel` + `ActionTracker` 3-tier model | `crates/grid-engine/src/security/` | per-skill autonomy gate |
| Sandbox | Native subprocess primary; Wasmtime + Bollard (Docker) optional features | `crates/grid-sandbox/` | feature-flagged |

**Open security items** (low severity):
- **D17 / D18 / D29** — FastAPI path-param validation gaps in L3 / L4 (🧹 tech-debt batch). Limited blast radius (input only reaches authenticated session paths) but should be closed in batch sweep.
- **D8 / D9 / D38 / D46** — RBAC / `access_scope` / per-user `user_id` filtering not yet enforced (🔴 phase3-gated). Required before multi-tenant Leg-B activation.

**Action:** Phase 4 — no security work needed. Phase 3 (multi-tenant) MUST close D8/D9/D38/D46 before any Leg-B activation per ADR-V2-023 P5.

---

## H. Performance

**No active performance regressions identified.**

| Area | Current state | File / ref |
|------|---------------|------------|
| Cargo workspace | resolver=2 (deterministic builds) | `Cargo.toml` |
| Vector index | HNSW (in-process, ADR-V2-015), per-`model_id` directory | `tools/eaasp-l2-memory-engine/` |
| L2 hybrid retrieval | FTS5 + HNSW + time-decay weighted score; graceful-degrade tested per Phase 2 S2.T2 | `tools/eaasp-l2-memory-engine/`, ADR-V2-015 |
| Graduated retry | `RetryPolicy::graduated()` + ±15% jitter + `FailoverReason::recovery_actions` routing | `crates/grid-engine/src/.../retry.rs` (S1.T7 @ `8b532cb`) |

**Open perf items (deferred):**
- **D93** (embed_batch sequential) — easy 4-10x batching win. Phase 5.
- **D98** (HybridIndex rebuilds HNSW per search, ~10ms disk read) — Phase 5 with D94 store singleton refactor.
- **D103** (find_tail_boundary O(N²)-ish) — only matters at very long sessions; Phase 5.

**Action:** No urgent perf work. Schedule D93+D98+D103 as a Phase 5 single-day batch.

---

## I. Bus factor / knowledge concentration

### NEW-I1 🟡 `docs/dev/WORK_LOG.md` is 1680 LOC single file (prepend-on-top)

**Verification 2026-04-26:** `wc -l` = 1680.

**Problem:** Per `CLAUDE.md` the work log is prepend-on-top, so the most recent entry is line 1. File grows unbounded; soft cap unclear. Any single-developer outage means the freshest context is bottlenecked behind one append-only file with no fan-out.

**Severity:** 🟡 — currently navigable; will become harder past 3000 LOC.

**Action:** Phase 5 — adopt a yearly-rollover convention (`docs/dev/WORK_LOG.md` for current year, `docs/dev/WORK_LOG-2026.md` archive when calendar-year rolls). Or split per-phase like the existing `docs/plans/` directory.

### NEW-I2 🟡 `MEMORY.md` (auto-loaded user-global memory) is 49 KB / 342 lines

**Source:** User-global `~/.claude/projects/.../memory/MEMORY.md`, auto-loaded into context.

**Problem:** User-global CLAUDE.md flags MEMORY.md as approaching truncation limits ("WARNING: MEMORY.md is 342 lines and 49KB. Only part of it was loaded.").

**Severity:** 🟡 — affects context budget for every session.

**Action:** User-managed (outside repo scope). Recommendation: archive entries older than 60 days into topic-specific files under the same memory directory.

### NEW-I3 🟢 ADRs are well-distributed

15 ADRs across 8 contract / 5 strategy / 1 record + 1 governance. No single-author pattern observed in commit history. Bus-factor risk LOW for ADR knowledge.

---

## Summary by Severity

**🔴 Critical (1 item):**
- NEW-B1 — STALE chunk_type values in `L1_RUNTIME_ADAPTATION_GUIDE.md` §4 will mislead every future runtime author.

**🟠 Important (6 items):**
- NEW-B2 — `docs/reviews/strategy-grid-two-leg-checklist.md` referenced but does not exist.
- NEW-B3 — `.github/CODEOWNERS` does not exist; Leg-B dormancy unenforced.
- NEW-C6 — Ledger row-edit-on-close convention not enforced; D120-style ambiguity recurs.
- NEW-D2 — `test_chunk_type_contract.py` 3 tests not "58 × 7" claimed in conftest docs.
- D109 — `workflow.required_tools` invariant undocumented; risks D87 lockout.
- D134 — Shipped skill hooks read nested envelope keys; ADR-V2-006 §2.3 says top-level. Fix before Phase 4 wires `with_event("Stop")` in production.
- D136 — grid-runtime hook firing on probe turn (3 contract xfails).

**🟡 Minor (~30 items):**
- NEW-B4, NEW-C1, NEW-C2, NEW-C3, NEW-C5, NEW-D1, NEW-D3, NEW-E1, NEW-E2, NEW-E3, NEW-E4, NEW-F1, NEW-F2, NEW-F3, NEW-F4, NEW-I1, NEW-I2 — refactor candidates / housekeeping / governance polish.
- D90, D93, D102, D105, D125, D137, D138, D142, D143 — deferred functional gaps with concrete owners.
- 22 P3-defer items — batch in a Phase X.5 sweep.
- 18 tech-debt items (D10-D65 batch) — same.

**🟢 Resolved or non-issue (3 items):**
- NEW-C4 — 1001 unwraps verified all in `#[cfg(test)]` blocks; not a concern.
- NEW-I3 — ADR knowledge distributed.
- D120 — ledger ambiguity resolved by reading both opening + closing rows; D120 is correctly CLOSED.

---

## Recommended Phase 4 / 4b inclusions

**Phase 4 (must-do, 2-3 day cleanup):**
1. NEW-B1 — fix `L1_RUNTIME_ADAPTATION_GUIDE.md` §4 chunk_type values (1-2 hours).
2. NEW-B2 + NEW-B3 — author `docs/reviews/strategy-grid-two-leg-checklist.md` + `.github/CODEOWNERS` (1 hour).
3. NEW-E4 — author `.planning/ROADMAP.md` (30 min).
4. D142 + D143 — wire `EAASP_DEPLOYMENT_MODE` in grid-runtime + claude-code-runtime (~40 LOC total).
5. D134 — sweep shipped skill hooks `.output.X` → `.X` (or document top-level convention).
6. NEW-E2 — sweep ADR `enforcement.trace` for ADR-V2-001/002/003/005/006/018.
7. NEW-E3 — accept ADR-V2-019 once D142/D143 close.

**Phase 4b (nice-to-have):**
- NEW-B4 — move 61 legacy `docs/design/*.md` to `docs/design/legacy/`.
- NEW-D2 — clarify or expand `test_chunk_type_contract.py` parametric matrix.
- D109 — ADR-V2-016 amendment + `skill_parser.rs` parse-time WARN.
- D136 — root-cause grid-runtime hook firing on probe turn.

**Phase 5+ (long horizon):**
- 22 P3 items + 18 tech-debt items batch sweep.
- NEW-C1 / NEW-C2 / NEW-C3 — large-file extractions when a real second consumer appears.
- D93 + D98 + D103 — perf batch.

---

*Concerns audit: 2026-04-26*
*Cross-references: `docs/design/EAASP/DEFERRED_LEDGER.md` (471 LOC), `docs/design/EAASP/adrs/AUDIT-2026-04-19.md`, `docs/dev/.phase_stack.json` (Phase 4a archived row)*
