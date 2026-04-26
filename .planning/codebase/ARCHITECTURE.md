# Architecture

**Analysis Date:** 2026-04-26

## Pattern Overview

**Overall:** Layered agent-runtime stack with two product legs (ADR-V2-023). The architecture follows the EAASP v2.0 L0-L4 model on the active leg (Leg A), while preserving a dormant Grid-independent product structure on Leg B. Cross-cutting design follows Domain-Driven Design with bounded contexts aligned to the EAASP layers.

**Key Characteristics:**
- **Substitutable L1 runtimes** behind a single 16-method gRPC contract (`proto/eaasp/runtime/v2/runtime.proto`). One flagship Rust runtime (`grid-runtime`) plus six comparison runtimes are validated against the same contract, proving portability rather than competing for a product slot.
- **Authoritative source priority**: ADRs at `docs/design/EAASP/adrs/ADR-V2-*.md` > `docs/design/EAASP/*.md` > `docs/design/Grid/*.md` > code. Root-level `docs/design/*.md` are PRE-EAASP-v2 LEGACY (~60 files, archaeological only).
- **Shared core / split toolchain**: `grid-engine` + `grid-types` + `grid-sandbox` (crate) + `grid-hook-bridge` are shared between both product legs and must compile cleanly for either (ADR-V2-023 P1).
- **Phase-driven evolution**: Architecture changes flow through ADRs (Proposed → Accepted → Deprecated/Superseded) with PreToolUse hook enforcement (`adr-guard.sh`) and CI lint (F1-F4).
- **Hook envelope contract** (ADR-V2-006): a single subprocess stdin JSON envelope shape for Pre/PostToolUse/Stop, normalized across Rust and Python L1 implementations.

## Layers

### L0 — Protocol

**Purpose:** The wire contract that lets L2/L3/L4 substitute any L1 runtime.
**Location:** `proto/eaasp/runtime/v2/`
**Contains:**
- `proto/eaasp/runtime/v2/common.proto` (7.5K) — `SessionPayload` 5-block priority stack (P1 PolicyContext .. P5 UserPreferences), `ChunkType` enum (8 closed variants incl `WORKFLOW_CONTINUATION=7` per ADR-V2-021).
- `proto/eaasp/runtime/v2/runtime.proto` (8.0K) — `RuntimeService` 16 methods: 12 MUST + 4 OPTIONAL + 1 PLACEHOLDER (`EmitEvent`, ADR-V2-001).
- `proto/eaasp/runtime/v2/hook.proto` (5.6K) — `HookEvent` oneof (incl `PRE_COMPACT=8` per ADR-V2-018).
- `proto/eaasp/runtime/v2/README.md` — runtime-tier classification (T1 Harness / T2 Aligned / T3 Framework).

**Frozen via ADRs:**
- ADR-V2-020 (Tool Namespace Contract) — `ToolLayer` enum; tools MUST declare layer.
- ADR-V2-021 (Chunk Type Contract Freeze) — `ChunkType` enum locked, mapping site at L4 `_chunk_type_to_wire`.
- ADR-V2-006 (Hook Envelope Contract) — stdin JSON shape, GRID_* env vars, exit 0/2/fail-open.

**Generation:** Rust via `tonic-build` in each crate's `build.rs`; Python via `scripts/gen_runtime_proto.py` post-processed by D152 fix (loosens grpcio-tools enum-int validation).

**Depends on:** Nothing (zero-dep wire format).
**Used by:** Every L1 runtime adapter, plus L4 `tools/eaasp-l4-orchestration/` for client invocation.

### L1 — Runtime (1 + 6 adapters)

**Purpose:** Executes the agent loop. Substitutable per session. Each adapter implements the same 16-RPC contract.

**Locations and roles:**

| Adapter | Path | Language | Role | Tier |
|---------|------|----------|------|------|
| **grid-runtime** | `crates/grid-runtime/` | Rust | Flagship — wraps `grid-engine` directly | T1 Harness |
| claude-code-runtime-python | `lang/claude-code-runtime-python/src/claude_code_runtime/` | Python | Comparison — Anthropic SDK baseline | T2 Aligned |
| goose-runtime | `crates/eaasp-goose-runtime/` + `crates/eaasp-scoped-hook-mcp/` | Rust | Comparison — Block goose via ACP subprocess + stdio MCP proxy | T3 Framework |
| nanobot-runtime-python | `lang/nanobot-runtime-python/src/nanobot_runtime/` | Python | Comparison — OpenAI-compat | T2 Aligned |
| pydantic-ai-runtime-python | `lang/pydantic-ai-runtime-python/src/pydantic_ai_runtime/` | Python | Comparison (Phase 3) | T3 Framework |
| claw-code-runtime | `crates/eaasp-claw-code-runtime/` | Rust | Comparison (Phase 3) | T3 Framework |
| ccb-runtime-ts | `lang/ccb-runtime-ts/src/` | TypeScript (Bun) | Comparison (Phase 3) | T3 Framework |
| hermes-runtime-python | `lang/hermes-runtime-python/src/` | Python | **Frozen** (ADR-V2-017) — replaced by goose+nanobot | n/a |

**Phase 3 sign-off (2026-04-18):** all 7 runtimes pass `contract-v1.1.0` (42 PASS / 22 XFAIL each).

**Authoritative reference:** `docs/design/EAASP/L1_RUNTIME_ADAPTATION_GUIDE.md` (21.4K, §12 covers TS/Bun) and `docs/design/EAASP/L1_RUNTIME_COMPARISON_MATRIX.md` (8.4K).

**Depends on:** L0 protocol (gRPC stubs), provider SDKs (Anthropic / OpenAI-compat).
**Used by:** L4 `eaasp-l4-orchestration` via `l1_client.py` (gRPC) and `tools/eaasp-cli-v2/`.

### L2 — Memory & Skills

**Purpose:** Persistent agent memory with hybrid retrieval, plus skill manifest registry and MCP server lifecycle.

**Tools:**
- `tools/eaasp-l2-memory-engine/src/eaasp_l2_memory_engine/` — Python / FastAPI:
  - `index.py` (18.7K) — hybrid retrieval orchestrator (FTS5 + HNSW + time-decay), `score = (w_fts*fts + w_sem*sem) * decay`, env override `EAASP_HYBRID_WEIGHTS` per ADR-V2-015.
  - `vector_index.py` (10.8K) + `embedding/` — HNSW per `model_id` directory.
  - `db.py` (7.0K) — SQLite + FTS5; schema migration with embedding columns (`embedding_model_id`, `embedding_dim`).
  - `files.py` (13.5K) — agent_suggested → confirmed → archived state machine; dual-write on confirm.
  - `mcp_server.py` (6.2K) + `mcp_tools.py` (9.2K) — 7 MCP tools: `memory_{search,read,write_file,write_anchor,confirm,list,delete}`.
  - `anchors.py` (4.1K), `event_index.py` (3.4K), `api.py` (3.3K), `main.py` (FastAPI entry).
- `tools/eaasp-skill-registry/src/` — Rust / Axum: `routes.rs` (11.6K), `skill_parser.rs` (15.3K), `store.rs` (14.5K), `git_backend.rs` (2.1K) — skill manifest storage + MCP tool bridge.
- `tools/eaasp-mcp-orchestrator/src/` — Rust / Axum: MCP server lifecycle across sessions (`manager.rs` 6.2K, `routes.rs` 3.2K).

**Three iron rules (ADR-V2-015):** model_id-keyed HNSW directories, dual-write on confirm only, semantic ranks decorate FTS hits (no pure semantic ranking).

**Depends on:** L0 protocol (events), embedding provider (Ollama bge-m3 or mock).
**Used by:** L1 runtimes (via `l2_memory_client.py` per runtime) + L4 (memory_refs in P3 SessionPayload).

### L3 — Governance

**Purpose:** Policy DSL, risk classification, deny/allow decisions before tool execution.

**Location:** `tools/eaasp-l3-governance/src/eaasp_l3_governance/`
**Files:**
- `policy_engine.py` (8.8K) — main policy evaluation loop.
- `audit.py` (5.5K) — audit trail.
- `managed_settings.py` (3.8K) — managed-policy bundles.
- `api.py` (6.8K) — FastAPI endpoints.
- `db.py` (2.5K) — SQLite-backed policy store.

**Depends on:** L0 protocol (`PolicyContext`), L4 events.
**Used by:** L4 orchestrator pre-tool gate.

### L4 — Orchestration

**Purpose:** Session lifecycle, SSE event fan-out, governance gates, MCP resolution. The hub that ties L1+L2+L3 together.

**Location:** `tools/eaasp-l4-orchestration/src/eaasp_l4_orchestration/`
**Key files:**
- `session_orchestrator.py` (753 LOC) — central orchestrator. Critical site: `_chunk_type_to_wire` (single mapping site per ADR-V2-021); `_ALLOWED_CHUNK_TYPES` whitelist; transitive L1 gRPC chain via session-mutating endpoints (`/v1/sessions/create` → `l1.initialize`, see Phase 2 S4.T3 lesson at `MEMORY.md`).
- `api.py` (541 LOC) — FastAPI REST endpoints (`/v1/sessions/*`, `/v1/events/*`).
- `l1_client.py` (399 LOC) — gRPC client wrapper to any L1 runtime.
- `event_engine.py`, `event_stream.py` (140 LOC, SSE), `event_backend.py`, `event_backend_sqlite.py`, `event_handlers.py`, `event_models.py`, `event_interceptor.py` — event pipeline (ADR-V2-002 / ADR-V2-003).
- `context_assembly.py` — assembles 5-block `SessionPayload` from L2/L3 inputs.
- `mcp_resolver.py` — resolves session-scoped MCP servers via L2 `eaasp-mcp-orchestrator`.
- `handshake.py` — three-way handshake (L4 ↔ L1).
- `db.py` — session store.

**Depends on:** L0 protocol, L1 (gRPC), L2 (HTTP/MCP), L3 (HTTP).
**Used by:** `tools/eaasp-cli-v2/` end-user CLI; `tools/eaasp-certifier/` contract harness.

## Two-Leg Product Strategy (ADR-V2-023)

### Leg A — EAASP integration (PRIMARY ACTIVE FOCUS)

**Status:** Active. All Phase 2 / 2.5 / 3 / 3.5 / 3.6 / 4a hardening targeted Leg A.

**What it is:** `grid-engine` + `grid-runtime` exposed via gRPC to a separate upstream EAASP project's L2/L3/L4. The `tools/eaasp-*/` here are **local shadows** of that upstream stack — high-fidelity test fixtures, NOT the production EAASP.

**Production target:** Enterprises buy EAASP from another team; Grid is their L1.

**Components used:**
- Shared core: `grid-types`, `grid-engine`, `grid-sandbox` (crate), `grid-hook-bridge`.
- L1 binding: `grid-runtime` (gRPC server wrapping `grid-engine`).
- Comparison runtimes: 6 adapters in `crates/eaasp-*-runtime/` and `lang/*-runtime-*/` for contract validation.
- Shadow EAASP stack: `tools/eaasp-{l2-memory-engine,l3-governance,l4-orchestration,skill-registry,mcp-orchestrator,certifier,cli-v2,mock-scada}/`.
- CLI: EAASP's own `tools/eaasp-cli-v2/`.

### Leg B — Grid independent product (DORMANT)

**Status:** Crates compile, no feature work. Activation requires ADR-V2-023 §P5 trigger conditions.

**What it is:** A direct-to-customer offering — Grid sold without going through EAASP.

**Production target:** Enterprises wanting Grid without EAASP.

**Components scaffolded:**
- `crates/grid-server/` — single-user workbench HTTP/WS server (Axum 0.8). `config.rs` (25.8K), `main.rs` (16.1K), `router.rs`, `state.rs`, `ws.rs` (20.2K), `api/`, `middleware/`, `migrations/`.
- `crates/grid-platform/` — multi-tenant platform server (Axum + JWT). `agent_pool.rs` (17.1K), `ws.rs` (17.5K), `auth/`, `tenant/`, `audit/`, `db/`.
- `crates/grid-desktop/` — Tauri app (excluded from default workspace build; `cargo build -p grid-desktop`).
- `web/` — single-user UI (React 19 + Vite 6 + Jotai + Tailwind). Scaffolding only.
- `web-platform/` — multi-tenant UI. Scaffolding only.
- `crates/grid-cli/` — primary client for Leg B; auxiliary for Leg A. `commands/`, `dashboard/`, `repl/`, `tui/`, `ui/`, `studio_main.rs` (4.8K).
- `crates/grid-eval/` — evaluation harness (primary for Leg B, auxiliary for Leg A). `runner.rs` (56.3K), `scorer.rs` (69.2K), `benchmark.rs` (29.1K), `comparison.rs` (31.6K).

**Rule:** Changes to shared core MUST work for both legs (ADR-V2-023 P1). New PRs touching `grid-server` / `grid-platform` / `grid-desktop` / `web*` need justification.

## Core Agent Loop Architecture (`grid-engine`)

**Scale:** ~109K LOC across `crates/grid-engine/src/`; 187 unit-test modules; 94 integration test files in `crates/grid-engine/tests/`.

**Top-level entry: `AgentRuntime`** (in `crates/grid-engine/src/agent/runtime.rs`, 88.3K / 2,136 LOC). Owns session lifecycle, scheduler, MCP wiring, scoped hook registration, and tool filter assembly.

**The agent loop core: `crates/grid-engine/src/agent/harness.rs`** (154.2K / **3,551 LOC** — the largest single file in the engine, file-length cap waived per CLAUDE.md "large files accepted when refactoring would break cohesion").

Companion modules in `crates/grid-engine/src/agent/`:

| File | LOC | Role |
|------|-----|------|
| `runtime.rs` | 2,136 | `AgentRuntime` — session orchestration, scheduler, scoped hook registration |
| `harness.rs` | 3,551 | Main agent loop, message threading, tool dispatch, hook fire sites, post-task hooks |
| `executor.rs` | 952 | `AgentExecutor` — per-turn execution, streaming, error handling |
| `loop_guard.rs` | 886 | Loop termination guard (death-spiral prevention, max iterations, token budget) |
| `loop_config.rs` | 632 | `AgentLoopConfig` — configurable loop bounds + stop_hooks vector |
| `dual.rs` | 623 | Dual-agent topology (`DualAgentManager`, `AgentSlot`, `ToolFilterMode`) |
| `subagent_runtime.rs` | 617 | Subagent runtime |
| `parallel.rs` | 549 | Parallel agent partition |
| `autonomous_trigger.rs` | 532 | Polling/cron/channel trigger sources for autonomous mode |
| `builtin_agents.rs` | 458 | Built-in agent profiles |
| `autonomous.rs` | 451 | Autonomous control / state |
| `queue.rs` | 406 | Task queue |
| `cancellation_tree.rs` | 338 | `CancellationTokenTree` (Session/Turn) |
| `self_repair.rs` | 289 | Self-repair on transient failures |
| `task_tracker.rs` | 267 | Task tracking |
| `loop_.rs` | 264 | `AgentLoop` |
| `stop_hooks.rs` | 261 | `StopHookDecision::{Noop, InjectAndContinue}`, `dispatch_stop_hooks` |
| `events.rs` | 259 | `AgentEvent`, `AgentLoopResult`, `NormalizedStopReason` |
| `interrupt.rs` | 256 | `SessionInterruptRegistry` |
| `collaboration/` | (subdir) | Multi-agent collaboration: `consensus.rs` (16.5K Byzantine), `protocol.rs` (13.2K), `manager.rs` (15.3K), `crypto.rs` (10.0K) |

**Other engine subsystems** (in `crates/grid-engine/src/`):
- `tools/` (51 files) — tool implementations and registry. Built-ins: `bash.rs` (23.0K), `bash_classifier.rs` (21.1K), `bash_guard.rs` (12.6K), `file_read.rs` (23.6K), `file_edit.rs`, `web_fetch.rs`, `web_search.rs` (17.7K), `task.rs` (17.7K subagent dispatch), `notebook_edit.rs`, `lsp.rs`, `tool_search.rs`, `mcp_*.rs`, `memory_*.rs`, `knowledge_graph.rs` (20.6K), `plan_mode.rs`, `worktree.rs`, etc. Registry in `mod.rs` (12.2K).
- `providers/` — LLM provider chain. `anthropic.rs` (24.3K), `openai.rs` (35.2K), `chain.rs` (25.8K), `pipeline.rs` (20.7K), `retry.rs` (26.9K), `error_classifier.rs` (22.6K — 14 `FailoverReason` variants), `smart_router.rs` (15.9K), `capabilities.rs` (14.6K), `response_cache.rs`, `usage_recorder.rs`, `metering_provider.rs`, `defaults.rs`.
- `hooks/` — hook engine. `context.rs` (27.9K — `HookContext::to_json` / `to_env_vars`), `registry.rs` (14.5K), `handler.rs`, `mod.rs` (`HookPoint` enum: PreToolUse, PostToolUse, PreTask, PostTask, SessionStart, SessionEnd, PreCompact, PostCompact, LoopTurnStart, LoopTurnEnd, AgentRoute, SkillsActivated, SkillDeactivated, SkillScriptStarted, ToolConstraintViolated, Stop, SubagentStop, UserPromptSubmit). Subdirs: `builtin/` (audit_log, security_policy), `declarative/`, `policy/`, `wasm/`.
- `memory/` — L0/L1/L2 memory. `sqlite_store.rs` (35.1K), `vector_index.rs` (20.0K), `hybrid_query.rs` (18.1K), `auto_extractor.rs` (22.2K), `memory_injector.rs` (12.6K), `reranker.rs` (15.5K), `round_memory.rs` (12.1K), `session_summarizer.rs`, `procedural_extractor.rs`, `event_extractor.rs`, `working.rs`, `graph.rs`, `fts.rs`, `embedding.rs`.
- `context/` — context engineering. `compaction_pipeline.rs` (40.0K — ADR-V2-018 PreCompact integration), `system_prompt.rs` (43.3K), `pruner.rs` (20.9K), `budget.rs`, `tool_use_summary.rs`, `observation_masker.rs`, `tiktoken_counter.rs`, `token_counter.rs`, `auto_compact.rs`, `compact_prompt.rs`, `collapse.rs`, `flush.rs`, `fork.rs`, `manager.rs`.
- `mcp/` — MCP client + server. `manager.rs` (26.5K), `oauth.rs` (20.1K), `server.rs` (12.3K), `traits.rs`, `storage.rs`, `bridge.rs`, `convert.rs`, `sse.rs`, `stdio.rs`.
- `security/` — `policy.rs` (`SecurityPolicy`), `pipeline.rs` (19.5K), `permission_engine.rs` (11.9K), `permission_rule.rs` (11.3K), `permission_types.rs`, `tracker.rs` (`ActionTracker`, `AutonomyLevel`, `CommandRiskLevel`), `ai_defence.rs` (17.7K).
- `audit/` — `storage.rs` (19.5K) `AuditStorage` / `AuditEvent`.
- `event/` — event store + bus + projection (CQRS-style). `bus.rs`, `store.rs` (10.7K), `projection.rs`, `reconstructor.rs`.
- `session/` — session store. `sqlite.rs` (13.8K), `thread_store.rs` (23.8K), `transcript.rs`, `events.rs`, `memory.rs`.
- `skills/` — skill loader/manager. `loader.rs` (41.9K), `selector.rs`, `dependency.rs`, `constraint.rs`, `execute_tool.rs`, `model_override.rs`, `semantic_index.rs`, `slash_router.rs`, `standards.rs`, `trust.rs`.
- `sandbox/` — sandbox dispatcher. `router.rs` (16.7K), `docker.rs` (19.7K), `wasm.rs` (16.7K), `subprocess.rs`, `external.rs`, `session_sandbox.rs` (16.1K), `target.rs` (19.1K), `profile.rs` (14.6K), `audit.rs`, `run_mode.rs`.
- `auth/`, `db/`, `metering/`, `metrics/`, `secret/`, `tls/`, `sync/`, `scheduler/`, `skill_runtime/`, `storage/`, `commands.rs`, `root.rs`.

## Tool Execution Flow

**Pipeline (per turn):**
1. LLM provider returns tool_call message.
2. `AgentExecutor` (`crates/grid-engine/src/agent/executor.rs`) extracts the tool invocation.
3. **PreToolUse hook fires** — `HookContext::with_event` builds the envelope (event, skill_id, tool_name, GRID_* env vars per ADR-V2-006). Hook subprocess receives JSON via stdin; exit 2 = deny, exit 0 = allow, other = fail-open.
4. `SecurityPolicy` evaluates against `PermissionRule` set; tool is classified by `CommandRiskLevel` (Low/Medium/High/Critical).
5. `ActionTracker` checks autonomy tier (`AutonomyLevel`).
6. Tool dispatched via `ToolRegistry::execute` (`grid-engine/src/tools/mod.rs`) or MCP bridge (`mcp/bridge.rs`).
7. Sandbox routing (`sandbox/router.rs`): native subprocess (default), Docker (`sandbox/docker.rs`), or WASM (`sandbox/wasm.rs`).
8. **PostToolUse hook fires** — same envelope shape; exit 2 = mutate/deny.
9. Result written into context via `tool_use_summary.rs`.
10. Loop continues; on natural termination, **Stop hooks dispatch** via `dispatch_stop_hooks` (`agent/stop_hooks.rs`).

## Stop Hooks (Phase 2 S3.T4 / ADR-V2-006 §10)

Defined in `crates/grid-engine/src/agent/stop_hooks.rs`:

```rust
pub enum StopHookDecision {
    Noop,
    InjectAndContinue(Vec<ChatMessage>),
}
```

**Semantics:** `dispatch_stop_hooks` runs all registered hooks; first `InjectAndContinue` wins (first decisive verdict), remaining hooks still run for observability side effects. Errors treated as `Noop` without poisoning subsequent hooks.

**Re-entry cap:** `MAX_STOP_HOOK_INJECTIONS = 3` prevents death-spirals.

**Dispatch site:** in `harness.rs` ~line 1732, after `if stop_reason != StopReason::ToolUse` block. Kept parallel to `fire_post_task_hooks` (per-round) — Stop is per-termination. 14 error return paths verified to return BEFORE the dispatch site.

**Bridge to bash hooks:** `grid-runtime/src/scoped_hook_handler.rs` (`ScopedStopHookBridge` impl `StopHook`) reuses `execute_command` + ADR-V2-006 envelope.

## Hook Envelope (ADR-V2-006)

**Contract:** subprocess stdin JSON envelope for Pre/PostToolUse/Stop, with `GRID_*` env vars and `${SKILL_DIR}` substitution. 5-second timeout. Exit 0 → allow / Noop. Exit 2 → deny / InjectAndContinue. Other → fail-open Noop.

**Rust side:**
- `crates/grid-engine/src/hooks/context.rs` (27.9K) — `HookContext::to_json` / `to_env_vars`. **Note:** D120 deferred — predates ADR-V2-006, missing `event` / `skill_id` / `*` fields + `GRID_EVENT` / `GRID_SKILL_ID` env. Shipped hooks coincidentally unaffected; MUST close before external runtime cert.
- `crates/grid-runtime/src/scoped_hook_handler.rs` (18.5K) — Stop bridge wiring.
- `crates/grid-runtime/src/harness.rs` (54.7K) — `register_scoped_hooks` materializes `SKILL.md` to `{workspace}/grid-session-{sid}/skill/`, builds `HookVars`, substitutes per hook.

**Python side (each runtime):**
- `lang/{runtime}/src/{pkg}/scoped_command_executor.py` — `ScopedCommandExecutor` (asyncio subprocess, env+stdin envelope, timeout kill+wait, fail-open on bad JSON / exit 127 / timeout). `ScopedHookBundle` partitioner.
- `lang/{runtime}/src/{pkg}/service.py` — `Initialize` materializes SKILL.md, substitutes, dispatch helpers `_dispatch_pre`, `_dispatch_post`, `_dispatch_stop` per gRPC site.

**Cross-runtime parity test:** `crates/grid-engine/tests/hook_envelope_parity_test.rs` (10.8K).

## Contract Testing Architecture

**Repo-wide harness:** `tests/contract/`
- `tests/contract/cases/` — 4 tests: `test_chunk_type_contract.py`, `test_pre_phase3_skill_compat.py`, `test_tool_conflict_resolution.py`, `test_tool_namespace_enforcement.py`.
- `tests/contract/contract_v1/` — frozen v1.0 baseline: `test_e2e_smoke.py`, `test_event_type.py`, `test_hook_envelope.py`, `test_mcp_bridge.py`, `test_proto_shape.py`, `test_skill_workflow.py`. Locked at tag `contract-v1.0.0` (local-only).
- `tests/contract/conftest.py` (26.1K) — fixtures: real runtime spawning (`GOOSE_BIN` / `shutil.which` skip-if-absent), `_free_port()`, mock OpenAI / Anthropic servers, hook_probe.py.
- `tests/contract/fixtures/` — fixture data.
- `tests/contract/harness/` — harness modules.

**Phase 3 sign-off:** all 7 runtimes × `contract-v1.1.0` = 42 PASS / 22 XFAIL each, run via `make v2-phase3-e2e` (112 pytest cases including B1-B8 batches).

**CI matrix:** `.github/workflows/phase3-contract.yml` (4.3K — 7-runtime matrix), `.github/workflows/phase2_5-contract.yml` (2.5K), `.github/workflows/phase4a-ccb-types-sync.yml` (1.3K).

## Workspace Dependency Graph (Cargo)

**Build order** (Cargo workspace handles automatically):
```
grid-types (zero-dep, foundation)
    ↓
grid-sandbox (crate) ─────── grid-engine
                                  ↓
                           grid-hook-bridge
                                  ↓
        ┌─────────────────────────┼────────────────────────────┐
        ↓                         ↓                            ↓
   grid-runtime              grid-cli  grid-eval         grid-server
   eaasp-goose-runtime                                   grid-platform
   eaasp-claw-code-runtime                               grid-desktop
   eaasp-scoped-hook-mcp                                 (Leg B dormant)
```

**Workspace members** (`Cargo.toml`):
- `crates/*` (all)
- `tools/eaasp-certifier`
- `tools/eaasp-mcp-orchestrator`
- `tools/eaasp-skill-registry`

**Default-members** (excludes `grid-desktop` due to Tauri rebuild-storm): `grid-types`, `grid-sandbox`, `grid-engine`, `grid-eval`, `grid-cli`, `grid-server`, `grid-platform`, `grid-runtime`, `grid-hook-bridge`.

## Data Flow

**Session lifecycle (Leg A):**

1. Client (`tools/eaasp-cli-v2/`) sends `eaasp session run -s <skill> -r <runtime> "<prompt>"`.
2. CLI hits L4 `/v1/sessions/create` (`tools/eaasp-l4-orchestration/src/.../api.py`).
3. L4 `session_orchestrator.py:259` builds 5-block `SessionPayload` (P1 PolicyContext from L3, P2 EventContext, P3 MemoryRefs from L2, P4 SkillInstructions from skill-registry, P5 UserPreferences).
4. L4 `l1_client.py:90-98` invokes `RuntimeService.Initialize(SessionPayload)` via gRPC against the chosen L1 runtime port (50051-50054).
5. L1 runtime materializes SKILL.md to workspace, registers scoped hooks, opens MCP servers per `ConnectMCP`.
6. Client sends `Send(UserMessage)` → L1 streams `SendResponse(ChunkType)`. ChunkType maps via L4 `_chunk_type_to_wire` (single-site, ADR-V2-021).
7. Per-turn: PreToolUse → SecurityPolicy → tool dispatch (sandbox or MCP) → PostToolUse.
8. On natural stop: Stop hooks dispatch (`InjectAndContinue` may re-enter, capped at 3).
9. L1 emits `EmitTelemetry` and (optionally) `EmitEvent` to L4 SSE backend (`event_backend_sqlite.py` per ADR-V2-002).
10. L1 `Terminate` → POST_SESSION_END.

**State Management:**
- Runtime state: in-memory + per-runtime SQLite (e.g. `data/grid.db`, L2 `EAASP_L2_DB_PATH`).
- Sessions: `crates/grid-engine/src/session/sqlite.rs` (13.8K) + `thread_store.rs` (23.8K).
- L4 sessions: `tools/eaasp-l4-orchestration/src/.../db.py`.
- Phase / checkpoint state: `docs/dev/.phase_stack.json`, `docs/plans/.checkpoint.json` (FROZEN as of 2026-04-26 cutover to GSD).
- Cross-phase debt: `docs/design/EAASP/DEFERRED_LEDGER.md` (69.1K SSOT, preserved under GSD).

## Key Abstractions

### `RuntimeService` (gRPC, L0)

**Purpose:** The 16-method contract every L1 runtime MUST satisfy.
**Examples:** `proto/eaasp/runtime/v2/runtime.proto` lines 23-77.
**Pattern:** 12 MUST + 4 OPTIONAL + 1 PLACEHOLDER (`EmitEvent` ADR-V2-001 pending).

### `AgentRuntime` (Rust, L1)

**Purpose:** Top-level engine entry. Owns sessions, scheduler, MCP, scoped hooks.
**Examples:** `crates/grid-engine/src/agent/runtime.rs` (88.3K). Re-exported via `crates/grid-engine/src/lib.rs:33`.
**Pattern:** Singleton-per-process; multiple sessions multiplexed.

### `AgentLoop` / `AgentExecutor` (Rust, L1)

**Purpose:** Per-turn execution. `AgentLoop` (loop_.rs 264 LOC) wraps `AgentExecutor` (executor.rs 952 LOC).
**Examples:** Spawned by `runtime.rs::build_and_spawn_executor_filtered`.
**Pattern:** `tokio` async; cancellation via `CancellationTokenTree` (Session ⊃ Turn).

### `HookContext` / `HookHandler` (Rust, L1)

**Purpose:** Hook envelope + dispatch.
**Examples:** `crates/grid-engine/src/hooks/context.rs`, `crates/grid-engine/src/hooks/handler.rs`.
**Pattern:** Async trait; `BoxHookHandler` for type-erased registration. Built-in handlers in `hooks/builtin/` (`audit_log.rs`, `security_policy.rs`).

### `StopHook` / `StopHookDecision` (Rust, L1)

**Purpose:** Termination-time hook injection.
**Examples:** `crates/grid-engine/src/agent/stop_hooks.rs`.
**Pattern:** Async trait; first decisive verdict; `MAX_STOP_HOOK_INJECTIONS=3`.

### `SessionPayload` (Proto, L0)

**Purpose:** 5-block priority stack for context budget trimming.
**Examples:** `proto/eaasp/runtime/v2/common.proto`. P1 PolicyContext (never removable) → P2 EventContext → P3 MemoryRefs → P4 SkillInstructions → P5 UserPreferences (removed first).
**Pattern:** Deterministic budget eviction across runtimes.

### `Provider` (Rust, L1)

**Purpose:** LLM provider abstraction.
**Examples:** `crates/grid-engine/src/providers/traits.rs`, with implementations in `anthropic.rs` / `openai.rs`. Chained via `chain.rs` + `pipeline.rs` with retry (`retry.rs`) and capability-driven routing (`smart_router.rs`, `capabilities.rs`).
**Pattern:** Trait + chain-of-responsibility; capability matrix (`PROVIDER_CAPABILITY_MATRIX.md`) for tool_choice support.

### `Tool` / `ToolRegistry` (Rust, L1)

**Purpose:** Built-in and MCP tool dispatch.
**Examples:** `crates/grid-engine/src/tools/mod.rs` (12.2K), `crates/grid-engine/src/tools/traits.rs`. ~50 built-in tools + dynamic MCP bridge.
**Pattern:** Trait registry + tier classification (ADR-V2-020 `ToolLayer`).

## Entry Points

### gRPC server (Leg A primary)

**Location:** `crates/grid-runtime/src/main.rs` (5.4K)
**Triggers:** Started by `make verify-dual-runtime`, `make claude-runtime-run`, or directly `cargo run -p grid-runtime`.
**Responsibilities:** Bind tonic gRPC server on `RUNTIME_PORT` (default 50051), implement `RuntimeService` 16 methods via `service.rs` (18.9K), wrap `grid-engine::AgentRuntime` via `harness.rs` (54.7K).

### Python runtime mains

**Locations:**
- `lang/claude-code-runtime-python/src/claude_code_runtime/__main__.py` (2.2K) — port 50052.
- `lang/nanobot-runtime-python/src/nanobot_runtime/__main__.py` (1.1K) — port 50054.
- `lang/pydantic-ai-runtime-python/src/pydantic_ai_runtime/__main__.py` — Phase 3.
- `lang/hermes-runtime-python/src/...` — frozen.

**Triggers:** `make {claude,nanobot,pydantic-ai}-runtime-run` or container.

### grid-server (Leg B dormant)

**Location:** `crates/grid-server/src/main.rs` (16.1K)
**Triggers:** `make server` / `make dev` (single-user workbench).
**Responsibilities:** Axum HTTP/WS server, in-process `grid-engine`, port 3001 (`GRID_PORT`).

### grid-platform (Leg B dormant)

**Location:** `crates/grid-platform/src/main.rs` (6.5K)
**Triggers:** Manual; not in `make dev` default.
**Responsibilities:** Multi-tenant Axum + JWT, agent-pool (`agent_pool.rs` 17.1K).

### grid-cli

**Location:** `crates/grid-cli/src/main.rs` (4.6K), `studio_main.rs` (4.8K)
**Triggers:** `make cli` / `grid` binary; studio TUI via `make studio-tui`.

### EAASP services (shadow stack)

**Locations:**
- `tools/eaasp-l4-orchestration/src/.../main.py` — FastAPI / uvicorn.
- `tools/eaasp-l2-memory-engine/src/.../main.py` — FastAPI / uvicorn.
- `tools/eaasp-l3-governance/src/.../main.py` — FastAPI / uvicorn.
- `tools/eaasp-skill-registry/src/main.rs` (1.4K) — Axum.
- `tools/eaasp-mcp-orchestrator/src/main.rs` (1.7K) — Axum.
- `tools/eaasp-cli-v2/src/...` — click/typer CLI.

**Triggers:** `make dev-eaasp` (orchestrates all under `.logs/latest/`); `make dev-eaasp-stop` to tear down.

## Error Handling

**Strategy:** Typed error enums via `thiserror`; classification via `providers/error_classifier.rs` (22.6K, 14 `FailoverReason` variants); graduated retry (`providers/retry.rs` 26.9K) with `RetryPolicy::graduated` + jitter + reason-based routing.

**Patterns:**
- **Hook errors:** Fail-open by default (per ADR-V2-006). Bad JSON / exit 127 / timeout → Noop.
- **Provider errors:** classified into `FailoverReason::{Transient, RateLimit, Unsupported, ModelGone, …}`; routed via `smart_router.rs` capability table.
- **Tool errors:** wrapped in `ToolResultBlock` with `is_error=true`; passed back to LLM as user message (per real Anthropic SDK behavior).
- **Loop guard:** `loop_guard.rs` (886 LOC) detects death-spirals (no progress, max iterations, token budget); raises `EStop` (`estop.rs`).
- **Cancellation:** `CancellationTokenTree` (`cancellation_tree.rs`) — Session token cascades to Turn tokens.

## Cross-Cutting Concerns

**Logging:** `tracing` + `tracing-subscriber` with `EnvFilter`. Init helpers in `crates/grid-engine/src/logging/`. Format selectable via `GRID_LOG_FORMAT=pretty|json`. Filter via `GRID_LOG=grid_server=debug,grid_engine=debug`. Python runtimes use `logging` stdlib + structured JSON.

**Validation:** Input validated at system boundaries:
- API: Axum extractors + serde.
- MCP tool calls: `mcp/convert.rs` + JSON schema check.
- CLI: `clap`-based.
- Deserialization: serde with `#[serde(deny_unknown_fields)]` where strict.
- File paths: `tools/path_safety.rs` (1.8K) prevents traversal.

**Authentication:** `crates/grid-engine/src/auth/` — `AuthMode`, `ApiKeyConfig`, HMAC (ADR-003). `crates/grid-platform/src/auth/` adds JWT for multi-tenant Leg B. AES-GCM + Argon2 + SHA-256 + `jsonwebtoken 9` per workspace deps.

**Observability:** `event/` (CQRS event store + bus + projection); `metrics/` (Counter/Gauge/Histogram registry); `audit/` (`AuditEvent` storage 19.5K). Telemetry via `EmitTelemetry` RPC (per-session) and SSE event stream (L4 fan-out).

**Sandboxing:** All tool execution flows through `sandbox/router.rs` (16.7K). Default native subprocess (`subprocess.rs`); optional Docker (`docker.rs` 19.7K via `bollard`); experimental WASM (`wasm.rs` 16.7K via `wasmtime`). Profile-driven (`profile.rs` 14.6K), session-scoped (`session_sandbox.rs` 16.1K).

**Skills:** Loaded via `skills/loader.rs` (41.9K); selected by `skills/selector.rs`; constraint-checked by `skills/constraint.rs`; trust-evaluated by `skills/trust.rs`. Built-in examples in `examples/skills/` (memory-confirm-test, skill-extraction, threshold-calibration, transformer-calibration).

**MCP Lifecycle:** Per-session via `mcp/manager.rs` (26.5K), persisted via `mcp/storage.rs`; OAuth flow in `oauth.rs` (20.1K); transports in `sse.rs` and `stdio.rs`. EAASP shadow has `tools/eaasp-mcp-orchestrator/` for cross-session lifecycle.

**Phase + ADR governance:** Phase state in `docs/dev/.phase_stack.json` (managed by `dev-phase-manager` skill). ADR governance plugin at `~/.claude/skills/adr-governance/` + vendored `.adr-plugin/scripts/` for CI. Hard rule: `adr-guard.sh` PreToolUse hook blocks edits to `affected_modules` of Accepted contract ADRs.

---

*Architecture analysis: 2026-04-26*
