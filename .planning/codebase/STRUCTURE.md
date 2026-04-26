# Codebase Structure

**Analysis Date:** 2026-04-26

## Directory Layout

```
grid-sandbox/
├── crates/                    # Rust workspace (12 crates) — shared core + Leg A + Leg B
│   ├── grid-types/            # Foundation: zero-dep type definitions (Shared)
│   ├── grid-sandbox/          # Sandbox runtime adapters (Shared) — distinct from repo name
│   ├── grid-engine/           # Core agent engine — ~109K LOC, 25 src subdirs (Shared)
│   ├── grid-hook-bridge/      # Hook event bridge between Rust ↔ L2/L3 (Shared)
│   ├── grid-runtime/          # Leg A primary — gRPC L1 wrapping grid-engine
│   ├── grid-cli/              # CLI binary (Leg A aux / Leg B primary)
│   ├── grid-eval/             # Evaluation harness (Leg A aux / Leg B primary)
│   ├── grid-server/           # Leg B DORMANT — single-user Axum HTTP/WS server
│   ├── grid-platform/         # Leg B DORMANT — multi-tenant Axum + JWT
│   ├── grid-desktop/          # Leg B DORMANT — Tauri (excluded from default workspace build)
│   ├── eaasp-goose-runtime/   # Leg A — comparison L1 (Block goose via ACP)
│   ├── eaasp-claw-code-runtime/  # Leg A — comparison L1 (Phase 3)
│   └── eaasp-scoped-hook-mcp/ # Leg A — stdio MCP proxy injecting Pre/Post-ToolUse hooks
│
├── lang/                      # Non-Rust L1 runtimes (1 sample + 4 comparisons + 1 frozen)
│   ├── claude-code-runtime-python/   # Python — Anthropic SDK baseline (T2 Aligned)
│   ├── nanobot-runtime-python/       # Python — OpenAI-compat (T2 Aligned)
│   ├── pydantic-ai-runtime-python/   # Python — Phase 3 comparison
│   ├── ccb-runtime-ts/               # TypeScript (Bun) — Phase 3 comparison
│   └── hermes-runtime-python/        # Python — FROZEN (ADR-V2-017)
│
├── tools/                     # EAASP shadow stack (NOT production EAASP) + utility tools
│   ├── eaasp-l4-orchestration/   # Python FastAPI — session lifecycle, SSE fan-out
│   ├── eaasp-l3-governance/      # Python FastAPI — policy DSL, risk classification
│   ├── eaasp-l2-memory-engine/   # Python FastAPI — FTS5 + HNSW + time-decay (ADR-V2-015)
│   ├── eaasp-skill-registry/     # Rust Axum — skill manifest + MCP tool bridge
│   ├── eaasp-mcp-orchestrator/   # Rust Axum — MCP server lifecycle across sessions
│   ├── eaasp-cli-v2/             # Python — end-user CLI (`eaasp session run …`)
│   ├── eaasp-certifier/          # Rust — contract certification harness
│   ├── mock-scada/               # Python — example external system for verification skills
│   └── archive/                  # Frozen v1.8 tools
│
├── proto/                     # Protobuf definitions (L0 protocol)
│   └── eaasp/runtime/v2/      # 3 proto files + README — frozen via ADR-V2-020/021
│
├── tests/                     # Top-level cross-runtime / cross-language tests
│   ├── contract/              # Contract harness (cases/, contract_v1/, fixtures/, harness/, conftest.py 26.1K)
│   ├── e2e/                   # End-to-end Python tests (api_contracts, hook_enforcement, hr_example, …)
│   ├── octo-workbench/        # 33 Playwright screenshot fixtures
│   ├── integration.rs         # Top-level Rust integration entry
│   └── test_*.py              # Top-level Python smoke tests
│
├── examples/                  # Sample skills + demo project + WASM hook plugin
│   ├── skills/                # 4 example skills incl. `skill-extraction` (Phase 2 S3.T2 meta-skill)
│   ├── demo-project/          # Demo project structure
│   └── wasm-hook-plugin/      # Example WASM-based hook
│
├── docs/                      # Documentation root (mostly Chinese per CLAUDE.md)
│   ├── design/                # Design docs (current authoritative + legacy)
│   │   ├── EAASP/             # CURRENT authoritative — incl. adrs/ (15 ADRs)
│   │   ├── Grid/              # CURRENT — Grid product framing, 3 docs
│   │   ├── AgentOS/           # Subdirectory
│   │   ├── claude-code-oss/   # Subdirectory
│   │   └── *.md               # PRE-EAASP-v2 LEGACY (~60 files, archaeology only)
│   ├── adr/                   # LEGACY generic ADRs (ADR-001 .. ADR-019)
│   ├── adr_archive/           # Archived ADRs
│   ├── ddd/                   # DDD docs
│   ├── decisions/             # Decision log
│   ├── dev/                   # Dev memory: WORK_LOG.md (83.7K), MEMORY_INDEX.md (91.5K), .phase_stack.json
│   ├── plans/                 # 135 phase plan files (FROZEN as of 2026-04-26 GSD cutover)
│   ├── main/                  # Branch-specific work logs incl. WORK_LOG.md (100.4K)
│   ├── work-logs/             # Additional work logs
│   └── found-by-sonnet4.6/    # Findings folder
│
├── scripts/                   # Build/verify/E2E scripts
│   ├── eaasp-e2e.sh           # 29.7K — full Phase 1-3 E2E
│   ├── verify-v2-{mvp,phase2}.{sh,py}  # 14 + 23K each — assertion harnesses
│   ├── phase2_5-e2e-verify.sh # 37K — Phase 2.5 E2E
│   ├── phase3-runtime-verification.sh  # 10K
│   ├── s4t3-runtime-verification.sh    # 17K — Phase 2 S4.T3 runbook
│   ├── dev-eaasp.sh           # 25.6K — orchestrates all EAASP services
│   ├── gen_runtime_proto.py   # 8.5K — Python proto codegen + D152 post-process
│   ├── check-ccb-types-ts-sync.sh
│   ├── check-pyright-prereqs.sh
│   ├── verify-dual-runtime.sh
│   ├── e2e-mvp.sh
│   ├── eval/                  # CLI/server eval drivers
│   └── assets/                # Test data (managed-settings JSON, threshold-calibration skill)
│
├── web/                       # Leg B DORMANT — single-user UI (React/Vite/Jotai/Tailwind)
├── web-platform/              # Leg B DORMANT — multi-tenant UI
├── data-platform/             # Platform data store
├── data/                      # Local SQLite databases (e.g. grid.db)
├── deploy/                    # Deployment artifacts
├── docker/                    # Dockerfiles
├── container/                 # Container build context
├── config/                    # Config templates
├── sdk/                       # SDK packaging
├── 3th-party/                 # Vendored third-party
├── eval_output/               # Evaluation output
├── target/                    # Cargo build cache
├── .planning/                 # GSD-managed planning state (THIS FILE'S HOME)
│   └── codebase/              # ARCHITECTURE.md, STRUCTURE.md (this file), …
├── .adr-plugin/               # Vendored ADR governance plugin scripts (.adr-plugin/scripts/)
├── .github/workflows/         # CI: adr-audit, container-build, eval-ci, phase2_5-contract, phase3-contract, phase4a-ccb-types-sync, release
├── .claude/, .claude-flow/, .omc/, .swarm/  # Claude Code + orchestration metadata
├── .grid/                     # Grid runtime state
├── .e2e/, .logs/              # E2E + log rotation
├── .playwright/, .playwright-cli/, .playwright-mcp/  # Playwright artifacts
├── .pytest_cache/, .ruff_cache/  # Python tooling caches
├── .venv/                     # Python virtualenv
├── tmp/                       # Scratch
│
├── CLAUDE.md                  # Project instructions (THIS file's authoritative anchor)
├── Cargo.toml                 # Workspace manifest (12 crates + 3 EAASP rust tools)
├── Cargo.lock
├── Makefile                   # 130 targets — see CLAUDE.md "Build & Test"
├── .adr-config.yaml           # ADR governance config
└── .env.example               # Sample env vars
```

## Directory Purposes

### `crates/` — Rust workspace

**Purpose:** All Rust source for both Leg A (EAASP integration) and Leg B (dormant Grid product).
**Contains:** 12 crate directories, each with `src/`, optional `tests/`, optional `Cargo.toml` features.
**Build order:** `grid-types` → `grid-sandbox` / `grid-engine` (parallel) → others. `grid-desktop` excluded from default workspace build.

**Key subdirectories:**

#### `crates/grid-types/`
- Purpose: Zero-dep shared type vocabulary.
- Files: `src/lib.rs` (314B), `src/error.rs` (579B), `src/execution.rs` (4.7K), `src/id.rs` (1.3K), `src/memory.rs` (15.2K), `src/message.rs` (1.9K), `src/provider.rs` (3.1K), `src/sandbox.rs` (1.1K), `src/skill.rs` (3.8K), `src/tool.rs` (5.5K).

#### `crates/grid-engine/src/`
**The largest cohesion area.** ~109K LOC across 25 subdirs:
- `agent/` — 45 modules (agent loop, harness, executor, hooks, scheduler, autonomous, collaboration, …):
  - **`harness.rs` (3,551 LOC / 154.2K)** — main agent loop, the file-length cap is waived here per CLAUDE.md.
  - `runtime.rs` (2,136 LOC / 88.3K) — `AgentRuntime` entry.
  - `executor.rs` (952 LOC), `loop_guard.rs` (886 LOC), `loop_config.rs` (632 LOC), `dual.rs` (623), `subagent_runtime.rs` (617), `parallel.rs` (549), `autonomous_trigger.rs` (532), `builtin_agents.rs` (458), `autonomous.rs` (451), `queue.rs` (406).
  - `stop_hooks.rs` (261 LOC) — Phase 2 S3.T4 stop-hook dispatch.
  - `cancellation_tree.rs` (338), `self_repair.rs` (289), `task_tracker.rs` (267), `loop_.rs` (264), `events.rs` (259), `interrupt.rs` (256), `streaming_executor.rs`, `coordinator.rs`, `continuation.rs`, `manifest_loader.rs`, `prompt_executor.rs`, `entry.rs`, `estop.rs`, `capability.rs`, `catalog.rs`, `config.rs`, `context.rs`, `deferred_action.rs`, `loop_steps.rs`, `mod.rs`, `router.rs`, `runtime_lifecycle.rs`, `runtime_mcp.rs`, `runtime_scheduler.rs`, `store.rs`, `subagent.rs`, `team.rs`, `tenant.rs`, `token_escalation.rs`, `turn_budget.rs`, `turn_gate.rs`, `yaml_def.rs`.
  - `agent/collaboration/` — Byzantine consensus + multi-agent: `consensus.rs` (16.5K), `manager.rs` (15.3K), `protocol.rs` (13.2K), `context.rs` (13.2K), `crypto.rs` (10.0K), `persistence.rs` (12.9K), `sqlite_store.rs` (20.0K), `channel.rs`, `handle.rs`, `injection.rs`, `mod.rs`.
- `hooks/` — Hook engine. `context.rs` (27.9K), `registry.rs` (14.5K), `handler.rs`, `mod.rs` (defines `HookPoint` enum: PreToolUse / PostToolUse / PreTask / PostTask / SessionStart / SessionEnd / PreCompact / PostCompact / LoopTurnStart / LoopTurnEnd / AgentRoute / SkillsActivated / SkillDeactivated / SkillScriptStarted / ToolConstraintViolated / Stop / SubagentStop / UserPromptSubmit). Subdirs: `builtin/` (`audit_log.rs`, `security_policy.rs`), `declarative/`, `policy/`, `wasm/` (`handler.rs`, `host_impl.rs`, `loader.rs`, `manifest.rs`).
- `tools/` — 51 files. Built-ins: `bash.rs` (23.0K), `bash_classifier.rs` (21.1K), `bash_guard.rs` (12.6K), `file_read.rs` (23.6K), `file_edit.rs` (4.7K), `file_write.rs` (3.1K), `web_fetch.rs` (12.8K), `web_search.rs` (17.7K), `task.rs` (17.7K), `subagent.rs` (14.2K), `notebook_edit.rs`, `lsp.rs` (13.9K), `tool_search.rs` (17.1K), `mcp_manage.rs` (16.7K), `mcp_prompt.rs`, `mcp_resource.rs`, `mcp_auth.rs`, `memory_*.rs` (10 files), `knowledge_graph.rs` (20.6K), `plan_mode.rs` (13.0K), `worktree.rs` (13.2K), `scheduler.rs` (18.7K), `session.rs` (23.0K), `team.rs` (11.8K), `todo.rs` (7.9K), `notifier.rs` (10.1K), `prompts.rs`, `interaction.rs`, `approval.rs`, `ask_user.rs`, `recorder.rs`, `traits.rs`, `truncation.rs`, `path_safety.rs`, `dev_commands.rs` (16.5K), `doctor.rs` (11.0K), `find.rs`, `glob.rs`, `grep.rs`, `cast_params.rs`, `config_tool.rs`, `input_risk.rs`, `interceptor.rs`, `rate_limiter.rs`, `send_message.rs`, `sleep.rs`, `mod.rs` (12.2K).
- `providers/` — `anthropic.rs` (24.3K), `openai.rs` (35.2K), `chain.rs` (25.8K), `pipeline.rs` (20.7K), `retry.rs` (26.9K), `error_classifier.rs` (22.6K), `smart_router.rs` (15.9K), `capabilities.rs` (14.6K), `response_cache.rs`, `usage_recorder.rs`, `metering_provider.rs`, `defaults.rs`, `config.rs`, `traits.rs`.
- `memory/` — `sqlite_store.rs` (35.1K), `vector_index.rs` (20.0K), `hybrid_query.rs` (18.1K), `auto_extractor.rs` (22.2K), `memory_injector.rs` (12.6K), `reranker.rs` (15.5K), `round_memory.rs` (12.1K), `session_summarizer.rs`, `session_summary_store.rs`, `procedural_extractor.rs`, `event_extractor.rs`, `working.rs`, `graph.rs`, `graph_store.rs`, `fts.rs`, `embedding.rs`, `injector.rs`, `semantic.rs`, `session_hook.rs`, `extractor.rs`, `budget.rs`, `sqlite_working.rs`, `traits.rs`, `store_traits.rs`, `mod.rs`.
- `context/` — `compaction_pipeline.rs` (40.0K), `system_prompt.rs` (43.3K), `pruner.rs` (20.9K), `budget.rs` (15.4K), `tool_use_summary.rs`, `observation_masker.rs`, `tiktoken_counter.rs`, `token_counter.rs`, `auto_compact.rs`, `compact_prompt.rs`, `collapse.rs`, `flush.rs`, `fork.rs`, `manager.rs`, `builder.rs`, `mod.rs`.
- `mcp/` — `manager.rs` (26.5K), `oauth.rs` (20.1K), `server.rs` (12.3K), `traits.rs` (10.9K), `storage.rs` (10.9K), `bridge.rs`, `convert.rs`, `sse.rs`, `stdio.rs`, `mod.rs`.
- `security/` — `policy.rs` (11.7K), `pipeline.rs` (19.5K), `permission_engine.rs` (11.9K), `permission_rule.rs` (11.3K), `permission_types.rs`, `tracker.rs` (3.2K — `ActionTracker`/`AutonomyLevel`/`CommandRiskLevel`), `ai_defence.rs` (17.7K), `mod.rs`.
- `sandbox/` — `router.rs` (16.7K), `docker.rs` (19.7K), `wasm.rs` (16.7K), `subprocess.rs`, `external.rs`, `session_sandbox.rs` (16.1K), `target.rs` (19.1K), `profile.rs` (14.6K), `audit.rs`, `run_mode.rs`, `traits.rs`, `mod.rs`.
- `audit/` — `storage.rs` (19.5K), `storage_test.rs` (16.7K), `mod.rs`.
- `event/` — `bus.rs`, `store.rs` (10.7K), `projection.rs`, `reconstructor.rs`, `mod.rs`.
- `session/` — `sqlite.rs` (13.8K), `thread_store.rs` (23.8K), `transcript.rs`, `events.rs`, `memory.rs`, `mod.rs`.
- `skills/` — `loader.rs` (41.9K), `selector.rs`, `dependency.rs`, `constraint.rs`, `execute_tool.rs`, `model_override.rs`, `semantic_index.rs`, `slash_router.rs`, `standards.rs` (9.6K), `trust.rs`, `tool_bridge.rs`, `runtime_bridge.rs`, `tool.rs`, `metadata.rs`, `index.rs`, `initializer.rs`, `manager.rs`, `registry.rs`, `mod.rs`.
- `auth/`, `db/`, `metering/`, `metrics/`, `secret/`, `tls/`, `sync/`, `scheduler/`, `skill_runtime/`, `storage/`, `logging/`, `commands.rs` (14.3K), `root.rs` (18.4K), `lib.rs` (3.7K).

#### `crates/grid-engine/tests/`
**94 integration test files**. Notable:
- `harness_basic.rs`, `harness_integration.rs` (19.0K), `harness_envelope_wiring_test.rs` (10.5K).
- `agent_events.rs`, `agent_events_serde.rs`, `agent_loop_config.rs`, `agent_loop_steps.rs`, `agent_parallel_partition.rs`.
- `hook_envelope_parity_test.rs` (10.8K) — cross-runtime parity.
- `hook_failure_mode.rs` (7.6K), `stop_hooks_integration.rs`.
- `assessment_*.rs` — context degradation, e-stop, memory consistency, provider failover, security adversarial, text-tool recovery (6 files).
- `byzantine_consensus.rs` (28.3K), `byzantine_persistence.rs` (13.5K), `collaboration.rs` (26.9K).
- `compaction_pipeline.rs` (28.1K), `auto_memory.rs` (13.3K), `auto_compact.rs`, `auto_snip.rs`, `context_fork.rs`, `context_manager.rs`.
- `provider_pipeline.rs` (14.2K), `retry_graduated_integration.rs` (8.3K), `stream_failover.rs`, `streaming_executor.rs`, `anthropic_prompt_caching.rs`.
- `sandbox_*.rs` (5 files: docker, router, subprocess, wasm, base), `nodejs_runtime.rs`, `shell_runtime.rs`.
- `skill_*.rs` (12 files), `skills_e2e.rs`.
- `tool_*.rs` (10 files), `tool_namespace_test.rs`, `tool_result_aggregate_spill.rs`.
- `mcp_annotations.rs`, `wasm_hook_plugin.rs`, `wasm_skill_runtime.rs`.
- `dual_agent.rs` (13.5K), `multi_session.rs`, `subagent.rs`, `coordinator_mode.rs`, `coordinator_tool_filter.rs`, `t6_t7_platform_ws_approval.rs`.
- `phase2_5_regression.rs` (6.6K), `d87_multi_step_workflow_regression.rs` (14.4K).
- `auth_config_test.rs`, `auth_middleware_test.rs`, `tls_config.rs`, `safety_pipeline.rs`, `canary_token.rs`.
- `event_backpressure_test.rs`, `error_not_persisted.rs`, `observation_masking.rs`, `offline_sync.rs` (14.1K), `session_interrupt_integration.rs`, `session_isolation.rs`.
- `smart_routing.rs`, `smart_routing_v2.rs`, `telemetry_observability.rs`, `tiktoken_counter.rs`, `token_counter.rs`.
- `unattended_retry.rs`, `turn_gate.rs`, `stop_reason.rs`, `max_tokens_continuation.rs`, `cast_params.rs`, `content_block.rs`, `deferred_action.rs`, `tool_approval.rs`, `tool_constraint.rs`, `tool_interceptor.rs`, `tool_output.rs`, `tool_risk_level.rs`, `tool_truncation.rs`, `test_grid_root.rs`.

#### `crates/grid-engine/builtin/`
Built-in command + skill bundles (templates).

#### `crates/grid-engine/wit/`
WIT (WebAssembly Interface Types) for WASM hooks/skills.

#### `crates/grid-runtime/src/`
- `main.rs` (5.4K) — gRPC server entry, port 50051.
- `service.rs` (18.9K) — `RuntimeService` 16-method impl.
- `harness.rs` (54.7K) — wraps `grid-engine::AgentRuntime` for L1 contract.
- `contract.rs` (26.0K) — contract enforcement.
- `scoped_hook_handler.rs` (18.5K) — `ScopedStopHookBridge`.
- `telemetry.rs` (12.3K), `l2_memory_client.rs` (9.5K), `session_payload.rs` (8.3K), `memory_write_hook.rs` (4.7K), `config.rs`, `l2_client.rs`, `lib.rs`.

#### `crates/grid-runtime/tests/`
5 integration files: `grpc_integration.rs` (11.2K), `harness_payload_integration.rs` (18.2K), `llm_provider_integration.rs` (5.3K), `scoped_hook_wiring_integration.rs` (23.3K), `v2_session_payload_test.rs` (6.7K).

#### `crates/grid-server/src/` (Leg B dormant)
- `main.rs` (16.1K), `config.rs` (25.8K), `router.rs`, `state.rs`, `ws.rs` (20.2K), `lib.rs`.
- `api/`, `middleware/`, `migrations/` — subdirs.

#### `crates/grid-platform/src/` (Leg B dormant)
- `main.rs` (6.5K), `lib.rs`, `agent_pool.rs` (17.1K), `ws.rs` (17.5K), `user_runtime.rs`.
- `api/`, `audit/`, `auth/`, `db/`, `middleware/`, `tenant/` — subdirs.

#### `crates/grid-cli/src/`
- `main.rs` (4.6K), `studio_main.rs` (4.8K), `lib.rs` (5.7K).
- `commands/`, `dashboard/`, `output/`, `repl/`, `tui/`, `ui/` — subdirs.

#### `crates/grid-eval/src/`
- `runner.rs` (56.3K), `scorer.rs` (69.2K), `main.rs` (52.4K), `comparison.rs` (31.6K), `benchmark.rs` (29.1K), `reporter.rs` (29.0K), `config.rs` (20.3K), `failure.rs` (19.9K), `run_store.rs` (19.6K), `mock_provider.rs` (15.6K), `recorder.rs`, `mock_tool.rs`, `model.rs`, `score.rs`, `task.rs`, `trace.rs`, `faulty_provider.rs`, `lib.rs`.
- `benchmarks/`, `datasets/`, `suites/` — subdirs.

#### `crates/grid-hook-bridge/src/`
- `lib.rs`, `traits.rs` (4.4K), `in_process.rs` (9.2K), `grpc_bridge.rs` (3.8K), `server.rs` (10.7K).

#### `crates/grid-sandbox/src/`
- `lib.rs` (32B), `native.rs` (1.9K), `traits.rs`.
- `commands/`, `skills/` — subdirs.

#### `crates/eaasp-goose-runtime/`, `crates/eaasp-claw-code-runtime/`, `crates/eaasp-scoped-hook-mcp/`
Leg A comparison runtimes + MCP proxy. Each has `src/`, `tests/`. Per `CLAUDE.md` and ADR-V2-006, `eaasp-scoped-hook-mcp` is a stdio MCP middleware that intercepts `tools/call` to inject Pre+PostToolUse hooks.

### `lang/` — non-Rust runtimes

#### `lang/claude-code-runtime-python/src/claude_code_runtime/`
- `service.py` (45.2K) — gRPC `RuntimeService` 16-method impl.
- `scoped_command_executor.py` (11.3K) — `ScopedCommandExecutor` (asyncio subprocess + envelope).
- `sdk_wrapper.py` (6.8K), `session.py` (6.0K), `mapper.py` (3.6K), `hook_executor.py` (4.4K), `hook_substitution.py` (4.3K), `l2_memory_client.py` (3.5K), `skill_loader.py` (2.8K), `state_manager.py`, `telemetry.py`, `config.py`, `__main__.py`, `__init__.py`.
- `_proto/eaasp/runtime/v2/` — generated stubs: `common_pb2.py` (7.7K), `common_pb2.pyi` (11.0K), `runtime_pb2.py` (11.4K), `runtime_pb2.pyi` (14.4K), `runtime_pb2_grpc.py` (35.2K), `hook_pb2.py` (8.5K), `hook_pb2.pyi` (11.7K), `hook_pb2_grpc.py` (11.2K). **Post-processed by `scripts/gen_runtime_proto.py` per D152** (loosens grpcio-tools enum-int validation).

#### `lang/claude-code-runtime-python/tests/`
12 test files (2026-04 set): `test_service.py` (19.7K), `test_skill_extraction_e2e.py` (12.6K), `test_sdk_wrapper.py`, `test_scoped_hooks.py`, `test_scoped_hook_executor_integration.py`, `test_l2_memory_client.py`, `test_session.py`, `test_hook_substitution.py`, `test_hook_executor.py`, `test_telemetry.py`, `test_config.py`, plus `fixtures/`.

#### `lang/nanobot-runtime-python/src/nanobot_runtime/`
- `service.py` (13.3K) — 16 gRPC methods (Phase 2.5 W2.T3).
- `session.py` (13.1K) — multi-turn loop, AsyncGenerator[AgentEvent], ADR-V2-006 envelope.
- `provider.py` (3.7K) — OpenAICompatProvider (httpx trust_env=False).
- `mcp_client.py` (5.7K), `__main__.py`, `__init__.py`.
- `_proto/` — generated stubs.

#### `lang/pydantic-ai-runtime-python/src/pydantic_ai_runtime/`
Phase 3 addition. Standard Python runtime layout (service, session, _proto).

#### `lang/ccb-runtime-ts/src/`
- `index.ts` (957B), `server.ts` (4.3K), `service.ts` (4.4K).
- `proto/` — generated TS stubs from L0 protocol.
- Phase 4a `check-ccb-types-ts-sync.sh` ensures types stay in sync.

#### `lang/hermes-runtime-python/`
**FROZEN** per ADR-V2-017. Reference only.

### `tools/` — EAASP shadow stack

#### `tools/eaasp-l4-orchestration/src/eaasp_l4_orchestration/`
**The L4 hub.**
- `session_orchestrator.py` (753 LOC) — main orchestrator. **`_chunk_type_to_wire`** (single-site mapping per ADR-V2-021); `_ALLOWED_CHUNK_TYPES` whitelist.
- `api.py` (541 LOC) — FastAPI endpoints.
- `l1_client.py` (399 LOC) — gRPC client to L1.
- `event_engine.py`, `event_stream.py` (140 LOC, SSE), `event_backend.py`, `event_backend_sqlite.py`, `event_handlers.py`, `event_models.py`, `event_interceptor.py`.
- `context_assembly.py` — 5-block SessionPayload builder.
- `mcp_resolver.py`, `handshake.py`, `db.py`, `main.py`, `__init__.py`.
- `_proto/eaasp/runtime/v2/` — generated stubs.

#### `tools/eaasp-l3-governance/src/eaasp_l3_governance/`
- `policy_engine.py` (8.8K), `api.py` (6.8K), `audit.py` (5.5K), `managed_settings.py` (3.8K), `db.py` (2.5K), `main.py`, `__init__.py`.

#### `tools/eaasp-l2-memory-engine/src/eaasp_l2_memory_engine/`
- `index.py` (18.7K) — hybrid retrieval orchestrator.
- `files.py` (13.5K) — agent_suggested → confirmed → archived.
- `vector_index.py` (10.8K), `mcp_tools.py` (9.2K), `mcp_server.py` (6.2K), `db.py` (7.0K), `anchors.py` (4.1K), `event_index.py` (3.4K), `api.py` (3.3K), `main.py`, `__init__.py`.
- `embedding/` — embedding provider implementations.

#### `tools/eaasp-skill-registry/src/`
Rust Axum: `routes.rs` (11.6K), `skill_parser.rs` (15.3K), `store.rs` (14.5K), `models.rs` (3.7K), `git_backend.rs` (2.1K), `main.rs` (1.4K), `lib.rs`.

#### `tools/eaasp-mcp-orchestrator/src/`
Rust Axum: `manager.rs` (6.2K), `routes.rs` (3.2K), `main.rs` (1.7K), `config.rs`, `lib.rs`.

#### `tools/eaasp-cli-v2/`, `tools/eaasp-certifier/`, `tools/mock-scada/`
End-user CLI, contract certifier (Rust), mock SCADA system for verification skills.

### `proto/eaasp/runtime/v2/`
- `common.proto` (7.5K) — `SessionPayload` 5-block, `ChunkType` enum.
- `runtime.proto` (8.0K) — `RuntimeService` 16-RPC.
- `hook.proto` (5.6K) — `HookEvent` oneof, `PRE_COMPACT=8`.
- `README.md` (3.3K) — runtime tier classification.

### `tests/` (top-level)
- `tests/contract/cases/` — 4 newer contract tests (chunk_type, pre-phase3 skill compat, tool conflict, tool namespace).
- `tests/contract/contract_v1/` — frozen v1.0 baseline (e2e_smoke, event_type, hook_envelope, mcp_bridge, proto_shape, skill_workflow, plus VERSION + CHANGELOG.md).
- `tests/contract/conftest.py` (26.1K) — large fixture file: real runtime spawning, `_free_port()`, mock OAI/Anthropic servers.
- `tests/contract/fixtures/`, `tests/contract/harness/` — test data + harness modules.
- `tests/contract/test_harness_smoke.py`, `tests/contract/__init__.py`, `tests/contract/pyproject.toml`, `tests/contract/.gitignore`, `tests/contract/README.md`.
- `tests/e2e/` — end-to-end Python: `test_api_contracts.py`, `test_hook_enforcement.py`, `test_hr_example.py` (4.6K), `test_session_lifecycle.py`, `test_three_modes.sh`, `test_three_way_handshake.py`, `helpers.py`, `conftest.py`. Plus `phase3/` and `fixtures/`.
- `tests/octo-workbench/` — 33 Playwright PNG screenshot fixtures.
- `tests/integration.rs` — top-level Rust integration entry.
- `tests/test_file_cache.py`, `tests/test_http_auth.py`, `tests/test_security_middleware.py`, `tests/test_url_field.py`, `tests/__init__.py`.

### `examples/skills/`
Sample skills (each has SKILL.md + scripts):
- `skill-extraction/` — Phase 2 S3.T2 meta-skill (`SKILL.md` 158 LOC + `verify_skill_draft.sh` 28 LOC + `check_final_output.sh` 14 LOC).
- `memory-confirm-test/`, `threshold-calibration/`, `transformer-calibration/`.

### `examples/`
- `examples/demo-project/` — demo project structure.
- `examples/wasm-hook-plugin/` — WASM hook example.

### `scripts/`
- `eaasp-e2e.sh` (29.7K), `verify-v2-mvp.sh` / `.py` (13.4K + 22.1K), `verify-v2-phase2.sh` / `.py` (15.3K + 23.2K), `phase2_5-e2e-verify.sh` (37.0K), `phase3-runtime-verification.sh` (10.0K), `s4t3-runtime-verification.sh` (17.3K), `dev-eaasp.sh` (25.6K), `gen_runtime_proto.py` (8.5K), `verify-dual-runtime.sh` (4.5K), `e2e-mvp.sh`, `download_swebench_lite.py`, `release-sign.sh`, `patch-ruflo-bridge.sh`, `check-ccb-types-ts-sync.sh`, `check-pyright-prereqs.sh`, `phase2_5-runtime-verification.sh` + `-checklist.md`, `test_hook_scripts.sh`.
- `scripts/eval/` — `run_cli_eval.sh`, `run_server_eval.sh`.
- `scripts/assets/` — `mvp-managed-settings.json`, `threshold-calibration-skill.md`.

### `docs/` — documentation root

#### `docs/design/EAASP/` — CURRENT authoritative
- `adrs/` — **15 active ADRs** (the SoT for decisions): `ADR-TEMPLATE.md` + `ADR-V2-{001..023}.md` (with gaps; latest accepted: V2-001/002/003/004/005/006/015/016/017/018/019/020/021/022/023). Plus `AUDIT-2026-04-19.md` (8.3K).
- `EAASP-Design-Specification-v2.0.docx` (4.2M) — primary design doc.
- `EAASP_v2_0_MVP_SCOPE.md` (25.5K), `EAASP_v2_0_EVOLUTION_PATH.md` (36.2K).
- `E2E_VERIFICATION_GUIDE.md` (25.8K), `DEFERRED_LEDGER.md` (69.1K — cross-phase debt SSOT, **preserved under GSD**).
- `L1_RUNTIME_ADAPTATION_GUIDE.md` (21.4K), `L1_RUNTIME_COMPARISON_MATRIX.md` (8.4K), `L1_RUNTIME_CANDIDATE_ANALYSIS.md` (65.5K), `L1_RUNTIME_STRATEGY.md` (17.2K), `L1_RUNTIME_TIER_SPEC_{CN,EN}.md`, `L1_RUNTIME_R{1..4}_*.md` (4 candidate evals), `L1_RUNTIME_T0_T3_COMPLETE.md`.
- `AGENT_LOOP_PATTERNS_TO_ADOPT.md`, `AGENT_LOOP_ROOT_CAUSE_ANALYSIS.md` — D87 lessons.
- `PROVIDER_CAPABILITY_MATRIX.md` — capability table for tool_choice support.
- `PHASE_3_DESIGN.md`, `PHASE1_EVENT_ENGINE_DESIGN.md`, `PHASE_0_5_ACCEPTANCE_CHECKLIST.md`.
- `GRID_CURRENT_STATE_2026-04-10.md` (79.8K), `R3_AGT_EVALUATION_MEMO.md`.
- Various `.docx` / `.pdf` strategic references.

#### `docs/design/Grid/` — CURRENT
- `GRID_PRODUCT_DESIGN.md` (29.0K), `GRID_CRATE_SPLIT_DESIGN.md` (12.4K), `GRID_UI_UX_DESIGN.md` (23.5K).
- `archive/` — older Grid design.

#### `docs/design/` (root level) — PRE-EAASP-v2 LEGACY
**Skim only for archaeology.** ~60 design docs from 2026-02 to 2026-03 — agent harness, context engineering, MCP, memory, sandbox, security, provider chain. Includes `ARCHITECTURE_DESIGN.md` (107.3K), `AGENT_CLI_DESIGN.md` (73.6K), `AGENT_SKILLS_BEST_IMPLEMENTATION_DESIGN.md` (45.4K), competitive analysis docs, etc.

#### `docs/design/AgentOS/`, `docs/design/claude-code-oss/`
Reference subdirectories.

#### `docs/dev/`
- `WORK_LOG.md` (83.7K — prepend-new-on-top).
- `MEMORY_INDEX.md` (91.5K — recent activity index).
- `.phase_stack.json` (22.8K — active phase stack, **FROZEN as of 2026-04-26 GSD cutover**).
- `.phase_stack-20260304-v1.0-release-sprint.json` — historical.
- `NEXT_SESSION_GUIDE*.md` — 5 stashed next-session pointers.

#### `docs/plans/` — **135 historical phase plans (FROZEN as of 2026-04-26)**
- `2026-{02-26..04-XX}-<topic>.md` — phase plan files.
- `.checkpoint.json` — current phase checkpoint (FROZEN).
- `.checkpoint.archive.json`, `.checkpoint-{phase2,phase2-11,harness,octo-platform-p1,pre-harness-refactor,…}.json` — archived checkpoints.
- `archive/`, `claude-code-runtime/`, `completed/` — archived plan groups.
- **Read-only after 2026-04-26 GSD cutover.**

#### `docs/main/`
Branch-specific work logs incl. `WORK_LOG.md` (100.4K), `CHECKPOINT_PLAN.md` (35.8K), `PHASE2_5_E2E_VERIFICATION_GUIDE.md` (11.0K).

#### `docs/adr/` — LEGACY generic ADRs
ADR-001 through ADR-019 (pre-EAASP-v2). Cited only when explicitly referenced (e.g. ADR-003 API_KEY_HMAC).

#### `docs/adr_archive/`, `docs/ddd/`, `docs/decisions/`, `docs/work-logs/`, `docs/found-by-sonnet4.6/`
Archive subdirectories.

### `web/` (Leg B dormant)
- `src/`, `index.html`, `package.json`, `pnpm-lock.yaml`, `tsconfig.json`, `vite.config.ts`. Tailwind/Jotai planned per Grid stack.
- `data/`, `.playwright-cli/` — local artifacts.

### `web-platform/` (Leg B dormant)
- `src/`, `index.html`, `package.json`, `tailwind.config.js`, `tsconfig.json`, `vite.config.ts`.

### `.adr-plugin/scripts/`
Vendored ADR governance scripts so CI runs without the global plugin. Refresh: `/adr:sync-scripts`. Plus `VERSION`, `README.md`.

### `.github/workflows/`
- `adr-audit.yml` (1.3K) — F1-F4 ADR lint gate.
- `phase2_5-contract.yml` (2.5K), `phase3-contract.yml` (4.3K) — contract test matrices.
- `phase4a-ccb-types-sync.yml` (1.3K) — TS types sync check.
- `container-build.yml` (3.4K), `eval-ci.yml` (4.4K), `release.yml` (1.8K).

### `.planning/codebase/`
**THIS file's home.** GSD-managed planning state. Holds `ARCHITECTURE.md`, `STRUCTURE.md`, plus future `STACK.md` / `CONVENTIONS.md` / `TESTING.md` / `INTEGRATIONS.md` / `CONCERNS.md` written by the GSD codebase mapper.

## Key File Locations

### Entry Points
- `crates/grid-runtime/src/main.rs` (5.4K) — Leg A primary L1 gRPC server.
- `crates/grid-server/src/main.rs` (16.1K) — Leg B single-user (dormant).
- `crates/grid-platform/src/main.rs` (6.5K) — Leg B multi-tenant (dormant).
- `crates/grid-cli/src/main.rs` (4.6K) + `studio_main.rs` (4.8K).
- `crates/grid-eval/src/main.rs` (52.4K) — eval driver.
- `tools/eaasp-l4-orchestration/src/eaasp_l4_orchestration/main.py` — L4 FastAPI/uvicorn.
- `tools/eaasp-l2-memory-engine/src/eaasp_l2_memory_engine/main.py` — L2 FastAPI.
- `tools/eaasp-l3-governance/src/eaasp_l3_governance/main.py` — L3 FastAPI.
- `tools/eaasp-skill-registry/src/main.rs` (1.4K), `tools/eaasp-mcp-orchestrator/src/main.rs` (1.7K).
- `lang/{claude-code,nanobot,pydantic-ai}-runtime-python/src/{pkg}/__main__.py` — Python L1 mains.
- `lang/ccb-runtime-ts/src/index.ts` — TS L1 main.

### Configuration
- `Cargo.toml` (workspace).
- `Makefile` (130 targets).
- `.adr-config.yaml` — ADR governance.
- `.env.example` — sample env. **Real `.env` gitignored.**
- `crates/grid-server/src/config.rs` (25.8K) — Leg B server config (regenerate `config.default.yaml` via `make config-gen`).
- `proto/eaasp/runtime/v2/*.proto` — wire contract.

### Core Logic
- `crates/grid-engine/src/agent/harness.rs` (3,551 LOC) — agent loop core.
- `crates/grid-engine/src/agent/runtime.rs` (2,136 LOC) — `AgentRuntime`.
- `crates/grid-engine/src/agent/stop_hooks.rs` (261 LOC) — Stop hook dispatch.
- `crates/grid-engine/src/hooks/context.rs` (27.9K) — `HookContext` envelope.
- `crates/grid-engine/src/context/compaction_pipeline.rs` (40.0K) — context compaction.
- `crates/grid-engine/src/providers/error_classifier.rs` (22.6K) — 14 `FailoverReason` variants.
- `crates/grid-runtime/src/service.rs` (18.9K) + `harness.rs` (54.7K) — gRPC RuntimeService + adapter.
- `tools/eaasp-l4-orchestration/src/eaasp_l4_orchestration/session_orchestrator.py` (753 LOC) — L4 hub.

### Testing
- `crates/grid-engine/tests/` — 94 integration files.
- `crates/grid-runtime/tests/` — 5 integration files.
- `tests/contract/` — cross-runtime contract harness.
- `tests/e2e/` — Python end-to-end tests.
- `lang/{runtime}/tests/` — per-runtime Python tests.

### Documentation references
- `CLAUDE.md` — root project instructions (authoritative).
- `docs/design/EAASP/adrs/` — 15 ADRs (decisions SoT).
- `docs/design/EAASP/DEFERRED_LEDGER.md` — cross-phase debt SSOT (preserved under GSD).
- `docs/design/EAASP/L1_RUNTIME_ADAPTATION_GUIDE.md` — how to add a new L1 runtime.
- `docs/dev/.phase_stack.json`, `docs/plans/.checkpoint.json` — phase state (FROZEN 2026-04-26).
- `docs/dev/WORK_LOG.md` — prepend-on-top work log.

## Naming Conventions

### Files

**Rust:**
- `snake_case.rs` for source files (`agent_loop.rs`, `harness.rs`, `error_classifier.rs`).
- `mod.rs` for module roots.
- Test files: same name as the unit they test (e.g. `auth_config_test.rs`, `hook_envelope_parity_test.rs`).
- Edition 2021, resolver 2.

**Python:**
- `snake_case.py` for source files (`session_orchestrator.py`, `scoped_command_executor.py`).
- `__init__.py` for package roots, `__main__.py` for executable entrypoints.
- Test files: `test_<unit>.py` (e.g. `test_skill_extraction_e2e.py`, `test_service.py`).
- Generated proto: `*_pb2.py` / `*_pb2.pyi` / `*_pb2_grpc.py` under `_proto/`.

**TypeScript:**
- `camelCase.ts` for source files (`index.ts`, `server.ts`, `service.ts`).
- Generated proto: under `proto/` subdir.

**Documentation:**
- `UPPERCASE_WITH_UNDERSCORES.md` for design docs (`L1_RUNTIME_ADAPTATION_GUIDE.md`, `DEFERRED_LEDGER.md`, `EAASP_v2_0_MVP_SCOPE.md`).
- `CLAUDE.md`, `README.md` in English (per CLAUDE.md File Organization Standards).
- Design docs (under `docs/design/`): Chinese, with English code identifiers.

**ADRs:**
- `ADR-V2-NNN-kebab-case-title.md` (e.g. `ADR-V2-023-grid-two-leg-product-strategy.md`).
- 15 active in `docs/design/EAASP/adrs/`.
- Legacy: `ADR-NNN-UPPERCASE_TITLE.md` in `docs/adr/`.

**Phase plans:**
- `docs/plans/YYYY-MM-DD-<topic>.md` (e.g. `docs/plans/2026-04-14-v2-phase2-plan.md`).
- 135 historical files, FROZEN after 2026-04-26 GSD cutover.

**Checkpoints:**
- `docs/plans/.checkpoint{,-<phase>}.json` (managed by `dev-phase-manager`, not hand-edited).

### Types and Identifiers

**Rust:**
- `PascalCase` for types (`AgentRuntime`, `HookContext`, `ToolRegistry`, `StopHookDecision`).
- `SCREAMING_SNAKE_CASE` for consts (`MAX_STOP_HOOK_INJECTIONS`).
- `snake_case` for fns + vars + module names.

**Python:**
- `PascalCase` for classes (`SessionOrchestrator`, `ScopedCommandExecutor`, `RuntimeConfig`).
- `snake_case` for fns + vars + modules.
- ASCII docstrings (no exotic unicode).

**Proto:**
- `PascalCase` for messages and enums (`SessionPayload`, `ChunkType`, `RuntimeService`).
- `snake_case` for field names.
- `SCREAMING_SNAKE_CASE` for enum variants (`PRE_TOOL_USE`, `WORKFLOW_CONTINUATION`).

### Directories

**Rust crates:** `kebab-case` (`grid-engine`, `eaasp-goose-runtime`, `eaasp-scoped-hook-mcp`).
**Python packages:** `snake_case` directory matching the import path (`eaasp_l4_orchestration`, `claude_code_runtime`).
**Cargo workspace member dirs:** `kebab-case` matching crate name.

## Where to Add New Code

### New L1 runtime adapter

**Primary code:** Rust → `crates/eaasp-<name>-runtime/src/` ; Python → `lang/<name>-runtime-python/src/<pkg>/` ; TS → `lang/<name>-runtime-ts/src/`.

**Tests:** sibling `tests/` directory + add to `tests/contract/cases/` for cross-runtime contract tests.

**Required follow-ups:**
1. Implement all 12 MUST RuntimeService methods (the 4 OPTIONAL are bonus per `proto/eaasp/runtime/v2/runtime.proto`).
2. Pass `make v2-phase3-e2e` (112 pytest cases).
3. Add to CI matrix in `.github/workflows/phase3-contract.yml`.
4. Update `docs/design/EAASP/L1_RUNTIME_COMPARISON_MATRIX.md`.
5. Read `docs/design/EAASP/L1_RUNTIME_ADAPTATION_GUIDE.md` first (21.4K reference).

### New tool (engine built-in)

**Implementation:** `crates/grid-engine/src/tools/<tool_name>.rs` — implement `Tool` trait from `tools/traits.rs`.
**Registry:** add to `tools/mod.rs` (`default_tools()` or `default_tools_with_search_priority()`).
**Tests:** `crates/grid-engine/tests/tool_<aspect>.rs` (integration) + inline `#[cfg(test)]` (unit).
**ToolLayer (ADR-V2-020):** declare layer in `RequiredTool` if used by skills.
**Sandbox profile:** if it shells out, add a profile in `sandbox/profile.rs`.

### New hook handler

**Built-in:** `crates/grid-engine/src/hooks/builtin/<name>.rs` — impl `HookHandler` trait. Register in `hooks/builtin/mod.rs`.
**Declarative (config-driven):** `crates/grid-engine/src/hooks/declarative/`.
**Policy:** `crates/grid-engine/src/hooks/policy/`.
**WASM:** `crates/grid-engine/src/hooks/wasm/`.
**Hook envelope schema (ADR-V2-006):** stdin JSON + GRID_* env vars; Rust must update `HookContext::to_json` / `to_env_vars` (D120 deferred).

### New skill

**Local skill:** `examples/skills/<skill-name>/SKILL.md` + supporting `*.sh` scripts.
**Registry skill:** load via `tools/eaasp-skill-registry/`. Schema: `frontmatter` per `skill_parser.rs` (15.3K).
**Tests:** unit in `crates/grid-engine/src/skills/loader.rs`; integration in `crates/grid-engine/tests/skill_*.rs`.

### New utility script

`scripts/<script-name>.{sh,py}` — keep one-shot scripts here, NOT at repo root.

### New EAASP shadow tool

`tools/eaasp-<name>/src/<pkg>/`. Add to `Cargo.toml` workspace members (Rust) or to `make dev-eaasp` orchestration (`scripts/dev-eaasp.sh`).

### New Leg B feature (DON'T unless ADR-V2-023 P5 met)

`crates/grid-{server,platform,desktop}/src/` or `web/` / `web-platform/`. Reviewer prompt: "is this really necessary now?". Justification required per CLAUDE.md.

## Special Directories

### `target/`
**Purpose:** Cargo build cache.
**Generated:** Yes.
**Committed:** No (gitignored).

### `.venv/`
**Purpose:** Python virtualenv for repo-root tools (e.g. `tests/contract/conftest.py` 3.12 vs `lang/claude-code-runtime-python/.venv` 3.14).
**Generated:** Yes.
**Committed:** No.

### `node_modules/` (under `web*/` and `lang/ccb-runtime-ts/`)
**Purpose:** JS dependencies.
**Generated:** Yes (via `pnpm install` for web, `bun install` for ccb-runtime-ts).
**Committed:** No.

### `.logs/latest/`
**Purpose:** Log rotation under `make dev-eaasp`.
**Generated:** Yes.
**Committed:** No.

### `eval_output/`, `data/`, `data-platform/`
**Purpose:** Evaluation outputs and local SQLite DB.
**Generated:** Yes (eval_output, data); `data-platform` may have committed seeds.
**Committed:** Mostly no.

### `docs/plans/.checkpoint*.json`, `docs/dev/.phase_stack.json`
**Purpose:** Phase state.
**Generated:** Yes (managed by `dev-phase-manager` skill — never hand-edit).
**Committed:** Yes (deliberately, for cross-session continuity).
**FROZEN as of 2026-04-26 GSD cutover** — preserved as historical record; future phase tracking moves to `.planning/`.

### `.planning/codebase/`
**Purpose:** GSD-managed codebase intelligence (this directory).
**Generated:** Yes (by `/gsd-map-codebase`).
**Committed:** Yes.

### `.adr-plugin/scripts/`
**Purpose:** Vendored ADR governance scripts so CI works without the global plugin.
**Generated:** Synced from `~/.claude/skills/adr-governance/`.
**Committed:** Yes.

### `crates/grid-engine/builtin/` and `crates/grid-engine/wit/`
**Purpose:** Built-in command bundles + WIT (WebAssembly Interface Types) for WASM hooks/skills.
**Generated:** No.
**Committed:** Yes.

### `lang/*/_proto/` and `tools/*/_proto/`
**Purpose:** Generated protobuf stubs (Python).
**Generated:** Yes (via `scripts/gen_runtime_proto.py`, post-processed per D152 to relax grpcio-tools enum-int validation).
**Committed:** Yes (so CI doesn't need codegen toolchain).

---

*Structure analysis: 2026-04-26*
