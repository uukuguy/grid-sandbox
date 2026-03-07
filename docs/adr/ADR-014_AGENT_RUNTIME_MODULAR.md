# ADR-014: AgentRuntime Modular Architecture

## Status
Completed

## Context

Agent module needs to support complete Agent lifecycle management, including:
- Agent runtime initialization and configuration
- Agent instance creation and destruction
- Multi-tenant isolation
- Tools and MCP service integration

Original design concentrated all functions in single module, causing:
- High code coupling, difficult to test
- Lack of clear boundaries
- Difficult to extend

## Decision

Adopt modular architecture, split Agent into multiple sub-modules:

| Sub-module | Responsibility |
|-----------|----------------|
| `runtime.rs` | AgentRuntime main entry, lifecycle management |
| `executor.rs` | AgentExecutor, per-session Agent instance |
| `loop.rs` | AgentLoop, single conversation turn loop |
| `catalog.rs` | AgentCatalog, Agent registry and state machine |
| `store.rs` | AgentStore, SQLite persistence |
| `router.rs` | AgentRouter, task routing decision |
| `manifest_loader.rs` | ManifestLoader, YAML declarative loading |
| `config.rs` | AgentRuntimeConfig, configuration management |

### Architecture Design

```
AgentRuntime
    ├── providers: Arc<ProviderChain>       # LLM provider
    ├── memory: Arc<MemorySystem>           # Memory system
    ├── tools: Arc<ToolRegistry>            # Tool registry
    ├── mcp: Arc<McpManager>               # MCP manager
    ├── security_policy: Arc<SecurityPolicy> # Security policy
    ├── catalog: AgentCatalog               # Agent catalog
    └── store: SqliteAgentStore            # Persistence
```

## Consequences

### Positive

- Responsibility separation, improved code maintainability
- Easier unit testing (can test each module individually)
- Good extensibility, new features don't affect existing code

### Negative

- Module dependencies need explicit management
- Initial development workload increases

## References

- Code paths: `src/agent/mod.rs`, `runtime.rs`, `executor.rs`, `loop.rs`, `catalog.rs`, `router.rs`, `manifest_loader.rs`
