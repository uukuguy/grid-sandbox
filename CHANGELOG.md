# Changelog

All notable changes to octo-sandbox are documented here.
This project follows [Semantic Versioning](https://semver.org/).

## [0.1.0] — 2026-03-11

### Wave 6: Production Hardening
- Server E2E test suite using `axum::Router` with `tower::ServiceExt::oneshot` (no port binding)
- Unified API error responses (`ApiError` enum with consistent JSON format)
- Graceful shutdown with `SIGTERM`/`SIGINT` signal handling
- `config.default.yaml` synced with all Wave 3-5 features (scheduler, provider_chain, smart_routing, sync, TLS, auth)
- Docker deployment verification checklist
- Production deployment guide (environment, Docker, reverse proxy, TLS, backup)
- CLI verification cases documentation
- Critical `.unwrap()` replaced with proper error handling in production code paths

### Wave 5: Consensus Persistence + Offline Sync + TLS
- **D1-P3**: Byzantine consensus persistence with AES-GCM encrypted keypairs (SQLite storage)
- **D4-lite**: TLS support — self-signed certificate generation + PEM file loading
- **D6**: Offline-first sync engine
  - Hybrid Logical Clock (HLC) timestamps for causal ordering
  - Change tracking with per-field changelog
  - Last-Write-Wins (LWW) conflict resolution
  - Sync protocol with client/server REST API
  - 30 offline sync tests covering HLC, changelog, LWW, protocol, server

### Wave 4: Byzantine Consensus + Singleton Agent
- **D1-P1**: PBFT-lite consensus state machine (Pre-Prepare, Prepare, Commit phases)
- **D1-P2**: Cryptographic signing (Ed25519) + view change protocol
- **D5**: Singleton agent channel design + Tauri auto-updater integration

### Wave 3: Core Enhancement
- **D2**: Extension system deprecated, unified to Hook system
- **D7**: SmartRouting V2 — cross-provider complexity-based model routing (3-tier: low/medium/high)
- **D3**: Image ContentBlock support for multimodal messages

### Wave 2: Platform Foundation (Deferred Completion)
- **T1+T2**: Canary token integration + symlink defense (security hardening)
- **T3+T4**: Event publishing to EventBus + EventStore REST API (observability)
- **T5**: TTL cleanup scheduled task for memory entries
- **T6+T7**: Platform WebSocket integration + ApprovalGate wiring
- **T8+T9**: Realtime event streaming + Collaboration panel (dashboard)

### Deferred Items (D2-D7)
- **D2**: Dashboard — embedded web dashboard with 12-theme system, API endpoints, integration tests
- **D4**: Auto-memory — SessionEndHook, `/memory` command, memory retrieval
- **D5**: Dual-agent mode — Plan + Build agent collaboration
- **D6**: N-Agent collaboration framework (10 collaboration sub-features)
- **D7**: Dashboard remote access with TLS, auth, RBAC, CORS (8 sub-features)
- **D3**: Tauri 2.0 desktop application (6 sub-features)

### CLI Redesign (Phase 1-5)
- **Phase 1**: Core infrastructure — Clap argument parsing, output formatting, color themes
- **Phase 2**: REPL interactive mode — streaming responses, command history, multi-line input
- **Phase 3**: Management subcommands — agent, session, memory, tools, MCP, config
- **Phase 4**: Full-screen TUI mode — 12 color themes, panel layout, keyboard navigation
- **Phase 5**: Advanced features — completions, doctor, dashboard, dual-agent mode

### Harness + Skills Completion (Phase 1-4)
- **Phase 1**: Type unification and naming cleanup
- **Phase 2**: Skills to AgentLoop integration
- **Phase 3**: Security and approval pipeline
- **Phase 4**: Provider pipeline and Skills API

### Phase 2: Engine Foundation
- Context engineering: SystemPromptBuilder, ContextBudgetManager, ContextPruner
- 7 built-in tools: bash, file_read, file_write, file_edit, grep, glob, find
- Multi-layer memory: WorkingMemory (L0), SessionMemory (L1), SqliteMemoryStore (L2)
- Knowledge graph with FTS5 full-text search
- SQLite persistence with async wrapper (tokio-rusqlite)
- Session management with SQLite backend
- Memory tools: memory_store, memory_search, memory_update
- Skill system: YAML manifest parser, hot-reload with file watcher
- MCP integration: McpClient (stdio), McpManager, McpToolBridge
- Tool execution recorder with SQLite storage
- Frontend: Chat, Tools, Memory, Debug, MCP Workbench pages

### Phase 1: Core Engine
- Agent architecture: AgentRuntime, AgentExecutor, AgentLoop
- LLM providers: Anthropic and OpenAI adapters with streaming
- Message types: User, Assistant, ToolCall, ToolResult
- Tool registry with dynamic registration
- Event bus for observability
- Sandbox runtime: native subprocess adapter
- Frontend: React + TypeScript + Vite + Jotai + TailwindCSS
- WebSocket streaming for real-time AI responses

---

## Known Limitations

- SQLite is the only supported database backend (no PostgreSQL/MySQL)
- WASM and Docker sandbox adapters are optional features (not enabled by default)
- MCP only supports stdio transport (SSE transport via rmcp)
- Smart routing requires manual tier configuration for custom providers
- Sync protocol is LWW-based (not CRDT); conflicts resolve by timestamp, not merge

## Deferred Items

| ID | Description | Status |
|----|-------------|--------|
| D4-ACME | Built-in ACME certificate automation | Deferred — use Caddy reverse proxy instead |
| D6-V2 | CRDT-based offline sync | Deferred — LWW sufficient for current use cases |
| D6-Desktop | Desktop sync integration | Deferred — not in production hardening scope |
| D2-merge | Extension + Hook system merge | Deferred — Extension deprecated, Hook system is primary |
| D3-multimodal | ContentBlock multimodal extensions | Deferred — requires multi-modal provider support |
| D5-autoupdate | Tauri auto-update with artifact hosting | Deferred — requires release pipeline |
| D7-V2 | SmartRouting V2 cross-provider | Deferred — V1 complete, V2 needs multi-provider scenarios |
| D8 | CLI Server mode (HTTP client) | Deferred — CLI embeds engine directly |
| D9 | OpenTelemetry export | Deferred — requires external monitoring infrastructure |
