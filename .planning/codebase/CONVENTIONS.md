# Coding Conventions

**Analysis Date:** 2026-04-26

This document captures the coding conventions enforced across the Grid (`grid-sandbox`) codebase. The repo is multi-language (Rust workspace, Python runtimes/tools, TypeScript/Bun runtime, future React frontend) and operates under ADR governance plus phase-driven development. Conventions below are derived from `CLAUDE.md` (root), the EAASP ADRs at `docs/design/EAASP/adrs/`, and observed patterns under `crates/`, `lang/`, `tools/`.

## Naming Patterns

**Files:**
- Rust source: `snake_case.rs` (e.g. `crates/grid-engine/src/agent/harness.rs`, `crates/grid-engine/src/agent/stop_hooks.rs`).
- Rust integration tests: `crates/<crate>/tests/<topic>.rs` (e.g. `crates/grid-engine/tests/stop_hooks_integration.rs`, `crates/grid-engine/tests/d87_multi_step_workflow_regression.rs`).
- Python source: `snake_case.py`. Package layout `lang/<runtime>/src/<pkg_name>/` and `tools/eaasp-*/src/eaasp_*/`.
- Python tests: `test_<module>.py` under `tests/` per package (e.g. `lang/claude-code-runtime-python/tests/test_service.py`).
- TypeScript: `kebab-case.ts` under `lang/ccb-runtime-ts/src/`.
- Proto: `lower_snake_case.proto` under `proto/eaasp/runtime/v2/` (`common.proto`, `runtime.proto`, `hook.proto`).
- Phase plans: `docs/plans/YYYY-MM-DD-<topic>.md` (e.g. `docs/plans/2026-04-14-v2-phase2-plan.md`).
- ADRs: `docs/design/EAASP/adrs/ADR-V2-XXX-<kebab-title>.md` (current EAASP) or `docs/adr/ADR-NNN-*.md` (legacy generic).
- Design docs: `docs/design/{EAASP,Grid}/<UPPER_SNAKE>.md` (e.g. `L1_RUNTIME_ADAPTATION_GUIDE.md`).
- Scripts: `scripts/<kebab-case>.{sh,py}` (e.g. `scripts/eaasp-e2e.sh`, `scripts/check-pyright-prereqs.sh`).

**Rust:**
- Functions: `snake_case` (e.g. `register_session_stop_hooks`, `dispatch_stop_hooks`).
- Types/structs/enums: `PascalCase` (e.g. `StopHookDecision`, `AgentLoopConfig`, `HookContext`, `FailoverReason`).
- Enum variants: `PascalCase` (e.g. `StopHookDecision::InjectAndContinue`, `ChunkType::WORKFLOW_CONTINUATION`).
- Constants: `SCREAMING_SNAKE_CASE` (e.g. `MAX_STOP_HOOK_INJECTIONS`).
- Modules: `snake_case` (e.g. `mod hooks;`, `mod stop_hooks;`).
- Public APIs: always `pub fn` / `pub struct` with explicit type signatures (per CLAUDE.md "Code style").

**Python:**
- Functions/methods: `snake_case` (e.g. `register_scoped_hooks`, `build_hook_vars`).
- Classes: `PascalCase` (e.g. `ScopedCommandExecutor`, `MockMemoryEngine`, `HookProbe`).
- Module-level constants: `UPPER_SNAKE_CASE` (e.g. `_ALLOWED_CHUNK_TYPES`).
- Private helpers: leading underscore (`_chunk_type_to_wire`, `_free_port`).
- Type annotations are mandatory on public functions (Python 3.12+ syntax: `list[str]`, `str | None`, no `typing.List`).

**Environment variables:**
- Grid-server prefix: `GRID_*` (e.g. `GRID_HOST`, `GRID_PORT`, `GRID_DB_PATH`).
- EAASP integration: `EAASP_*` (e.g. `EAASP_PROMPT_EXECUTOR`, `EAASP_DEPLOYMENT_MODE`, `EAASP_TOOL_FILTER`).
- LLM providers: `ANTHROPIC_*` and `OPENAI_*` — **never** invent variant names like `LLM_MODEL` (see MEMORY.md "Env Var Conventions"). Correct: `OPENAI_MODEL_NAME`, `ANTHROPIC_MODEL_NAME`.

## Code Style

**Rust:**
- Edition `2021`, `rust-version = "1.75"`, `resolver = "2"` (workspace `Cargo.toml`).
- Formatter: `rustfmt` via `make fmt` / `make fmt-check` (default settings).
- Linter: `clippy` via `make lint`.
- Workspace deps centralised in `Cargo.toml` `[workspace.dependencies]`; member crates inherit via `tokio = { workspace = true }`.
- Error handling: `Result<T, E>` plus `?` chaining; explicit `match` arms when behaviour diverges per variant. `unwrap()` and `expect()` are confined to test code (Phase 4a Rust review verdict).
- `unsafe`: ZERO `unsafe` blocks across the production codebase (Phase 4a verified). Three legacy occurrences exist only in test/legacy modules.
- `#[allow(...)]`: 60 directives total in 5 justified clusters: `dead_code`, `deprecated`, `clippy::ptr_arg`, `clippy::too_many_arguments`, `clippy::large_enum_variant`. Each must have a comment justifying the exception.
- Async: Tokio 1.42 (`features = ["full"]`) — never block the runtime; use `tokio::spawn`, `tokio::sync::Mutex`/`RwLock`. `DashMap` is preferred over `RwLock<HashMap<_>>` when iteration/mutation interleave (see `crates/grid-engine/src/agent/harness.rs` `session_stop_hooks`).
- File length: aim for <500 LOC per `CLAUDE.md` "Code style". Large files (`harness.rs` ~3551 LOC, `agent_loop.rs`) are accepted when refactor would break cohesion — prefer extracting modules over arbitrary splits.

**Python:**
- Formatter: Black (line length 100). Import sorter: isort (`profile=black`). Per global `~/.claude/CLAUDE.md` "Code Style Preferences".
- Floor version: `requires-python = ">=3.12"` for 8/9 packages. Hermes (`lang/hermes-runtime-python/pyproject.toml`) still allows ≥3.11 — pre-existing outlier per Phase 4a review; runtime is frozen (ADR-V2-017).
- f-strings preferred; never `.format()` or `%`-style formatting.
- `pathlib.Path` for filesystem work; never `os.path.join`.
- Logging: `loguru` (`from loguru import logger`) — required by global rules. `nanobot` and `pydantic-ai` runtimes already use it; legacy code calling `logging` is being migrated.
- `# type: ignore` density extremely low (~0.03% across ~19K runtime LOC; Phase 4a D152 post-process script eliminated 12 sites in `_proto/*` stubs to 0).
- Build backend: `hatchling` for 8/9 Python packages. `hermes-runtime-python` uses setuptools — Phase 4a noted as frozen runtime exception.
- pyproject convention: every package has `[tool.pytest.ini_options]` with `asyncio_mode = "auto"` and `testpaths = ["tests"]`.
- Common pyproject template (see `lang/claude-code-runtime-python/pyproject.toml`):
  ```toml
  [project]
  requires-python = ">=3.12"

  [project.optional-dependencies]
  dev = ["pytest>=8.0", "pytest-asyncio>=0.24"]

  [build-system]
  requires = ["hatchling"]
  build-backend = "hatchling.build"

  [tool.hatch.build.targets.wheel]
  packages = ["src/<pkg_name>"]

  [tool.pytest.ini_options]
  asyncio_mode = "auto"
  testpaths = ["tests"]
  ```

**TypeScript:**
- Strict mode mandatory (`tsconfig.json` `"strict": true`).
- Prefer `const`; avoid `var`. Arrow functions for callbacks.
- Bun runtime + TypeScript 5 for `lang/ccb-runtime-ts/`.
- Frontend (planned, dormant Leg B): React 19 + Vite 6 + Jotai 2.16 + Tailwind 4 (per CLAUDE.md "Tech Stack").

## Documentation Language

Per CLAUDE.md "File Organization Standards" and global "Documentation Language Rules":

| Document Type | Language | Example |
|---------------|----------|---------|
| Root `CLAUDE.md`, `README.md` | English | `CLAUDE.md` |
| ADRs (frontmatter keys) | English | `docs/design/EAASP/adrs/ADR-V2-023-*.md` |
| ADRs (body / decision text) | Bilingual (English + 中文 quotes acceptable) | ADR-V2-006, V2-017 |
| Design docs under `docs/design/{EAASP,Grid}/` | Chinese (中文) | `L1_RUNTIME_ADAPTATION_GUIDE.md` |
| Phase plans `docs/plans/*.md` | Chinese (中文) | `2026-04-14-v2-phase2-plan.md` |
| Work log `docs/dev/WORK_LOG.md` | Chinese (中文) | — |
| Code comments | English | All `*.rs`, `*.py`, `*.ts` |
| Error messages, log strings | English | All runtimes |
| Pre-EAASP-v2 root-level `docs/design/*.md` | Chinese (legacy, skip for current truth) | ADR/EAASP/Grid wins per CLAUDE.md §"Generic engine" |

**Authoritative source priority** (per CLAUDE.md): ADRs > `docs/design/EAASP/*.md` > `docs/design/Grid/*.md` > code > legacy root-level design docs.

## Import Organization

**Rust** (idiomatic, observed under `crates/grid-engine/src/`):
1. `std::*` imports first.
2. External crates (`tokio`, `serde`, `tonic`, `rmcp`, etc.).
3. Workspace crates (`grid_types`, `grid_sandbox`, `grid_engine`).
4. Crate-local (`crate::*`, `super::*`, `self::*`).
- Group separated by blank lines; sorted alphabetically within group (rustfmt default).

**Python** (per Black + isort `profile=black`):
1. `from __future__ import annotations` (top of file when present).
2. Standard library (`os`, `sys`, `pathlib`, `typing`).
3. Third-party (`pytest`, `httpx`, `grpc`, `pydantic`, `loguru`).
4. First-party / local (`from claude_code_runtime._proto...`, `from .module import ...`).
- Each group separated by a blank line.

**Path aliases:** None project-wide; `extraPaths` in `pyrightconfig.json` lets pyright find per-runtime `.venv` site-packages and `src/` directories per `executionEnvironments`.

## Error Handling

**Rust:**
- Custom errors via `thiserror::Error` with explicit variants.
- Propagation via `?` at boundaries; explicit `match` only when behaviour diverges per variant.
- Recovery: `RetryPolicy::graduated` with `FailoverReason` routing in `crates/grid-engine/src/recovery/`. 14 `FailoverReason` variants (Phase BH-MVP S1.T6, commit `4001de2`).
- Error context: prefer `anyhow::Context::with_context` for human-readable trails; `thiserror` for typed propagation.
- Hook execution failures fail-open by default (per ADR-V2-006 §10): exit 127 / timeout / bad JSON → `Noop`, never poison the session.

**Python:**
- Use specific exception types (`ValueError`, `RuntimeError`); avoid bare `except`.
- Async: handle `asyncio.TimeoutError`, `httpx.TimeoutException` explicitly.
- Hook executor (`lang/claude-code-runtime-python/src/claude_code_runtime/scoped_command_executor.py`) follows ADR-V2-006 §10 fail-open: exit 0 → allow, exit 2 → InjectAndContinue, all other / timeout → Noop.

**Configuration loading (project rule, MEMORY.md "No Fallback"):**
- Missing required config → fail loudly with explicit error; **never** silently fall back to a default. Helps catch typos and stale `.env` files in CI / dev.

## Logging

**Rust:**
- Framework: `tracing` 0.1 + `tracing-subscriber` 0.3 (`features = ["env-filter", "fmt", "json"]`).
- Env config: `GRID_LOG=grid_server=debug,grid_engine=debug` (per-crate filters).
- Format: `GRID_LOG_FORMAT=pretty` (dev) or `json` (prod).
- Spans for request/session lifecycle.

**Python:**
- Required: `loguru` (`from loguru import logger`). Standard `logging` module is being deprecated.
- No global config; each module imports the singleton `logger`.

**TypeScript:** TBD; `lang/ccb-runtime-ts/` uses `console.*` for now.

## Comments

**When to comment:**
- Public API: rustdoc / docstring required for `pub fn`, `pub struct`, all classes/methods exposed via gRPC / HTTP.
- Decision rationale: cite the ADR (`// Per ADR-V2-006 §3 …`).
- Non-obvious invariants: state `// SAFETY:` / `// INVARIANT:` / `// PRECONDITION:` blocks.
- TODO with phase reference: `// TODO(s2.t1): …` (see `crates/grid-runtime/tests/grpc_integration.rs`). Format: `TODO(<phase>.<task>)` ties debt to the phase that should clear it.

**When NOT to comment:**
- Self-explanatory variable names (per global rules: "Variable names should be self-explanatory (no comments needed)").
- Restating what code says.

**Rustdoc / TSDoc / docstrings:**
- Module-level `//!` summary at top of every Rust module.
- `///` doc comments on public items; describe ownership, error cases, and async/blocking semantics.
- Python: `"""triple-quoted docstrings"""`, type annotations supplement (don't repeat them).

## Function Design

- Size: aim <50 LOC per CLAUDE.md "Behavioral Rules"; large composers (loop drivers in `harness.rs`) accepted when extraction would shred cohesion.
- Nesting: avoid >3 levels.
- Single responsibility per function.
- Public functions: typed parameters + typed return; no untyped `**kwargs` in public Python APIs.
- Async-vs-sync split is explicit: `async fn` (Rust) / `async def` (Python) marks I/O boundaries; sync helpers stay sync.
- Input validation at system boundaries (per CLAUDE.md "Code style"): API endpoints, MCP tool invocations, CLI args, deserialization. Use `crates/grid-engine/src/security/` helpers for path sanitisation.

## Module Design

**Rust:**
- DDD with bounded contexts: module boundaries align with EAASP layers (per CLAUDE.md "Code style" + ADR-V2-023 P1).
  - `crates/grid-engine/src/agent/` — agent loop, harness.
  - `crates/grid-engine/src/event/` — event sourcing.
  - `crates/grid-engine/src/security/` — policy, risk, sandboxing helpers.
  - `crates/grid-engine/src/hooks/` — hook handlers.
  - `crates/grid-engine/src/recovery/` — retry/failover.
- Public re-exports via `mod.rs` / `lib.rs` (no barrel files needed; cargo handles dep graph).

**Python:**
- One package per runtime/tool (`src/<pkg_name>/`).
- `__init__.py` exposes the public surface; submodules host implementation.
- Proto stubs live under `<pkg>/_proto/eaasp/runtime/v2/` (underscore-prefix marks "generated, don't edit by hand"). `claude-code-runtime-python` proto stubs are reused by `tests/contract/conftest.py` via `sys.path` injection.

**Build order** (Cargo workspace handles automatically):
`grid-types` → `grid-sandbox` / `grid-engine` (parallel) → `grid-runtime` / `grid-cli` / `grid-eval` / `grid-server` / `grid-platform` / `grid-hook-bridge` → `eaasp-goose-runtime` / `eaasp-claw-code-runtime` / `eaasp-scoped-hook-mcp`.

## File Organization

Per CLAUDE.md "File organization (project-specific)":

| Kind | Location |
|------|----------|
| Rust source | `crates/<crate>/src/` |
| Rust integration tests | `crates/<crate>/tests/<topic>.rs` |
| Rust unit tests | inline `#[cfg(test)] mod tests` |
| Python runtime source | `lang/<runtime>/src/<pkg_name>/` |
| Python runtime tests | `lang/<runtime>/tests/test_<module>.py` |
| TS runtime source | `lang/ccb-runtime-ts/src/` |
| EAASP Python tools | `tools/eaasp-*/src/eaasp_*/` |
| Proto definitions | `proto/eaasp/runtime/v2/` |
| Cross-runtime contract tests | `tests/contract/` |
| Design docs (Chinese) | `docs/design/{EAASP,Grid,AgentOS,claude-code-oss}/` |
| ADRs (current EAASP) | `docs/design/EAASP/adrs/ADR-V2-XXX-*.md` |
| ADRs (legacy generic) | `docs/adr/ADR-NNN-*.md` |
| Phase plans | `docs/plans/YYYY-MM-DD-<topic>.md` |
| Work log (prepend new on top) | `docs/dev/WORK_LOG.md` |
| Phase state | `docs/dev/.phase_stack.json`, `docs/plans/.checkpoint.json` |
| Scripts | `scripts/<kebab-name>.{sh,py}` |
| Examples / fixtures | `examples/skills/<skill-name>/`, `tools/*/tests/fixtures/`, `lang/*/tests/fixtures/` |
| CI workflows | `.github/workflows/*.yml` |
| Vendored ADR plugin | `.adr-plugin/scripts/` |

**Hard rules** (per CLAUDE.md "Behavioral Rules"):
- **Never** save scratch markdown / ad-hoc tests / working files to repo root.
- **Never** create new `.md` or README docs unless explicitly requested.
- **Never** commit secrets, credentials, or `.env` files.
- **Always** read a file before editing it.
- **Never** run full test suites (`cargo test --workspace`, `make test`) unsolicited; ask first.

## Security & Validation

- Never hardcode API keys, credentials, or secrets in source. `.env` only (gitignored).
- Validate all user input at system boundaries.
- Sanitize file paths (prevent directory traversal) — use `crates/grid-engine/src/security/` helpers.
- Tool execution flows through `SecurityPolicy` + `CommandRiskLevel` + `ActionTracker` autonomy tiers.
- Hook envelope per ADR-V2-006: stdin JSON schema + `GRID_*` env vars + exit 0/2/fail-open + `${SKILL_DIR}` substitution + 5s timeout + cross-runtime parity.

## Commit Conventions

Per CLAUDE.md "Git Commit Guidelines" (project-level extends global):

- Subject ≤72 chars, imperative mood.
- Type prefixes: `feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`, `perf:`.
- Body answers WHY (the diff tells you WHAT).
- **MANDATORY footer** on every commit:
  ```
  Generated-By: Claude (claude-<model>) via Claude Code CLI
  Co-Authored-By: claude-flow <ruv@ruv.net>
  ```
- HEREDOC for multi-line messages:
  ```bash
  git commit -m "$(cat <<'EOF'
  feat(phase4a): tighten T2 pyright executionEnvironments
  ...
  Generated-By: Claude (claude-opus-4-7) via Claude Code CLI
  Co-Authored-By: claude-flow <ruv@ruv.net>
  EOF
  )"
  ```
- Always commit after `/dev-phase-manager:end-phase` or `/dev-phase-manager:checkpoint-progress`.
- Never commit mechanically before `/clear`; only after meaningful work.
- Before destructive operations (major refactor, branch switch), commit first.

Recent example commits (`git log --oneline -5`):
```
3df969e chore(phase4a): end-phase — archive 7/7 complete + Phase 4 handoff
8629505 fix(proto): D152 post-process grpcio-tools stubs to accept enum ints (T7)
a274ebd fix(tests): D148 tighten T6 signatures + reconcile ledger LOC
07318fd test(pydantic-ai): D148 thicken pytest bench to >=12 (T6)
aaf85aa fix(ci): D149 tighten T5 guard — wire-int check + portability
```

## ADR Governance

Project uses the global ADR governance plugin (meta-ADR: `docs/design/EAASP/adrs/ADR-V2-022-*.md`). Vendored scripts at `.adr-plugin/scripts/` so CI runs without the global plugin.

**ADR types:** `contract` (binding, has `affected_modules` + enforcement), `strategy` (architectural intent), `record` (one-time decision logging).

**Frontmatter keys (English, schema-enforced by `/adr:new`):** `id`, `status`, `type`, `accepted_date`, `superseded_by`, `affected_modules`, `enforcement.level`, `enforcement.trace`, etc.

**Enforcement levels** (per ADR-V2-022): `advisory` / `review-gate` / `contract-test` / `hook`.

**Lint gates F1–F5** run by `/adr:audit` and `.github/workflows/adr-audit.yml`:
- F1: schema correctness.
- F2: status transitions valid.
- F3: cross-references resolvable.
- F4: contract ADRs declare enforcement (demoted to **advisory warning** 2026-04-20 per S2476 — was hard-fail before).
- F5: title/id consistency.

**Slash commands:**
- `/adr:status` — session-start dashboard.
- `/adr:trace <path>` — which ADRs constrain a file (run BEFORE editing files in `affected_modules`).
- `/adr:new --type contract|strategy|record` — schema-enforced creation.
- `/adr:accept <id>` — runs F1-F4 lint then promotes Proposed → Accepted.
- `/adr:audit` — full lint gate.
- `/adr:review --health` — staleness check.

**Hard rules:**
1. Before editing files in an Accepted `contract` ADR's `affected_modules`, run `/adr:trace`. PreToolUse hook `~/.claude/hooks/adr-guard.sh` blocks violations automatically.
2. Never write ADR frontmatter by hand — `/adr:new` enforces the schema.
3. New `contract` ADRs without enforcement (`enforcement.trace` array + CI/hook) fail F4 lint (now advisory).

Current ADR audit: `docs/design/EAASP/adrs/AUDIT-2026-04-19.md`. Re-run quarterly. Latest accepted ADRs: `ADR-V2-021-chunk-type-contract.md` (chunk_type freeze), `ADR-V2-023-grid-two-leg-product-strategy.md` (product strategy anchor).

## Phase Workflow

Project development is **phase-driven**. State files (managed by `dev-phase-manager` skill — never hand-edit):

- `docs/dev/.phase_stack.json` — active phase stack.
- `docs/plans/.checkpoint.json` — current checkpoint.
- `docs/plans/.checkpoint.archive.json` — previous phase.
- `docs/dev/WORK_LOG.md` — work log (prepend new on top).
- `docs/dev/MEMORY_INDEX.md` — recent activity index.
- `docs/design/EAASP/DEFERRED_LEDGER.md` — cross-phase Deferred D-items (single SSOT for debt).

**Skills (don't write phase logic by hand):**
- `/dev-phase-manager:start-phase "<name>"` — open new phase.
- `/dev-phase-manager:end-phase` — archive (always commit after).
- `/dev-phase-manager:checkpoint-progress` — mid-phase save.
- `/dev-phase-manager:resume-plan` — pick up after `/clear`.
- `/dev-phase-manager:deferred-scan` — scan for unresolved Deferred items.
- `/superpowers:subagent-driven-development` — execute plan with reviewer loops (same session).
- `/superpowers:executing-plans` — parallel-session variant.

## Linting

**Rust:**
- `make fmt` / `make fmt-check` — rustfmt.
- `make lint` — `cargo clippy --workspace -- -D warnings` (zero clippy warnings on default-members).
- `make check` — fast `cargo check`.

**Python:**
- No project-wide `[tool.ruff]` config in `pyproject.toml` files (Phase 4a finding).
- Pyright: `pyrightconfig.json` (root) — 9 per-env `executionEnvironments`, all `pythonVersion: "3.12"` (Phase 4a T2/D154).
- Pyright prereq check: `scripts/check-pyright-prereqs.sh` (Phase 4a T3/D155) — verifies each runtime has `.venv` + symlinks before running pyright.
- Per-runtime `.ruff_cache/` exists where ruff is run ad-hoc (e.g. `lang/claude-code-runtime-python/.ruff_cache/`); no enforced CI gate yet.

**TypeScript:**
- `make web-check` / `make web-lint` — wraps `tsc --noEmit` and ESLint (configured per `web/` and `lang/ccb-runtime-ts/`).

**Proto:**
- `tonic-build` runs at compile time; no separate lint gate. Phase 4a T5 added a TS enum guard at `.github/workflows/phase4a-ccb-types-sync.yml` (D149) to detect drift between proto and TS-generated types.

## Service Ports (do NOT hardcode — use config)

| Port | Service | Source |
|------|---------|--------|
| 3001 | Backend (`grid-server`) | `GRID_PORT` / `config.yaml` |
| 5180 | Frontend Vite dev server (planned) | `web/vite.config.ts` |
| 50051 | `grid-runtime` gRPC | runtime config |
| 50052 | `claude-code-runtime-python` | runtime config |
| 50053 | `goose-runtime` | runtime config |
| 50054 | `nanobot-runtime` | runtime config |

---

*Convention analysis: 2026-04-26*
