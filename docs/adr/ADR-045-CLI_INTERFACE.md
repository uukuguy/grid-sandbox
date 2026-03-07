# ADR-045: CLI Interface - octo-cli

## Status

Accepted

## Date

2026-03-07

## Context

octo-sandbox currently provides a web-based interface (octo-server + web frontend) for interacting with Octo agents. However, for local development, debugging, and automation, a command-line interface (CLI) is needed.

## Decision

Create a new binary crate `octo-cli` in the workspace that provides a local CLI for interacting with Octo agents.

### Architecture

```
octo-cli
├── Cargo.toml
└── src/
    ├── main.rs              # CLI entry with clap
    └── commands/
        ├── mod.rs           # Module exports
        ├── state.rs         # AppState (AgentRuntime, Catalog)
        ├── types.rs         # Command enums
        ├── agent.rs         # agent list/run/info
        ├── session.rs       # session list/create/show
        ├── memory.rs        # memory search/list/add
        ├── tools.rs         # tools list/invoke/info
        └── config.rs        # config show/validate
```

### Dependencies

- `octo-engine` - Core engine (AgentRuntime, SessionStore, MemorySystem)
- `octo-types` - Shared types
- `octo-sandbox` - Sandbox runtime
- `clap` - CLI argument parsing
- `rusqlite` - Database access

### Commands

| Command | Description |
|---------|-------------|
| `octo agent list` | List all available agents |
| `octo agent info <id>` | Show agent details |
| `octo agent run [id]` | Start interactive session (placeholder) |
| `octo session list` | List all sessions |
| `octo session create` | Create new session |
| `octo session show <id>` | Show session details |
| `octo config show` | Display current configuration |
| `octo config validate` | Validate configuration |

## Consequences

### Positive

- Local CLI for debugging and automation
- Direct access to AgentRuntime without HTTP server
- Configuration validation without starting server

### Negative

- Additional maintenance burden
- Limited interactive capabilities (no WebSocket streaming)
- Duplicates some octo-server functionality

### Limitations

- Interactive mode not implemented (requires dialoguer integration)
- Memory/Tools commands show placeholders (need full runtime initialization)
- Session deletion not implemented

## Related

- [ADR-044: Database Layer](ADR-044-DATABASE_LAYER.md) - SQLite usage
- [CLAUDE.md: Crate Dependency Graph](../CLAUDE.md#crate-dependency-graph--build-order) - Build order
