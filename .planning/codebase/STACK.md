# Technology Stack

**Analysis Date:** 2026-04-26

Brand: **Grid** (working repo `grid-sandbox`). Rust-centric agent-runtime stack with two product legs governed by ADR-V2-023:

- **Leg A (active, primary):** EAASP integration — `grid-engine` + `grid-runtime` exposed as L1 over gRPC.
- **Leg B (dormant):** Grid independent product — `grid-server` / `grid-platform` / `grid-desktop` + `web/` / `web-platform/`.

The repo additionally hosts **6 comparison L1 runtimes** (Python + TS + Rust) and **9 EAASP layer-2/3/4 shadow tools** used as a high-fidelity test harness; per ADR-V2-023 P3 these are local shadows, not production EAASP.

## Languages

**Primary:**
- **Rust** edition `2021`, `rust-version = "1.75"`, `resolver = "2"` — workspace defined at `Cargo.toml`. Source totals span the 13 workspace crates (~178K LOC per project memory).
- **Python** `>=3.12` — runtime services and L1 adapters. Repo-root `.python-version` pins `3.12`. Some adapter `pyproject.toml` files allow `>=3.11` for legacy hermes (`lang/hermes-runtime-python/pyproject.toml`).

**Secondary:**
- **TypeScript** `~5.4` (ccb runtime) and `~5.7` (frontends) — `lang/ccb-runtime-ts/` (Bun runtime) plus dormant `web/` and `web-platform/` Vite apps.
- **Protobuf 3** — `proto/eaasp/runtime/v2/{common,runtime,hook}.proto` (3 files, package `eaasp.runtime.v2`).
- **Bash** — `scripts/*.sh` (>15 scripts, e.g. `scripts/eaasp-e2e.sh`, `scripts/phase2_5-e2e-verify.sh`, `scripts/dev-eaasp.sh`).
- **YAML / TOML** — config, ADR frontmatter, pyproject, Cargo manifests.

## Runtime

**Rust async runtime:**
- `tokio` 1.42 with `features = ["full"]` (workspace-pinned in `Cargo.toml`).
- `tokio-stream` 0.1, `futures-util` 0.3, `async-stream` 0.3.
- `async-trait` 0.1.

**Python runtime:**
- CPython 3.12+ (some test caches under 3.14 observed via `__pycache__/*.cpython-314.pyc`).
- `asyncio` (`asyncio_mode = "auto"` in every Python `pyproject.toml`).
- `anyio` 4.x for the runtimes that wrap async SDKs (`lang/claude-code-runtime-python/pyproject.toml`, `lang/nanobot-runtime-python/pyproject.toml`, `lang/pydantic-ai-runtime-python/pyproject.toml`).

**TS runtime:**
- **Bun** (test runner + dev runner). `bun run src/index.ts` and `bun test` from `lang/ccb-runtime-ts/package.json`.

**Frontend dev runtime (Leg B, dormant):**
- Node 20 implied by Dockerfile FE stage. Vite 6 dev server.

**Package managers:**
- **Cargo** — workspace lockfile `Cargo.lock` (252.5 KB).
- **`uv`** — Python package manager per CLAUDE.md and global feedback `feedback_env_var_conventions.md`. Per-package `uv.lock` files (e.g. `tools/mock-scada/uv.lock`).
- **`npm`** for `web/` (lockfile `package-lock.json`) and **`pnpm`** for the FE Docker build path (`web/pnpm-lock.yaml`). `web-platform/` has both `package-lock.json` and `pnpm-lock.yaml`.
- **Bun** for `lang/ccb-runtime-ts/` (no lockfile-as-source-of-truth declared yet).

## Frameworks

**Rust core:**
- **Web/HTTP:** `axum` 0.8 (with `ws`), `axum-extra` 0.10 (`typed-header`), `tower` 0.5, `tower-http` 0.6 (`cors`, `trace`, `limit`, `timeout`).
- **gRPC:** `tonic` 0.12 + `prost` 0.13 + `prost-types` 0.13. Build via `tonic-build` 0.12 in `crates/grid-runtime/build.rs`, `crates/grid-hook-bridge/build.rs`, `crates/eaasp-goose-runtime/build.rs`, `crates/eaasp-claw-code-runtime/build.rs`, `tools/eaasp-certifier/build.rs`.
- **MCP SDK:** `rmcp` 1 (workspace pin) with `client`, `server`, `transport-child-process`, `transport-streamable-http-client-reqwest`. Imported in `crates/grid-engine/src/mcp/sse.rs` (StreamableHttpClient), `crates/grid-engine/src/mcp/server.rs` (ServerHandler), `crates/grid-engine/src/mcp/stdio.rs`, `crates/grid-engine/src/mcp/convert.rs`.
- **HTTP client:** `reqwest` 0.12 (`stream`, `json`, `blocking` for engine).
- **TUI / desktop:** `ratatui` 0.29 (gated under `grid-cli` `studio` feature), `crossterm` 0.28, `tauri` 2 + `tauri-plugin-shell` / `tauri-plugin-single-instance` / `tauri-plugin-updater` (`crates/grid-desktop/Cargo.toml`).
- **CLI:** `clap` 4.5 + `clap_complete`, `dialoguer` 0.11, `rustyline` 17 (REPL), `termimad` 0.31, `indicatif` 0.17, `owo-colors` 4.

**Python frameworks:**
- **FastAPI** `>=0.115` + **uvicorn** `>=0.30` + **starlette** `>=0.36` — used by EAASP shadow tools `tools/eaasp-l2-memory-engine/`, `tools/eaasp-l3-governance/`, `tools/eaasp-l4-orchestration/`.
- **Pydantic** `>=2.5–2.8` — schemas across all Python tools and runtimes.
- **MCP** `mcp>=1.2` — used by `tools/eaasp-l2-memory-engine/`, `tools/mock-scada/`, `lang/hermes-runtime-python/`.
- **gRPC:** `grpcio>=1.62/1.70/1.80` + `grpcio-tools` + `protobuf>=5.0/5.26/6.31` (varies per package).
- **CLI:** `typer>=0.12` + `rich>=13` (`tools/eaasp-cli-v2/pyproject.toml`).
- **Logging:** `loguru` (`lang/nanobot-runtime-python/pyproject.toml`, `lang/pydantic-ai-runtime-python/pyproject.toml`).
- **HTTP:** `httpx>=0.27` (most Python services), `respx>=0.21` (test fakes).
- **Per-runtime SDK:**
  - `lang/claude-code-runtime-python/`: `claude-agent-sdk>=0.1.0` (Anthropic SDK wrapper).
  - `lang/pydantic-ai-runtime-python/`: `pydantic-ai>=0.0.14`.
  - `lang/nanobot-runtime-python/`: bare `httpx` + custom `OpenAICompatProvider`.

**Frontend frameworks (Leg B, dormant):**
- `web/package.json`: React 19 + react-dom 19, Jotai 2.16, lucide-react 0.469, Tailwind CSS 4 (`@tailwindcss/vite`), Vite 6, TypeScript 5.7, react-markdown 10 + rehype-highlight 7 + remark-gfm 4, highlight.js 11, class-variance-authority 0.7, clsx 2, tailwind-merge 3.
- `web-platform/package.json`: React 19, react-router-dom 7, Jotai 2.16, Tailwind 4, Vite 6, TypeScript 5.7.

**Testing frameworks:**
- Rust: built-in `#[test]` + `#[tokio::test]`. Dev-deps: `tempfile` 3, `tower` (util), `http-body-util` 0.1.
- Python: `pytest>=8.0` + `pytest-asyncio>=0.24` (every Python project). Some add `respx>=0.21`, `pytest-httpx>=0.32`.
- TS: `bun test` (built-in).

## Workspace crate inventory

Source of truth: `Cargo.toml` `[workspace] members = ["crates/*", "tools/eaasp-certifier", "tools/eaasp-mcp-orchestrator", "tools/eaasp-skill-registry"]` and `default-members = [...]`. `grid-desktop` is intentionally excluded from default build; build with `cargo build -p grid-desktop`.

| Crate | Path | Leg | Role | Notes |
|-------|------|-----|------|-------|
| `grid-types` | `crates/grid-types/` | Shared | Zero-dep type definitions (messages, tools, sessions, sandbox, IDs, errors) | `serde`, `thiserror`, `uuid`, `chrono`, `ulid`, `serde_yaml` only |
| `grid-sandbox` (crate) | `crates/grid-sandbox/` | Shared | Sandbox runtime adapters (native subprocess primary, optional wasm/docker via features) | Name collides with repo name — distinct concept |
| `grid-engine` | `crates/grid-engine/` | Shared | Core engine — agent loop, providers, MCP, memory L0/L1/L2, tools, skills, security, audit, metering, sandbox routing, scheduler | 30+ optional features (`full`, `sandbox-wasm`, `sandbox-docker`, `file-parsing`, `keyring`, `hnsw`, `tiktoken`, `trigger-redis`, `tls`) |
| `grid-hook-bridge` | `crates/grid-hook-bridge/` | Shared | Hook event bridge between Rust and L2/L3 | tonic + tokio + dashmap |
| `grid-runtime` | `crates/grid-runtime/` | A primary / B in-process | L1 runtime adapter wrapping `grid-engine`. Leg A exposes via gRPC; Leg B uses in-process | Depends on `eaasp-skill-registry` by path |
| `grid-cli` | `crates/grid-cli/` | A aux / B primary | CLI binary `grid`; secondary bin `grid-studio` (gated by `studio` feature) | clap, dialoguer, rustyline, ratatui (Studio) |
| `grid-eval` | `crates/grid-eval/` | A aux / B primary | Evaluation harness — suites, scorers, benchmarks | binary `grid-eval` |
| `grid-server` | `crates/grid-server/` | **B only (dormant)** | Single-user workbench HTTP/WS server | Axum 0.8 + axum-extra; optional `tls` feature via `axum-server` 0.7 + rustls |
| `grid-platform` | `crates/grid-platform/` | **B only (dormant)** | Multi-tenant platform server | Axum + `jsonwebtoken` 9 + `argon2` 0.5 + OAuth via `reqwest` |
| `grid-desktop` | `crates/grid-desktop/` | **B only (dormant)** | Tauri 2 desktop app | Excluded from default build (`tauri-build`, plugins for shell/single-instance/updater) |
| `eaasp-goose-runtime` | `crates/eaasp-goose-runtime/` | A | L1 adapter for Block goose (Outcome B subprocess via ACP/MCP) | `which`, no `goose` dep — uses `eaasp-scoped-hook-mcp` as MCP middleware |
| `eaasp-claw-code-runtime` | `crates/eaasp-claw-code-runtime/` | A | L1 adapter for claw-code (UltraWorkers subprocess bridge) | tonic + which |
| `eaasp-scoped-hook-mcp` | `crates/eaasp-scoped-hook-mcp/` | A | stdio MCP proxy injecting Pre/Post-ToolUse hooks per ADR-V2-006 | Binary; tokio process+io-util only |

**Build order:** `grid-types` → (`grid-sandbox`, `grid-engine`) → everything else (cargo workspace handles automatically).

## Major Rust dependencies (workspace-pinned)

From `[workspace.dependencies]` in `Cargo.toml`:

| Category | Crate@Version | Features |
|----------|---------------|----------|
| Async | `tokio` 1.42 | `full` |
| Async | `tokio-stream` 0.1 | — |
| Async | `futures-util` 0.3 | — |
| Async | `async-stream` 0.3 | — |
| Async | `async-trait` 0.1 | — |
| Web | `axum` 0.8 | `ws` |
| Web | `axum-extra` 0.10 | `typed-header` |
| Web | `tower` 0.5 | — |
| Web | `tower-http` 0.6 | `cors`, `trace`, `limit`, `timeout` |
| HTTP | `reqwest` 0.12 | `stream`, `json` (engine adds `blocking`) |
| Serde | `serde` 1.0 | `derive` |
| Serde | `serde_json` 1.0 | — |
| Serde | `serde_yaml` 0.9 | — |
| Serde | `toml` 0.8 | — |
| Serde | `bincode` 1.3 | embedding blob storage |
| Errors | `anyhow` 1.0 | — |
| Errors | `thiserror` 2.0 | — |
| IDs | `uuid` 1.11 | `v4` |
| IDs | `ulid` 1.1 | `serde` |
| Time | `chrono` 0.4 | `serde` |
| Logging | `tracing` 0.1 | — |
| Logging | `tracing-subscriber` 0.3 | `env-filter`, `fmt`, `json` |
| Env | `dotenvy` 0.15 | — |
| Concurrency | `dashmap` 6 | — |
| DB | `rusqlite` 0.32 | `bundled`, `vtab` (FTS5) |
| DB | `tokio-rusqlite` 0.6 | — |
| DB | `sqlx` 0.8 | `runtime-tokio`, `sqlite` |
| FS | `notify` 7 + `notify-debouncer-mini` 0.5 | — |
| MCP | `rmcp` 1 | `client`, `server`, `transport-child-process`, `transport-streamable-http-client-reqwest` |
| Vector | `hnsw_rs` 0.3 | optional via `hnsw` feature on engine |
| Paths | `dirs` 6 | — |
| TUI | `ratatui` 0.29 | `unstable-rendered-line-info` |
| URL | `urlencoding` 2.1 | — |
| Bytes | `bytes` 1.0 | — |

**Engine-only direct deps (`crates/grid-engine/Cargo.toml`):**
- Crypto: `aes-gcm` 0.10, `argon2` 0.5, `ed25519-dalek` 2 (`rand_core`), `sha2` 0.10, `hmac` 0.12, `subtle` 2.5, `jsonwebtoken` 9.
- Secret hygiene: `zeroize` 1.7 (`derive`), `rand` 0.8.
- Misc: `glob` 0.3, `cron` 0.15, `regex` 1, `lru` 0.12, `include_dir` 0.7, `flate2` 1, `url` 2, `hex` 0.4, `keyring` 3 (optional).
- Optional heavy: `wasmtime` 36 + `wasmtime-wasi` 36 (sandbox-wasm), `bollard` 0.18 (sandbox-docker), `base64` 0.22, `calamine` 0.26 (xlsx), `pdf-extract` 0.7, `zip` 2 (`deflate`), `tiktoken-rs` 0.6, `rcgen` 0.13 (tls), `redis` 0.27 (`tokio-comp`, `streams`).

**Notable per-crate deps:**
- `grid-platform`: `jsonwebtoken` 9, `argon2` 0.5, `rand` 0.8, `reqwest`, `urlencoding`.
- `grid-cli`: `clap` 4.5 + `clap_complete` 4.5, `dialoguer` 0.11, `rustyline` 17 (`with-file-history`), `termimad` 0.31, `indicatif` 0.17, `owo-colors` 4, `directories` 6; Studio extras: `crossterm` 0.28, `ratatui`, `axum`, `unicode-width` 0.2, `ignore` 0.4, optional `axum-server` 0.7 (`tls-rustls`) + `rcgen` 0.13.
- `grid-desktop`: `tauri` 2 (`tray-icon`), `tauri-plugin-{single-instance,shell,updater}` 2.
- `eaasp-goose-runtime`: `which` 6 (locating `goose` binary), `tokio` (`process`).
- `eaasp-claw-code-runtime`: `which` 6, `tokio` (`process`).
- `eaasp-skill-registry` (Rust, `tools/eaasp-skill-registry/Cargo.toml`): `axum`, `git2` 0.19, `clap` 4.
- `eaasp-mcp-orchestrator` (Rust, `tools/eaasp-mcp-orchestrator/Cargo.toml`): `axum`, `clap` 4, `serde_yaml`.
- `eaasp-certifier` (Rust, `tools/eaasp-certifier/Cargo.toml`): tonic + clap; gRPC contract certifier for the 16-method RuntimeService.

## Python runtimes (lang/)

5 Python runtimes — 4 active comparison adapters, 1 frozen (per ADR-V2-017).

| Runtime | Path | Core SDK | Build backend | Status |
|---------|------|----------|---------------|--------|
| `claude-code-runtime` | `lang/claude-code-runtime-python/pyproject.toml` | `claude-agent-sdk>=0.1.0` (Anthropic) + `grpcio` 1.70 + `pydantic` 2 + `anyio` 4 + `python-dotenv` 1 | hatchling | Active — wraps Anthropic SDK |
| `nanobot-runtime` | `lang/nanobot-runtime-python/pyproject.toml` | bare `httpx>=0.27` + custom OpenAI-compat provider; `pydantic` 2.5+, `loguru`, `anyio` 4, `grpcio` 1.70 | hatchling | Active — minimal-real OpenAI-compat runtime |
| `pydantic-ai-runtime` | `lang/pydantic-ai-runtime-python/pyproject.toml` | `pydantic-ai>=0.0.14`, `pydantic` 2.5+, `loguru`, `anyio` 4, `grpcio` 1.70 | hatchling | Active — pydantic-ai over OpenAI-compat |
| `hermes-runtime` | `lang/hermes-runtime-python/pyproject.toml` | `httpx>=0.27`, `mcp>=1.2`, `grpcio` 1.80, `protobuf` 6.31 | setuptools | **FROZEN per ADR-V2-017** — fork+grpc+monkey-patch issues; replaced by goose+nanobot |
| (referenced) `claude-code-runtime-python` Dockerfile | `lang/claude-code-runtime-python/Dockerfile` | — | — | Sets `CLAUDE_RUNTIME_PORT=50052` |

**TypeScript runtime:**
- `lang/ccb-runtime-ts/package.json` — Bun + TypeScript 5.4. Deps: `@grpc/grpc-js` ^1.10, `@grpc/proto-loader` ^0.7. Devs: `typescript` ^5.4, `@types/bun`. Entry: `src/index.ts`. Server source: `src/server.ts`, service: `src/service.ts`, hand-rolled enum sync to proto: `src/proto/types.ts` (validated by `scripts/check-ccb-types-ts-sync.sh`). Default gRPC: `0.0.0.0:50057`.

## EAASP Tools (tools/)

9 directories — 4 Python services + 4 Rust tools/crates + 1 mock service. Per ADR-V2-023 P3, these are **local high-fidelity shadows** of the upstream EAASP project.

| Tool | Path | Language | Build backend | Role |
|------|------|----------|---------------|------|
| `eaasp-l2-memory-engine` | `tools/eaasp-l2-memory-engine/pyproject.toml` | Python | hatchling | L2 memory: FastAPI + aiosqlite + `hnswlib>=0.8` + `numpy>=1.26` + MCP server. Two scripts: `eaasp-l2-memory-engine` (main API) and `eaasp-l2-memory` (MCP server) |
| `eaasp-skill-registry` | `tools/eaasp-skill-registry/Cargo.toml` | Rust | cargo | Skill manifest storage (axum + git2 0.19 + rusqlite); referenced by-path from `crates/grid-runtime/` |
| `eaasp-mcp-orchestrator` | `tools/eaasp-mcp-orchestrator/Cargo.toml` | Rust | cargo | YAML-driven MCP server lifecycle (axum + serde_yaml + clap) |
| `eaasp-l3-governance` | `tools/eaasp-l3-governance/pyproject.toml` | Python | hatchling | Policy DSL + risk classification (FastAPI + aiosqlite + httpx) |
| `eaasp-l4-orchestration` | `tools/eaasp-l4-orchestration/pyproject.toml` | Python | hatchling | Session orchestration + SSE streaming + governance gates (FastAPI + grpcio 1.62 + aiosqlite + respx for tests) |
| `eaasp-cli-v2` | `tools/eaasp-cli-v2/pyproject.toml` | Python | hatchling | End-user CLI (`eaasp` script via `typer>=0.12` + `rich>=13` + `httpx`); ruff configured `line-length=100`, `target-version="py312"` |
| `eaasp-certifier` | `tools/eaasp-certifier/Cargo.toml` | Rust | cargo | gRPC contract certifier for 16-method RuntimeService (tonic + clap) |
| `mock-scada` | `tools/mock-scada/pyproject.toml` | Python | hatchling | Stdio MCP server for threshold-calibration e2e (mcp + starlette + uvicorn) |
| `archive/` | `tools/archive/` | mixed | — | Archived legacy tools |

## Configuration

**Priority** (lowest → highest, per CLAUDE.md): `config.yaml` < `.env` (gitignored) < CLI args < shell env vars.

**Generation:** `make config-gen` regenerates `config.default.yaml` from `crates/grid-server/src/config.rs` (`cargo run -p grid-server -- config-gen > config.yaml`).

**Existing config files at repo root:**
- `Cargo.toml` (workspace) — Rust manifest.
- `Makefile` — 130+ targets, primary build orchestration.
- `config.default.yaml` (7.8 KB) — generated default; do NOT hand-edit.
- `config.yaml` (3.3 KB) — actual local config.
- `.env` — local env (gitignored, presence noted; **never read**).
- `.env.example` (804 B) — template for local setup.
- `.python-version` — pins `3.12`.
- `.adr-config.yaml` — ADR plugin config.
- `.mcp.json` — MCP server entry config (496 B).
- `pyproject.toml` (158 B) — root stub (most Python projects own their own pyproject).
- `package.json` (158 B) — root stub.
- `pyrightconfig.json` (2.9 KB) — multi-package per-venv pyright config (`scripts/check-pyright-prereqs.sh` validates 9 venvs exist; D155 alerts on fallback to root `.venv`).
- `Cargo.lock`, `package-lock.json`, `uv.lock` — lockfiles.

**Compiler profiles (Cargo.toml):**
- `[profile.dev]`: `split-debuginfo = "unpacked"` (macOS link speedup), `codegen-units = 16`, `incremental = true`.
- `[profile.dev.package."*"]`: `opt-level = 1`, `codegen-units = 16` (third-party deps compile once).

**Cargo features (selectable):**
- Engine: `default = ["tiktoken"]`, `full = ["sandbox-wasm", "sandbox-docker", "file-parsing"]`. Individual: `keyring`, `hnsw`, `trigger-redis`, `tls`.
- Server / Runtime: `full`, `sandbox-wasm`, `sandbox-docker`, `tls`.
- CLI: `studio`, `full`, `dashboard-tls`.

## Environment variables (project conventions)

Per `feedback_env_var_conventions.md` and `feedback_no_fallback.md`: do NOT invent variable names; missing required keys must error out, never fall back.

**LLM access (read by `crates/grid-engine/src/providers/config.rs` and `smart_router.rs`):**
- `LLM_PROVIDER` — `"anthropic"` (default) or `"openai"`.
- `ANTHROPIC_API_KEY`, `ANTHROPIC_BASE_URL`, `ANTHROPIC_MODEL_NAME`.
- `OPENAI_API_KEY`, `OPENAI_BASE_URL`, `OPENAI_MODEL_NAME` (NOT `LLM_MODEL`).
- `AZURE_OPENAI_API_KEY` (referenced in `crates/grid-engine/src/providers/defaults.rs`).

**Server (`crates/grid-server/src/config.rs`):**
- `GRID_HOST` (default 127.0.0.1), `GRID_PORT` (3001).
- `GRID_DB_PATH` (default `./data/grid.db`).
- `GRID_LOG`, `GRID_LOG_FORMAT` (`pretty` or `json`).
- `GRID_CORS_STRICT`, `GRID_CORS_ORIGINS`.
- `GRID_HOOKS_FILE`, `GRID_POLICIES_FILE`.
- `GRID_GLOBAL_ROOT` (default `~/.grid`), `GRID_AUTH_MODE`, `GRID_API_KEY`, `GRID_API_KEY_USER`, `GRID_HMAC_SECRET`, `GRID_ENABLE_EVENT_BUS`, `GRID_MAX_BODY_SIZE`.

**EAASP runtime / shadow tools:**
- `EAASP_DEPLOYMENT_MODE` — `shared` (default) or `per_session` (ADR-V2-019). Read in `lang/nanobot-runtime-python/src/nanobot_runtime/service.py`, `lang/pydantic-ai-runtime-python/src/pydantic_ai_runtime/service.py`, `lang/ccb-runtime-ts/src/index.ts`.
- `EAASP_PROMPT_EXECUTOR`.
- `EAASP_L2_DB_PATH` — auto-injected by nanobot runtime when memory MCP server requires it.
- `EAASP_TOOL_FILTER` (deprecated; superseded by skill-declared filter per ADR-V2-020).
- `EAASP_HYBRID_WEIGHTS` — comma-separated weights for L2 hybrid retrieval, default `0.5,0.5` (`tools/eaasp-l2-memory-engine/src/eaasp_l2_memory_engine/index.py`).

**gRPC ports (do NOT hardcode — use config):**

| Port | Service | Source |
|------|---------|--------|
| 3001 | `grid-server` HTTP | `GRID_PORT` / `config.yaml` |
| 5180 | Vite dev server | `web/vite.config.ts` |
| 50051 | `grid-runtime` gRPC | runtime config |
| 50052 | `claude-code-runtime` gRPC | `CLAUDE_RUNTIME_PORT` (`lang/claude-code-runtime-python/src/claude_code_runtime/config.py`) |
| 50053 | `goose-runtime` gRPC | runtime config; `GOOSE_RUNTIME_GRPC_ADDR` for client |
| 50054 | `nanobot-runtime` gRPC | `NANOBOT_RUNTIME_PORT` (`lang/nanobot-runtime-python/src/nanobot_runtime/__main__.py`) |
| 50055 | `pydantic-ai-runtime` gRPC | `PYDANTIC_AI_RUNTIME_PORT` (`lang/pydantic-ai-runtime-python/src/pydantic_ai_runtime/__main__.py`) |
| 50057 | `ccb-runtime` gRPC | `CCB_RUNTIME_GRPC_ADDR` (`lang/ccb-runtime-ts/src/index.ts`) |

## Build orchestration

Primary entry: `Makefile` at repo root, 130+ targets. Run `make help` for complete listing.

**Key target groups:**
- Setup: `setup` (cd web && npm install).
- Dev loops: `dev` (server + web -j2), `dev-eaasp` (full 4+ EAASP services with rotation under `.logs/latest/`, via `scripts/dev-eaasp.sh`), `dev-eaasp-stop`, `server`, `web`.
- Build: `check`, `build`, `build-full`, `release`, `all`, `build-eaasp-all`.
- Tests (targeted): `test`, `test-types`, `test-engine`, `test-sandbox`, `test-server`, `claude-runtime-test`, `goose-runtime-*`.
- Quality: `fmt`, `fmt-check`, `lint`, `web-check`, `web-lint`, `check-pyright-prereqs`, `check-ccb-types-ts-sync`.
- Verification: `verify`, `verify-runtime`, `verify-dual-runtime`.
- EAASP E2E: `v2-mvp-e2e`, `v2-phase2-e2e`, `v2-phase2-e2e-full`, `v2-phase3-e2e`, `v2-phase3-e2e-rust`.
- CLI: `cli`, `cli-ask`, `cli-session`, `cli-config`, `cli-doctor`, `studio-tui`, `studio-dashboard`.
- Containers: `claude-runtime-build`, `goose-runtime-container-build`, `goose-runtime-container-verify-f1`.

**Repo-root Dockerfile** (`Dockerfile`): multi-stage Rust 1.92-slim → Ubuntu noble. **Stale** — references `octo-server` / `octo-types` / `octo-engine` (pre-rename), exposes 3001/5180. Cleanup item in CLAUDE.md.

**Per-runtime Dockerfile:** `lang/claude-code-runtime-python/Dockerfile` (sets `CLAUDE_RUNTIME_PORT=50052`); goose runtime container template per project memory `project_s1_w1_t2_5_dockerfile_adr019.md` (361 MB image, image id `fce46f95e216`).

## Proto codegen

**Source:** `proto/eaasp/runtime/v2/{common,runtime,hook}.proto` (3 files, package `eaasp.runtime.v2`).

**Rust codegen:** `tonic-build` 0.12 in each runtime crate's `build.rs`.

**Python codegen:** centralized SoT after Phase 3.6 T4 — `scripts/gen_runtime_proto.py` (8.5 KB):
- Generates `_pb2.pyi` stubs for 4 packages: `claude-code-runtime`, `nanobot-runtime`, `pydantic-ai-runtime`, `eaasp-l4-orchestration`.
- Includes `_loosen_enum_stubs` post-process closing ADR-V2-021 / D152 (2026-04-20) — accepts enum ints from grpcio-tools-generated stubs.
- Driven via `PROTO_ROOT` env override.

**TS codegen:** hand-rolled in `lang/ccb-runtime-ts/src/proto/types.ts`; sync validated by `scripts/check-ccb-types-ts-sync.sh` (D149).

## Frontend (Leg B, dormant)

Both apps are **scaffolding only** per ADR-V2-023 P2. Do NOT treat as implementation targets.

| Path | Stack | Status |
|------|-------|--------|
| `web/` | React 19 + Vite 6 + TypeScript 5.7 + Jotai 2.16 + Tailwind 4 + react-markdown 10 + lucide-react 0.469 | Dormant scaffolding |
| `web-platform/` | React 19 + react-router-dom 7 + Vite 6 + TypeScript 5.7 + Jotai 2.16 + Tailwind 4 | Dormant scaffolding |

`web/vite.config.ts` proxies `/api` → `http://127.0.0.1:3001` and `/ws` → `ws://127.0.0.1:3001`, dev port 5180.

## Platform Requirements

**Development:**
- macOS (project memory references M3 Max / arm64 — codegen-units=16). Linux fully supported.
- Rust 1.75+ (toolchain installed via `rustup` typical).
- Python 3.12+ via `uv`.
- Node 20+ (for `web/`) and/or Bun (for `lang/ccb-runtime-ts/`).
- SQLite (`bundled` via `rusqlite` so no system SQLite required for Rust).
- Optional: Docker (for `sandbox-docker` feature), Wasmtime toolchain, Goose binary on PATH (`which goose`) for `eaasp-goose-runtime` integration tests.

**Production / containerized:**
- Per-runtime Dockerfiles emit slim Linux images.
- Repo-root `Dockerfile` produces Ubuntu noble image with `octo-server` (legacy name, pending cleanup).

---

*Stack analysis: 2026-04-26*
