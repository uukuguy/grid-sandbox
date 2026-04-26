# Testing Patterns

**Analysis Date:** 2026-04-26

This document captures testing patterns across the Grid (`grid-sandbox`) codebase: Rust unit + integration, Python pytest per-runtime, the cross-runtime contract suite at `tests/contract/`, the Phase E2E harnesses under `scripts/`, and the CI workflows in `.github/workflows/`.

**Critical operational rule** (per CLAUDE.md "Behavioral Rules"): **Never** run full test suites (`cargo test --workspace`, `make test`) unsolicited. Run only targeted tests for changed code. If a full run is needed, **ask first**.

## Test Framework

**Rust:**
- Runner: built-in `cargo test`.
- Integration: each `crates/<crate>/tests/<topic>.rs` file is a separate test binary.
- Unit: inline `#[cfg(test)] mod tests { … }` at the bottom of source files.
- Async: Tokio test runtime via `#[tokio::test]` — `crates/grid-engine` and `grid-runtime` both depend on `tokio = { workspace = true }`.
- Workspace test command (do NOT run autonomously): `cargo test`.
- Targeted Make targets:
  - `make test-types` — `cargo test -p grid-types`
  - `make test-engine` — `cargo test -p grid-engine`
  - `make test-sandbox` — `cargo test -p grid-sandbox`
  - `make test-server` — `cargo test -p grid-server`

**Python:**
- Runner: `pytest` 8.x.
- Async: `pytest-asyncio` 0.24+ with `asyncio_mode = "auto"` (configured in every `pyproject.toml` `[tool.pytest.ini_options]`). No `@pytest.mark.asyncio` decorator needed.
- Per-runtime test discovery: `testpaths = ["tests"]`.
- HTTP mock: `respx` 0.21 (l4-orchestration), `pytest-httpx` 0.32 (nanobot-runtime).
- Run commands (per runtime):
  - `make claude-runtime-test`
  - `make goose-runtime-test`
  - `make hermes-runtime-test`
  - Direct: `cd lang/<runtime>/ && uv run pytest -xvs`

**TypeScript (Bun):**
- Runner: built-in `bun test` for `lang/ccb-runtime-ts/`.
- Currently smoke-only; contract suite drives full validation.

## Test File Organization

**Rust:**
- Inline unit tests: appended to source modules with `#[cfg(test)] mod tests`.
  - `grid-engine`: ~181 source files contain inline `#[cfg(test)]` modules.
  - `grid-cli`: ~59 inline test modules.
  - `grid-eval`: ~32 inline test modules.
- Integration tests: `crates/<crate>/tests/<topic>.rs`.
  - `grid-engine/tests/`: 94 `*.rs` files (highest coverage; contains regression suites like `d87_multi_step_workflow_regression.rs`, `stop_hooks_integration.rs`, `scoped_hook_wiring_integration.rs`).
  - `grid-server/tests/`: 17 `*.rs` files.
  - `grid-runtime/tests/`: 5 `*.rs` files (incl. `llm_provider_integration.rs`, `grpc_integration.rs`).
  - `grid-types/tests/`, `grid-eval/tests/`, `grid-platform/tests/`, `eaasp-claw-code-runtime/tests/`, `eaasp-goose-runtime/tests/`, `eaasp-scoped-hook-mcp/tests/`: present per crate.

**Python (per-runtime + per-tool):**

| Package | Tests directory | File count | Test density (test:src) |
|---------|----------------|------------|-------------------------|
| `lang/claude-code-runtime-python/` | `tests/` | 12 `test_*.py` | ~0.60x |
| `lang/nanobot-runtime-python/` | `tests/` | 6 `test_*.py` | ~0.42x |
| `lang/pydantic-ai-runtime-python/` | `tests/` | 4 `test_*.py` | 0.22x → 1.05x after Phase 4a T6 (22 tests, D148) |
| `lang/hermes-runtime-python/` | `tests/` | frozen (ADR-V2-017) | n/a |
| `tools/eaasp-l2-memory-engine/` | `tests/` | 16 files | ~1.13x (best in suite) |
| `tools/eaasp-l4-orchestration/` | `tests/` | 4+ files | ~0.92x |
| `tools/eaasp-l3-governance/` | `tests/` | smaller | — |
| `tools/eaasp-skill-registry/` | `tests/` | smaller | — |
| `tests/contract/` | (cross-runtime) | 22 files | n/a |

**Naming pattern:** `test_<module_under_test>.py` (e.g. `lang/claude-code-runtime-python/tests/test_service.py`, `tools/eaasp-l2-memory-engine/tests/test_files.py`).

**Fixtures and shared setup:**
- Rust: helper modules in `tests/common/mod.rs` per crate (when needed).
- Python: `conftest.py` per `tests/` directory. Top-level `tests/contract/conftest.py` is the shared cross-runtime harness.
- Test fixtures (data) under `tests/fixtures/` (e.g. `lang/claude-code-runtime-python/tests/fixtures/skill_extraction_input_trace.json` — 148 LOC, schema_version=1).
- Skill examples used as fixtures: `examples/skills/skill-extraction/`, `examples/skills/threshold-calibration/`, `examples/skills/check-output-anchor/`, `examples/skills/block-write-scada/`.

## Test Structure

**Rust unit (inline) example pattern** — observed in `crates/grid-engine/src/agent/stop_hooks.rs`:

```rust
// production code
pub enum StopHookDecision {
    Noop,
    InjectAndContinue(Vec<ChatMessage>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn stop_hook_noop_returns_no_messages() {
        let hook = NoOpStopHook::default();
        let decision = hook.on_stop(&Default::default()).await;
        assert!(matches!(decision, StopHookDecision::Noop));
    }
}
```

**Rust integration example pattern** — observed in `crates/grid-engine/tests/stop_hooks_integration.rs`:

```rust
//! Phase 2 S3.T4 integration tests for Stop hook dispatch.
//! Validates first-decisive-verdict semantics + MAX_STOP_HOOK_INJECTIONS cap.

use grid_engine::agent::*;

#[tokio::test]
async fn first_decisive_verdict_wins() {
    // Arrange real harness + register two Stop hooks
    // Act dispatch
    // Assert first InjectAndContinue's messages are returned
}
```

**Python suite organisation** — observed in `lang/claude-code-runtime-python/tests/test_service.py`:

```python
"""Tests for the claude-code-runtime gRPC Service.

S0.T5 contract harness fixtures live in tests/contract/. These
service-level tests exercise the in-process service object directly
without spinning up a real gRPC server.
"""
from __future__ import annotations

import pytest
from claude_code_runtime.service import ClaudeCodeRuntimeService

@pytest.fixture
def service() -> ClaudeCodeRuntimeService:
    return ClaudeCodeRuntimeService(...)

class TestInitialize:
    async def test_materialises_skill_files(self, service, tmp_path):
        ...
```

## Mocking

**Rust:**
- TDD-London (mock-first) preferred for new engine/runtime code (per CLAUDE.md "Code style").
- Trait-based mocks: define `trait XxxClient` then implement `MockXxxClient` for tests. Avoids `mockall` macro deps in most places.
- `tokio::sync::Mutex`/`RwLock` for shared mock state in async tests; `DashMap` for concurrent maps.
- ENV mutex pattern: tests touching env vars (`tokio::test`) coordinate via a static `ENV_MUTEX` plus captured `TempDir` to avoid parallel-test poisoning + Drop mid-test (lesson from S3.T5).

**Python:**
- `unittest.mock` for sync (`MagicMock`, `patch`).
- `unittest.mock.AsyncMock` for async coroutines.
- `MagicMock(spec=ServiceClass)` to lock attribute-typo failures (catch `srv.do_thing()` typos at attribute access time, not test runtime).
- `monkeypatch.setenv` / `monkeypatch.delenv` for env isolation.
- `respx` (l4-orchestration) and `pytest-httpx` (nanobot) for HTTP mocking.
- LLM call mocking: `tests/contract/harness/mock_openai_server.py` and `mock_anthropic_server.py` are real uvicorn-hosted mock servers; runtime subprocesses talk to them via `OPENAI_BASE_URL` / `ANTHROPIC_BASE_URL` overrides.

**What to mock:**
- External LLM providers (OpenAI, Anthropic) — always mocked in unit tests.
- gRPC L2 clients (memory engine, skill registry) — `MockMemoryEngine` provides deterministic 4-tool sequence (search → read → write_anchor → write_file).
- File-system I/O at unit level; integration tests use real `tempfile.TemporaryDirectory` / `tempfile.NamedTemporaryFile`.
- Subprocess hooks: replace `bash` script with deterministic Python mock that emits the same envelope.

**What NOT to mock (live in tests despite cost):**
- Hook bash scripts: `tests/contract/harness/hook_probe.py` runs the real `examples/skills/*/hooks/*.sh` via subprocess. Mock-of-shell is brittle and misses arg-parsing bugs.
- gRPC over loopback: contract suite spawns the runtime as a real subprocess and connects via real gRPC stub (`grpc.aio.insecure_channel`).
- L2 memory FTS5 + HNSW: tests use real SQLite + HNSW indexes (in `tempfile`).

**Hygiene rule (per MEMORY.md "Integration Test Before E2E"):** stubs MUST match real runtime output; tests MUST verify data landed in DB / disk, not just HTTP 200. Each phase concludes with an Explore-agent audit of integration blind spots.

**No live LLM in tests** (per CLAUDE.md "Subagent Usage" + project policy): never call real OpenAI / Anthropic from unit or integration tests. The single `#[ignore]` test in `crates/grid-runtime/tests/llm_provider_integration.rs` requires a live API key and is run manually.

## Fixtures and Factories

**Rust:**
- Fixture data: under `crates/<crate>/tests/fixtures/`.
- Builder pattern for complex types: `AgentLoopConfig::builder().stop_hooks(vec![...]).build()`.
- `tempfile::TempDir` for filesystem-isolated tests.

**Python:**
- Pytest fixtures: scoped via `@pytest.fixture(scope="function|module|session")` in `conftest.py`.
- Tempdir: `tmp_path` (pytest builtin) for per-test directories.
- JSON fixture replay: `lang/claude-code-runtime-python/tests/fixtures/skill_extraction_input_trace.json` (148 LOC, 6 TOOL_RESULT events) drives deterministic Phase 3 S3.T3 replay tests via `MockMemoryEngine`.

**Mock skill manifests:**
- `examples/skills/skill-extraction/SKILL.md` (158 LOC) — meta-skill used in S3.T2 / S3.T3 tests.
- `examples/skills/threshold-calibration/SKILL.md` — threshold tuning fixture (128 LOC, refined for compactness).

## Coverage

- **Rust:** no project-wide coverage gate enforced. `cargo tarpaulin` available locally; CI does not block on it.
- **Python:** no project-wide coverage gate. Per-runtime test density (test:src ratio) is the proxy metric tracked in Phase 4a reviews.
- **Contract suite (cross-runtime):** `make v2-phase3-e2e` runs 112 pytest cases × 7 runtimes; pass/fail per runtime tracked in CI matrix.

Phase 3 sign-off (2026-04-18): all 7 runtimes pass `contract-v1.1.0` with 42 PASS / 22 XFAIL each (XFAIL marks deferred contract gaps tracked in DEFERRED_LEDGER.md).

## Test Types

**Unit tests:**
- Scope: single function / type, no I/O, no subprocess.
- Location: inline `#[cfg(test)]` (Rust) / `tests/test_<module>.py` (Python).
- Example: `crates/grid-engine/src/agent/stop_hooks.rs` 4 inline unit tests cover `StopHookDecision` semantics + `MAX_STOP_HOOK_INJECTIONS` cap.

**Integration tests:**
- Scope: multi-component, may use tempdir / mock-server / loopback gRPC.
- Location: `crates/<crate>/tests/*.rs` (Rust) / `tests/test_*_integration.py` or `tests/test_*_e2e.py` (Python).
- Example: `crates/grid-engine/tests/scoped_hook_wiring_integration.rs` (399 LOC) materialises real `SKILL.md` to `{workspace}/grid-session-{sid}/skill/`, executes real bash hooks, asserts handler-count delta.

**Contract tests** (cross-runtime, frozen):
- Scope: same `.proto`-defined behaviour across all 7 runtimes.
- Location: `tests/contract/` (22 files).
- Tag: `contract-v1.0.0` (Phase 2.5 freeze) and `contract-v1.1.0` (Phase 3 freeze; tag local-only).
- Coverage:
  - `tests/contract/contract_v1/test_proto_shape.py` — proto wire shape.
  - `tests/contract/contract_v1/test_event_type.py` — event type enum.
  - `tests/contract/contract_v1/test_hook_envelope.py` — ADR-V2-006 envelope.
  - `tests/contract/contract_v1/test_skill_workflow.py` — skill required_tools dispatch.
  - `tests/contract/contract_v1/test_e2e_smoke.py` — end-to-end smoke.
  - `tests/contract/contract_v1/test_mcp_bridge.py` — MCP bridge.
  - `tests/contract/cases/test_chunk_type_contract.py` — Phase 3.5 chunk_type freeze (3 guard tests + 7-runtime matrix per `phase3-contract.yml`).
  - `tests/contract/cases/test_pre_phase3_skill_compat.py` — backward compat.
  - `tests/contract/cases/test_tool_namespace_enforcement.py` — ADR-V2-020 enforcement.
  - `tests/contract/cases/test_tool_conflict_resolution.py` — ADR-V2-020 conflict resolution.

**E2E (end-to-end) tests:**
- Scope: full L0 → L4 stack, multiple subprocesses, real gRPC.
- Driven by `scripts/eaasp-e2e.sh` and Make `v2-phase*-e2e` targets.

**Regression tests:**
- Named after the issue/Deferred ID: `crates/grid-engine/tests/d87_multi_step_workflow_regression.rs`, `tests/test_s2t4_state_machine.py`.
- Locked behaviour at the moment of fix; never relaxed without a follow-up Deferred entry.

**Skipped/Ignored tests** (5 total in Rust):
- `crates/grid-engine/tests/d87_multi_step_workflow_regression.rs` — `#[ignore]` per file header (D87 historical fix verified, kept as canary).
- `crates/grid-runtime/tests/llm_provider_integration.rs` — `#[ignore]` (requires live API key).
- `crates/grid-runtime/tests/grpc_integration.rs` — `#[ignore = "TODO(s2.t1): rewrite for v2 terminate telemetry envelope"]`.
- `crates/grid-eval/src/suites/e2e.rs` — inline `#[ignore]`.

## Common Patterns

**Async testing (Rust):**
```rust
#[tokio::test]
async fn dispatch_returns_first_decisive_verdict() {
    let decision = dispatch_stop_hooks(&hooks, &ctx).await;
    assert!(matches!(decision, StopHookDecision::InjectAndContinue(_)));
}
```

**Async testing (Python — auto mode):**
```python
# pyproject.toml: asyncio_mode = "auto"
# No decorator needed; `async def test_*` is auto-recognised.
async def test_initialize_materialises_skill_files(service, tmp_path):
    await service.initialize(...)
    assert (tmp_path / "skill" / "SKILL.md").exists()
```

**Error testing (Rust):**
```rust
#[tokio::test]
async fn invalid_input_returns_typed_error() {
    let err = service.do_thing(bad_input).await.unwrap_err();
    assert!(matches!(err, ServiceError::InvalidInput(_)));
}
```

**Error testing (Python):**
```python
def test_invalid_input_raises():
    with pytest.raises(ValueError, match="must be non-empty"):
        do_thing("")
```

**Subprocess + loopback gRPC** (`tests/contract/conftest.py` `RuntimeLauncher`):
1. Spawn the runtime binary (Rust) or `python -m <runtime>` as subprocess.
2. Pass env vars: `OPENAI_BASE_URL` (mock), `OPENAI_API_KEY=test`, `NO_PROXY=127.0.0.1,localhost` (macOS Clash quirk fix), `httpx trust_env=False`.
3. Connect via `grpc.aio.insecure_channel("127.0.0.1:<port>")`.
4. Drive contract assertions over real proto.
5. Tear down: `runtime.terminate()` + `runtime.wait()`.

**ENV isolation (macOS-specific quirk, per S0.T5 lesson):**
- `httpx.AsyncClient(trust_env=False)` — never honour proxy env vars in test client.
- Runtime subprocess gets `NO_PROXY=127.0.0.1,localhost` so the local Clash proxy doesn't hijack loopback.

**Probe-skill harness:**
- `tests/contract/harness/hook_probe.py` materialises Pre/Post/Stop envelopes for hook-contract assertions per ADR-V2-006 §2/§3.
- `tests/contract/cases/` directory hosts the canonical probe-skill SKILL.md + hooks.

## Test Commands

```bash
# Rust — targeted (preferred per CLAUDE.md)
make test-types                    # cargo test -p grid-types
make test-engine                   # cargo test -p grid-engine
make test-sandbox                  # cargo test -p grid-sandbox
make test-server                   # cargo test -p grid-server
cargo test -p grid-engine stop_hooks   # single test name match

# Rust — workspace (do NOT run autonomously)
make test                          # cargo test (workspace)

# Python — per runtime
make claude-runtime-test           # lang/claude-code-runtime-python pytest
make goose-runtime-test            # lang/nanobot dispatcher in goose harness
make hermes-runtime-test           # lang/hermes-runtime-python pytest (frozen)
cd lang/<runtime>/ && uv run pytest -xvs

# Python — per tool
cd tools/eaasp-l4-orchestration/ && uv run pytest -xvs
cd tools/eaasp-l2-memory-engine/ && uv run pytest -xvs

# Contract suite (cross-runtime)
make v2-phase2_5-e2e               # 4-runtime Phase 2.5 cut
make v2-phase3-e2e                 # 7-runtime, 112 pytest, 5 skip
make v2-phase3-e2e-rust            # Rust-side regression

# Phase E2E (drives multiple stacks)
make v2-phase2-e2e                 # 14 @assertions A1-A14 (SKIP_RUNTIMES=true default)
make v2-phase2-e2e-full            # with runtime 6-step
make v2-phase2-e2e-build           # with build step

# Static verification (always safe)
make verify                        # cargo check + tsc + vite build
make verify-runtime                # print manual runtime verification checklist
make verify-dual-runtime           # start both Rust + Python runtimes + certifier

# Quality
make fmt-check                     # rustfmt diff
make lint                          # cargo clippy
make web-lint                      # eslint
```

## E2E Harnesses

**Phase 2 harness** (`scripts/verify-v2-phase2.sh` + `verify-v2-phase2.py`):
- 14 `@assertions` A1-A14 covering 5/6 Phase 2 exit criteria (criterion #2 — three-runtime 6-step — explicitly deferred to human runbook per ADR-V2-017).
- `verify-v2-phase2.sh` (363 LOC) is the bash entry; `verify-v2-phase2.py` (621 LOC) holds the assertion logic.
- Default `SKIP_RUNTIMES=true` — set `WITH_RUNTIMES=true` to drive runtime subprocess.
- Default `WITH_BUILD=false` — set `WITH_BUILD=true` to add build step.
- Human runbook: `scripts/s4t3-runtime-verification.sh` (398 LOC) with 128-LOC checklist (Phase 2 S4.T3 deliverable).

**Phase 2.5 harness** (`scripts/phase2_5-e2e-verify.sh`):
- 37 KB bash script driving the 4-runtime cut.
- Manual sign-off: `scripts/phase2_5-runtime-verification.sh` (172 LOC) + `phase2_5-runtime-verification-checklist.md` (128 LOC).

**Phase 3 harness** (`scripts/phase3-runtime-verification.sh`):
- 10 KB; covers all 7 runtimes.
- Drives the 112-pytest contract suite via `make v2-phase3-e2e`.

**MVP harness** (`scripts/verify-v2-mvp.{sh,py}`, `scripts/e2e-mvp.sh`):
- Pre-Phase-2 baseline; kept for archaeological reference.

**Sub-utilities:**
- `scripts/check-pyright-prereqs.sh` (1.5 KB, Phase 4a T3/D155) — verifies `.venv` symlinks before running pyright.
- `scripts/check-ccb-types-ts-sync.sh` (4.5 KB, Phase 4a T5/D149) — TS-vs-proto enum drift guard.
- `scripts/gen_runtime_proto.py` (8.5 KB) — regenerates Python proto stubs.
- `scripts/dev-eaasp.sh` (25.6 KB) — boots all 4+ EAASP services with log rotation under `.logs/latest/`.

## CI Workflows (`.github/workflows/`)

| Workflow | Purpose | Trigger |
|----------|---------|---------|
| `phase3-contract.yml` (4.3 KB) | 7-runtime contract matrix v1.1 | PR + push to main on contract / proto / runtime paths |
| `phase2_5-contract.yml` (2.5 KB) | 4-runtime Phase 2.5 cut | PR + push (Phase 2.5 paths) |
| `phase4a-ccb-types-sync.yml` (1.3 KB) | TS enum guard (D149) | PR touching `lang/ccb-runtime-ts/` |
| `adr-audit.yml` (1.3 KB) | ADR governance F1-F5 lint | PR + push to docs/design/EAASP/adrs/ |
| `eval-ci.yml` (4.4 KB) | grid-eval evaluation suite | PR + push |
| `release.yml` (1.8 KB) | Release tagging + signing | Tag push |
| `container-build.yml` (3.4 KB) | Runtime container image builds | PR + push |

**Phase 3 CI matrix structure** (`.github/workflows/phase3-contract.yml`):
- `strategy.matrix.runtime: [grid, claude-code, goose, nanobot, pydantic-ai, ccb, claw-code]`.
- `fail-fast: false` (so all 7 runtimes get verdicts even if one fails).
- Conditional steps: protoc install only for Rust runtimes (`grid`, `goose`, `claw-code`); Bun install only for `ccb`.
- Single shared dependency setup: `make v2-phase2_5-ci-setup`.
- Security note: matrix values are static; no `github.event.*` interpolated into shell commands.

## Test Review Pattern (Phase 4a established)

**Two-stage review** for high-risk tasks (proto, contract, harness, multi-runtime impact):
1. **Spec-compliance reviewer** — verifies proto / ADR / contract semantics. Catches I1/I2/I3-class issues GSD single-pass review would miss.
2. **Code-quality reviewer** — types, error paths, edge cases, performance.

Driven via `/superpowers:subagent-driven-development` (same-session) or `/superpowers:executing-plans` (parallel-session).

**GSD-only review** for medium/low risk: single reviewer pass via `code-reviewer` subagent.

**Reviewer escalation:**
- 0 Criticals → APPROVE.
- 1+ Critical → REQUEST-CHANGES; orchestrator fixes inline before re-verify.
- Majors → either inline-fix or file as Deferred D-item in `docs/design/EAASP/DEFERRED_LEDGER.md` with rationale.

**Adversarial test patterns established Phase 2-3** (per MEMORY.md):
- Honeypot tests: `MockMemoryEngine` raises `AssertionError` if the LLM ever calls a forbidden tool. Catches drift even when the assertion shape looks "happy path".
- Lock tests for blueprint constants: explicit assertion on `MAX_STOP_HOOK_INJECTIONS == 3` so the cap can never silently shift.
- Tautological-assertion guard: never write `assert "skill_submit_draft" not in mock_calls` on a 4-method mock — replace with structural traps that error on the forbidden code path.
- Tenant isolation in HNSW search: explicit scope/category filter on missing-ID DB fetch.

## Phase Verification Targets

| Phase | Make target | Python tests | Rust tests | Skipped |
|-------|-------------|--------------|------------|---------|
| Phase 2 | `make v2-phase2-e2e` | 14 @assertions A1-A14 | bundled | criterion #2 (3-runtime 6-step) |
| Phase 2.5 | `make v2-phase2_5-e2e` | 4-runtime contract | grid + claude-code + goose + nanobot | — |
| Phase 3 | `make v2-phase3-e2e` | 112 pytest | bundled | 5 |
| Phase 3 (Rust) | `make v2-phase3-e2e-rust` | — | regression suite | — |
| Phase 3.5 | `tests/contract/cases/test_chunk_type_contract.py` | 3 guard tests + matrix | — | goose/ccb DEP-SKIP |

## Known Test Skips / Deferred

- `crates/grid-engine/tests/d87_multi_step_workflow_regression.rs` — `#[ignore]` (canary kept for future schema drift detection).
- `crates/grid-runtime/tests/llm_provider_integration.rs` — `#[ignore]` (live LLM, manual run only).
- `crates/grid-runtime/tests/grpc_integration.rs` — `#[ignore = "TODO(s2.t1): rewrite for v2 terminate telemetry envelope"]`.
- `crates/grid-eval/src/suites/e2e.rs::e2e` — `#[ignore]` (inline).
- `tests/contract/cases/test_chunk_type_contract.py` — goose / ccb skipped (DEP-SKIP) in Phase 3.5 matrix; tracked in DEFERRED_LEDGER.md.

## Test Hygiene Rules (project-specific)

- **No live LLM calls** in any unit/integration test. Live calls are gated behind `#[ignore]` or `pytest.mark.live` and require explicit env vars.
- **No `unsafe` in tests** — verified Phase 4a (zero `unsafe` blocks across full codebase).
- **Mock servers > module mocks** for HTTP boundaries (real uvicorn-hosted mock OpenAI/Anthropic in `tests/contract/harness/`).
- **Verify data persisted** — assert against DB rows / disk files, not just HTTP 200 / pydantic round-trip (per MEMORY.md "Integration Test Before E2E").
- **Per-phase integration audit** — Explore-agent sweep before phase end-sign-off to identify integration blind spots.
- **Stub-realism check** — when adding mocks, pull a real-runtime trace and diff against the stub output; mismatch → fix the stub or the runtime.
- **Test:src ratio** is the target metric for Python packages; Phase 4a thickened pydantic-ai from 0.22x → 1.05x via D148.

---

*Testing analysis: 2026-04-26*
