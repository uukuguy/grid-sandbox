# External Integrations

**Analysis Date:** 2026-04-26

Grid is an agent runtime stack — almost every "external integration" is mediated through one of three planes:

1. **LLM provider integrations** (Anthropic, OpenAI-compat) consumed by `crates/grid-engine/src/providers/`.
2. **MCP (Model Context Protocol)** as the canonical extension surface for tools and external systems (rmcp 1 in Rust, `mcp>=1.2` in Python).
3. **EAASP L0 contract** — gRPC `proto/eaasp/runtime/v2/{common,runtime,hook}.proto` (16-method `RuntimeService` + HookBridge) used by L2/L3/L4 to drive any compliant L1 runtime.

Per ADR-V2-023 P3, the `tools/eaasp-*/` services in this repo are **local high-fidelity SHADOWS** of the real upstream EAASP project (separate team). They are integration test fixtures, not production EAASP.

## APIs & External Services

### LLM Providers

**Anthropic API:**
- Rust client: `crates/grid-engine/src/providers/anthropic.rs` (711 LOC) — wraps `reqwest` 0.12 directly, no SDK.
- Auth: `ANTHROPIC_API_KEY` (required when `LLM_PROVIDER=anthropic`).
- Optional override: `ANTHROPIC_BASE_URL` (proxy / Anthropic-compat endpoints), `ANTHROPIC_MODEL_NAME`.
- Note: per `feedback_env_var_conventions.md`, do NOT append `/v1` to `ANTHROPIC_BASE_URL`.
- Python: `lang/claude-code-runtime-python/` — uses **`claude-agent-sdk>=0.1.0`** (Anthropic's official agent SDK, not the bare client).

**OpenAI-compat (OpenAI / OpenRouter / Azure / DeepSeek):**
- Rust client: `crates/grid-engine/src/providers/openai.rs` (998 LOC) — wraps `reqwest` 0.12.
- Auth: `OPENAI_API_KEY`, `OPENAI_BASE_URL`, `OPENAI_MODEL_NAME` (NOT `LLM_MODEL` — see `feedback_env_var_conventions.md`).
- Azure variant: `AZURE_OPENAI_API_KEY` (referenced in `crates/grid-engine/src/providers/defaults.rs`).
- Python adapters using the OpenAI-compat surface:
  - `lang/nanobot-runtime-python/` — bare `httpx` `OpenAICompatProvider` with `trust_env=False` and a strict OAI subset (bans `HTTP-Referer`, `X-Title`, provider routing per ADR-V2-006).
  - `lang/pydantic-ai-runtime-python/` — uses `pydantic-ai>=0.0.14` over OpenAI-compat endpoints.

**Provider routing/middleware (Rust):**
- `crates/grid-engine/src/providers/smart_router.rs` — multi-provider routing.
- `crates/grid-engine/src/providers/chain.rs` — provider chaining.
- `crates/grid-engine/src/providers/error_classifier.rs` — `FailoverReason` matrix (14 variants, 36 unit tests; per project memory `project_s1_t6_error_classifier.md`).
- `crates/grid-engine/src/providers/retry.rs` — graduated retry + jitter (`RetryPolicy::graduated`).
- `crates/grid-engine/src/providers/capabilities.rs` — capability matrix + Eager probe (closes D87 per project memory `project_d87_root_cause_revised.md`).
- `crates/grid-engine/src/providers/response_cache.rs`, `crates/grid-engine/src/providers/pipeline.rs`.

**LLM provider policy (`feedback_l1_runtime_llm_provider.md` / D20):**
- `grid-runtime` defaults to `OPENAI_*` env vars.
- `claude-code-runtime` only uses `ANTHROPIC_*` (Anthropic SDK is hardcoded path).

### EAASP services (local shadows of upstream EAASP)

| Service | Path | Transport | Purpose |
|---------|------|-----------|---------|
| L2 Memory Engine | `tools/eaasp-l2-memory-engine/` | FastAPI HTTP + MCP server | Versioned memory files + evidence anchors + hybrid retrieval (FTS + HNSW) |
| L2 Skill Registry | `tools/eaasp-skill-registry/` | Axum HTTP + git2 backing | Skill manifest storage + MCP tool bridge |
| L2 MCP Orchestrator | `tools/eaasp-mcp-orchestrator/` | Axum HTTP | YAML-driven MCP server lifecycle |
| L3 Governance | `tools/eaasp-l3-governance/` | FastAPI HTTP | Policy DSL + risk classification + telemetry ingest |
| L4 Orchestration | `tools/eaasp-l4-orchestration/` | FastAPI HTTP + gRPC client | Session three-way handshake + SSE event stream + intent dispatch |

**EAASP CLI** (`tools/eaasp-cli-v2/`): user-facing `eaasp` command (typer) talking to L4 over HTTPS/HTTP — entry: `eaasp_cli_v2.main:app`.

**Mock external system:** `tools/mock-scada/` — stdio MCP server simulating an industrial SCADA system, used in threshold-calibration e2e tests (`mcp>=1.2`, `starlette`, `uvicorn`).

### Runtime Contract (EAASP L0 → L1 gRPC)

**Proto location:** `proto/eaasp/runtime/v2/`:
- `common.proto` — shared message types (sessions, chunks, ChunkType enum 8 variants per ADR-V2-021).
- `runtime.proto` — 16-method `RuntimeService` (12 MUST + 4 OPTIONAL).
- `hook.proto` — `HookBridge` service + `HookEvent` (PreToolUse / PostToolUse / Stop / PreCompact per ADR-V2-018).

**16 RuntimeService methods** are implemented by all 7 L1 runtimes (per Phase 3 sign-off project memory):
- Rust: `crates/grid-runtime/`, `crates/eaasp-goose-runtime/`, `crates/eaasp-claw-code-runtime/`.
- Python: `lang/claude-code-runtime-python/`, `lang/nanobot-runtime-python/`, `lang/pydantic-ai-runtime-python/`, `lang/hermes-runtime-python/` (frozen).
- TypeScript: `lang/ccb-runtime-ts/` (Bun + `@grpc/grpc-js`).

**Contract certifier:** `tools/eaasp-certifier/` (Rust + tonic + clap) — validates 16-method gRPC compliance. Run via `make certifier-verify` / `make certifier-blindbox`.

**Contract version:** `contract-v1.1.0` (project memory). Phase 3 sign-off: 7 runtimes × 42 PASS / 22 XFAIL each.

## Data Storage

### Relational / SQLite

**Engine + server:**
- `rusqlite` 0.32 + `tokio-rusqlite` 0.6 (workspace pinned in `Cargo.toml`).
- Rust feature `bundled` — SQLite shipped statically; no system SQLite required.
- FTS5 enabled via `vtab` feature.
- Default DB path: `./data/grid.db` (env `GRID_DB_PATH`). Visible: repo root has `grid-runtime.db` (0 B, placeholder), `octo.db` (1.9 MB legacy DB), `octo.db-shm`, `octo.db-wal`.
- Optional secondary stack: `sqlx` 0.8 with `runtime-tokio`, `sqlite` (`Cargo.toml` workspace).

**EAASP services:**
- L2 memory, L3 governance, L4 orchestration use `aiosqlite>=0.20` (`tools/eaasp-l2-memory-engine/pyproject.toml` etc.).

**Skill registry storage:**
- `tools/eaasp-skill-registry/` — `rusqlite` for metadata + `git2` 0.19 for skill asset versioning.

### Vector Store

**Rust path:** `hnsw_rs` 0.3 (workspace) — gated behind `grid-engine` `hnsw` feature. ADR-V2-015 mandates HNSW in-process (no external vector DB).

**Python path (L2 memory engine):**
- `hnswlib>=0.8` + `numpy>=1.26` (`tools/eaasp-l2-memory-engine/pyproject.toml`).
- Implementation: `tools/eaasp-l2-memory-engine/src/eaasp_l2_memory_engine/vector_index.py` and `files.py`.
- Per-`embedding_model_id` directory layout, dual-write path in `files.py`.
- Hybrid retrieval (`tools/eaasp-l2-memory-engine/src/eaasp_l2_memory_engine/index.py`): `score = (w_fts*fts + w_sem*sem) * decay`, weights via `EAASP_HYBRID_WEIGHTS` env (default `0.5,0.5`), graceful degrade to keyword-only.

### Embeddings

**Provider:** Ollama (local) — `bge-m3:fp16` model (per project memory `project_s2_t1_vector_embedding.md`).
- `OllamaEmbedding` class implements an `EmbeddingProvider` Protocol.
- `MockEmbedding` for tests.
- No remote-API embedding integration in current scope.

### File Storage

**Filesystem only:**
- `GRID_GLOBAL_ROOT=~/.grid` for engine global state.
- Project / session workspaces in `examples/`, `data/`, `data-platform/`, `tools/*/tests/fixtures/`.
- Skill assets versioned via `git2` (`tools/eaasp-skill-registry/`).

**Optional file parsing (engine, behind `file-parsing` feature):**
- `calamine` 0.26 — Excel/xlsx.
- `pdf-extract` 0.7 — PDF text extraction.
- `zip` 2 — archive support.

### Caching

**In-process only:**
- `lru` 0.12 (engine) — provider response cache (`crates/grid-engine/src/providers/response_cache.rs`).
- `dashmap` 6 (workspace) — concurrent maps (sessions, hooks).

**Optional Redis** (gated by `grid-engine` `trigger-redis` feature): `redis` 0.27 with `tokio-comp`, `streams` — for trigger event bus only, NOT general caching.

## MCP (Model Context Protocol) Integration

MCP is the **primary extension surface** for tool integration.

### Rust (rmcp 1)

Workspace pin: `rmcp = { version = "1", features = ["client", "server", "transport-child-process", "transport-streamable-http-client-reqwest"] }` (`Cargo.toml`).

**Client/server in `grid-engine`:**
- `crates/grid-engine/src/mcp/server.rs` — `ServerHandler` impl (rmcp server).
- `crates/grid-engine/src/mcp/sse.rs` — `StreamableHttpClientTransport` (HTTP/SSE transport for remote MCP).
- `crates/grid-engine/src/mcp/stdio.rs` — stdio child-process transport.
- `crates/grid-engine/src/mcp/convert.rs` — type conversions.

**Scoped-Hook MCP middleware:** `crates/eaasp-scoped-hook-mcp/` — wrapping MCP middleware per ADR-V2-006 §2/§3. Intercepts `tools/call` for Pre/Post-ToolUse hooks, proxies all other JSON-RPC. Used by `eaasp-goose-runtime` (Outcome B subprocess via ACP/MCP, since Block goose is not embeddable per project memory `project_s1_w1_t0_goose_spike.md`).

### Python (`mcp>=1.2`)

- `tools/eaasp-l2-memory-engine/` — full MCP server with 7 tools: search/read/write_file/write_anchor/confirm/list/delete (entry: `eaasp_l2_memory_engine.mcp_server:run`).
- `tools/mock-scada/` — example MCP stdio server.
- `lang/hermes-runtime-python/` — uses `mcp>=1.2` directly (frozen runtime).

### TypeScript

- `lang/ccb-runtime-ts/` — uses `@grpc/grpc-js` for runtime contract; MCP wiring deferred per scaffolding status.

### MCP server registry

- Repo-root `.mcp.json` (496 B) — Claude Code MCP server definitions.
- L2 MCP Orchestrator (`tools/eaasp-mcp-orchestrator/`) — YAML-driven lifecycle.

## gRPC

**Universal transport for L1 runtime contract.**

**Rust stack:**
- `tonic` 0.12 — server + client.
- `prost` 0.13 / `prost-types` 0.13 — message types.
- `tonic-build` 0.12 — codegen in each runtime crate's `build.rs` (`crates/grid-runtime/build.rs`, `crates/grid-hook-bridge/build.rs`, `crates/eaasp-goose-runtime/build.rs`, `crates/eaasp-claw-code-runtime/build.rs`, `tools/eaasp-certifier/build.rs`).

**Python stack:**
- `grpcio` 1.62/1.70/1.80 + `grpcio-tools` + `protobuf` 5.0/5.26/6.31 (varies per package).
- Centralized codegen: `scripts/gen_runtime_proto.py` for 4 Python packages.
- Post-process: `_loosen_enum_stubs` accepts enum ints (D152 closure 2026-04-20 per ADR-V2-021).

**TS stack (`lang/ccb-runtime-ts/`):**
- `@grpc/grpc-js` ^1.10 + `@grpc/proto-loader` ^0.7.
- Hand-rolled gRPC server using `net.createServer` + HTTP/2 framing (per `lang/ccb-runtime-ts/src/index.ts` comments — minimal scaffold for contract suite).
- Hand-written enum sync: `lang/ccb-runtime-ts/src/proto/types.ts` validated by `scripts/check-ccb-types-ts-sync.sh` (D149).

**Standard gRPC ports** (see STACK.md service table): 50051 grid-runtime, 50052 claude-code, 50053 goose, 50054 nanobot, 50055 pydantic-ai, 50057 ccb.

## Authentication & Identity

### Provider auth

**LLM provider auth** (above): `ANTHROPIC_API_KEY` / `OPENAI_API_KEY` / `AZURE_OPENAI_API_KEY` via env. No OAuth flow for LLM providers.

### Server / platform auth

**`grid-server` auth (`crates/grid-server/`):**
- API key + HMAC (per ADR-003-API_KEY_HMAC reference in CLAUDE.md).
- Env: `GRID_AUTH_MODE`, `GRID_API_KEY`, `GRID_API_KEY_USER`, `GRID_HMAC_SECRET`.
- Engine-side helpers in `crates/grid-engine/src/auth/` and `crates/grid-engine/src/security/`.

**`grid-platform` auth (`crates/grid-platform/Cargo.toml`):**
- JWT: `jsonwebtoken` 9.
- Password hashing: `argon2` 0.5 + `rand` 0.8 (salt generation).
- OAuth client: `reqwest` + `urlencoding` (provider implementations live under `crates/grid-platform/src/`; `async-trait` for provider abstraction).

### Crypto primitives (engine, `crates/grid-engine/Cargo.toml`)

- AEAD: `aes-gcm` 0.10 (Secret Manager).
- KDF: `argon2` 0.5.
- Hashing: `sha2` 0.10.
- MAC: `hmac` 0.12 + `subtle` 2.5 (constant-time compare).
- Asymmetric: `ed25519-dalek` 2 with `rand_core` (release signing).
- Tokens: `jsonwebtoken` 9.
- Hygiene: `zeroize` 1.7 with `derive` (memory zeroing).
- Random: `rand` 0.8.
- TLS cert generation (optional): `rcgen` 0.13 (engine `tls` feature; `grid-server` `tls` feature; `grid-cli` `dashboard-tls` feature).
- TLS HTTP server: `axum-server` 0.7 with `tls-rustls` (server + cli, optional).
- OS keyring (optional): `keyring` 3 (engine `keyring` feature).

### Identity / IDs

- `uuid` 1.11 (`v4`) — session/agent IDs.
- `ulid` 1.1 — sortable IDs (memory anchors etc.).

## Sandbox / Code Execution

**Modular sandbox in `crates/grid-engine/src/sandbox/` (12 files):**
- `subprocess.rs` — native subprocess (default, no feature flag).
- `wasm.rs` — Wasmtime 36 + `wasmtime-wasi` 36 (gated by `sandbox-wasm` feature; uses `base64` 0.22).
- `docker.rs` — Bollard 0.18 Docker client (gated by `sandbox-docker` feature; uses `base64` 0.22).
- `external.rs`, `target.rs`, `router.rs`, `profile.rs`, `audit.rs`, `run_mode.rs`, `session_sandbox.rs`, `traits.rs`.

**Sandbox crate (`crates/grid-sandbox/`):** depends on `grid-types`, `tokio`, `async-trait` only — sandbox runtime adapters.

## Hooks System

### Hook envelope (ADR-V2-006)

Hooks invoked as subprocess with stdin JSON envelope + `GRID_*` env vars + 5-second timeout + exit-code semantics (0 = OK, 2 = inject-and-continue / deny, other = fail-open).

**Rust executor:** `crates/grid-engine/src/hooks/` (subdirs: `wasm/`, `declarative/`, `builtin/`, `policy/`).
- `crates/grid-engine/src/agent/stop_hooks.rs` (per project memory `project_s3_t4_stop_hooks.md`) — Stop hook trait + `InjectAndContinue` semantics + max re-entry cap.
- `crates/grid-runtime/src/scoped_hook_handler.rs` — `ScopedStopHookBridge` impl.

**Python executor:** `lang/claude-code-runtime-python/src/claude_code_runtime/scoped_command_executor.py` — `ScopedCommandExecutor` using `asyncio.create_subprocess_shell` with full envelope per ADR-V2-006.

**Hook MCP middleware:** `crates/eaasp-scoped-hook-mcp/` — Method A stdio MCP proxy intercepting `tools/call` for Pre/Post-ToolUse hooks; used by `eaasp-goose-runtime`.

### PreCompact hook (ADR-V2-018)

- Proto: `hook.proto` `PreCompactHook` oneof field 18.
- Rust: `crates/grid-engine/src/context/compaction_pipeline.rs` (per project memory `project_s3_t1_precompact_hook.md`) — surgical deltas T1.A-G with `CompactionContext` threading `context_window`, `reactive_summary_ratio` config field, linear summary chain, cross-compaction budget.

## CI/CD & Deployment

### CI

**GitHub Actions:**
- `.github/workflows/phase2_5-contract.yml` — 7-runtime contract matrix (per project memory `project_phase2_5_s2_s3_s4_complete.md`).
- `.github/workflows/phase3-contract.yml` — Phase 3 contract enforcement (chunk_type contract, ADR-V2-021).
- `.github/workflows/adr-audit.yml` — ADR F1-F5 frontmatter lint.

**ADR governance:**
- Vendored plugin scripts: `.adr-plugin/scripts/` (so CI runs without global plugin); refresh via `/adr:sync-scripts`.
- PreToolUse hook: `~/.claude/hooks/adr-guard.sh` (blocks edits to Accepted contract ADR `affected_modules`).

### Hosting / Deployment

**Repo-root Dockerfile** (`Dockerfile`): Rust 1.92-slim builder → Ubuntu noble runtime, exposes 3001/5180. **Stale** — references legacy `octo-server` / `octo-types` / `octo-engine` names; flagged for cleanup.

**Per-runtime Dockerfile:**
- `lang/claude-code-runtime-python/Dockerfile` — sets `CLAUDE_RUNTIME_PORT=50052`.
- `crates/eaasp-goose-runtime/` reference container template (per project memory `project_s1_w1_t2_5_dockerfile_adr019.md`, image id `fce46f95e216`, 361 MB; replaces frozen hermes-runtime).

**Compose file:** `docker-compose.yml` (1 KB) at repo root.

**Tauri desktop bundling:** `tauri-plugin-updater` 2 (`crates/grid-desktop/Cargo.toml`) — auto-update channel for Leg B desktop product (dormant).

## Webhooks & Callbacks

**Incoming webhooks:** None directly in current scope. The closest analogue is the L4 Orchestration SSE stream (`tools/eaasp-l4-orchestration/`) which is server-pushed, not webhook-style.

**Outgoing webhooks:** None directly.

**Server-Sent Events (SSE):**
- L4 orchestration emits SSE for session events (`tools/eaasp-l4-orchestration/`).
- MCP `StreamableHttp` transport in `crates/grid-engine/src/mcp/sse.rs` consumes SSE from remote MCP servers.

**WebSocket:**
- `axum` 0.8 with `ws` feature in `grid-server` and `grid-platform`.
- `web/vite.config.ts` proxies `/ws` → `ws://127.0.0.1:3001`.

## Monitoring & Observability

**Logs:**
- Rust: `tracing` 0.1 + `tracing-subscriber` 0.3 (workspace) with `env-filter`, `fmt`, `json` features.
- Format env: `GRID_LOG_FORMAT` (`pretty` or `json`), level via `GRID_LOG` (e.g. `grid_server=debug,grid_engine=debug`).
- Python: `loguru` (`lang/nanobot-runtime-python/`, `lang/pydantic-ai-runtime-python/`); standard logging elsewhere.
- Dev rotation: `make dev-eaasp` rotates logs into `.logs/latest/`.

**Error tracking:** None (no Sentry / Honeycomb integration detected).

**Metrics:**
- Engine: `crates/grid-engine/src/metrics/` (in-process counters).
- Metering: `crates/grid-engine/src/metering/` + `crates/grid-engine/src/providers/metering_provider.rs` + `usage_recorder.rs` (provider token usage tracking).

**Audit trail:**
- `crates/grid-engine/src/audit/` — security/audit log abstractions.
- `crates/grid-engine/src/sandbox/audit.rs` — sandbox audit events.

## Environment Configuration

**Env-var precedence (CLAUDE.md):** `config.yaml` < `.env` (gitignored) < CLI args < shell env vars.

**Critical env vars (LLM):**
- `LLM_PROVIDER` — `anthropic` (default) or `openai`.
- `ANTHROPIC_API_KEY` / `ANTHROPIC_BASE_URL` / `ANTHROPIC_MODEL_NAME`.
- `OPENAI_API_KEY` / `OPENAI_BASE_URL` / `OPENAI_MODEL_NAME` (NOT `LLM_MODEL`).
- `AZURE_OPENAI_API_KEY` (Azure path, `crates/grid-engine/src/providers/defaults.rs`).

**Critical env vars (server):** `GRID_HOST`, `GRID_PORT`, `GRID_DB_PATH`, `GRID_GLOBAL_ROOT`, `GRID_LOG`, `GRID_LOG_FORMAT`, `GRID_AUTH_MODE`, `GRID_API_KEY`, `GRID_API_KEY_USER`, `GRID_HMAC_SECRET`, `GRID_CORS_ORIGINS`, `GRID_CORS_STRICT`, `GRID_HOOKS_FILE`, `GRID_POLICIES_FILE`, `GRID_ENABLE_EVENT_BUS`, `GRID_MAX_BODY_SIZE`.

**Critical env vars (EAASP / runtimes):**
- `EAASP_DEPLOYMENT_MODE` — `shared` (default) / `per_session` (ADR-V2-019).
- `EAASP_PROMPT_EXECUTOR`.
- `EAASP_L2_DB_PATH` (auto-injected by nanobot when memory MCP needs it).
- `EAASP_TOOL_FILTER` (deprecated → ADR-V2-020 skill-declared filter).
- `EAASP_HYBRID_WEIGHTS` — L2 hybrid retrieval weights, default `0.5,0.5`.
- `CLAUDE_RUNTIME_PORT` (50052), `NANOBOT_RUNTIME_PORT` (50054), `PYDANTIC_AI_RUNTIME_PORT` (50055), `CCB_RUNTIME_GRPC_ADDR` (`0.0.0.0:50057`), `GOOSE_RUNTIME_GRPC_ADDR`, `GRID_RUNTIME_GRPC_ADDR`.
- `GOOSE_BIN` (path to goose binary, falls back to `which`).

**Secrets handling:**
- `.env` at repo root (gitignored, presence noted, **never read by analysis tools**).
- `.env.example` (804 B) provides template.
- All runtime-time secret access goes through `crates/grid-engine/src/secret/` (AES-GCM + Argon2 + zeroize) and optional OS `keyring` 3.

**Behavioral rule (CLAUDE.md / `feedback_no_fallback.md`):** missing required config errors out — do NOT silently fall back to placeholder values.

## Subprocess / External Binary Integrations

**`goose`:** `crates/eaasp-goose-runtime/` invokes the Block goose binary via subprocess + ACP/MCP per project memory `project_s1_w1_t0_goose_spike.md`. `which` 6 used for binary discovery; `GOOSE_BIN` env override. Crate intentionally has no `goose` Rust dep — Outcome B subprocess approach.

**`claw-code`:** `crates/eaasp-claw-code-runtime/` — UltraWorkers subprocess bridge using `which` 6 + `tokio` `process` feature.

**Generic shell:** `crates/grid-engine/src/tools/bash.rs` — sandboxed bash tool with `PASSTHROUGH_ENV_VARS` whitelist (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `OPENAI_BASE_URL`, ...).

**Cron:** `cron` 0.15 (engine, scheduler) — internal scheduling, not OS cron integration.

**Filesystem watchers:** `notify` 7 + `notify-debouncer-mini` 0.5 (workspace) — config / skill hot-reload.

## Internal MCP Tools

**L2 memory engine MCP tools** (`tools/eaasp-l2-memory-engine/src/eaasp_l2_memory_engine/mcp_tools.py`):
1. `memory_search` — hybrid retrieval.
2. `memory_read` — file content fetch.
3. `memory_write_file` — versioned write.
4. `memory_write_anchor` — evidence anchor.
5. `memory_confirm` — state-machine transition.
6. `memory_list` — paginated listing.
7. `memory_delete` — tombstone.

**Skill registry tools** (`tools/eaasp-skill-registry/`): exposes skill manifest CRUD via REST + MCP bridge.

---

*Integration audit: 2026-04-26*
