# Domain-Driven Design Models

> **Note**: DDD files are organized by bounded context (RuView style).
> Each file documents one domain with Ubiquitous Language, Aggregates, Value Objects, and Invariants.

This folder contains Domain-Driven Design (DDD) specifications for the octo-sandbox project.

## Bounded Context Files

| File | Context | Description |
|------|---------|-------------|
| [DDD_AGENT_EXECUTION.md](DDD_AGENT_EXECUTION.md) | Agent Execution | Agent lifecycle, runtime, executor, loop, catalog |
| [DDD_MEMORY.md](DDD_MEMORY.md) | Memory | Multi-layer memory, vector search, knowledge graph |
| [DDD_MCP_INTEGRATION.md](DDD_MCP_INTEGRATION.md) | MCP Integration | MCP server lifecycle, tool bridge, protocol support |
| [DDD_TOOL_EXECUTION.md](DDD_TOOL_EXECUTION.md) | Tool Execution | Tool registry, execution, risk assessment |
| [DDD_PROVIDER.md](DDD_PROVIDER.md) | Provider | LLM provider chain, failover, load balancing |
| [DDD_SECURITY.md](DDD_SECURITY.md) | Security Policy | Path validation, autonomy levels, action tracking |
| [DDD_EVENT_SYSTEM.md](DDD_EVENT_SYSTEM.md) | Event System | Event bus, event sourcing, projections |
| [DDD_HOOK_SYSTEM.md](DDD_HOOK_SYSTEM.md) | Hook System | Hook registry, handlers, context propagation |
| [DDD_DOMAIN_ANALYSIS.md](DDD_DOMAIN_ANALYSIS.md) | (Legacy) | Comprehensive domain analysis (Chinese) |
| [DDD_CHANGE_LOG.md](DDD_CHANGE_LOG.md) | (Legacy) | Change tracking log |

## Domain Model Overview

| # | Context | Responsibility | Key Modules |
|---|---------|---------------|-------------|
| 1 | Agent Execution | Agent lifecycle, execution loop, routing | `agent/runtime.rs`, `agent/executor.rs`, `agent/loop.rs` |
| 2 | Memory Management | Multi-layer memory, vector search, knowledge graph | `memory/working.rs`, `memory/store.rs` |
| 3 | MCP Integration | MCP server lifecycle, protocol support, tool bridge | `mcp/manager.rs`, `mcp/client.rs` |
| 4 | Tool Execution | Tool registry, execution, risk assessment | `tools/registry.rs`, `tools/executor.rs` |
| 5 | Provider Management | LLM provider chain, failover, load balancing | `providers/chain.rs` |
| 6 | Security Policy | Path validation, autonomy levels, action tracking | `security/policy.rs` |
| 7 | Session Management | Session store, lifecycle, state management | `session/store.rs` |
| 8 | Event System | Event bus, event sourcing, projections | `event/bus.rs`, `event/store.rs` |
| 9 | Hook System | Hook registry, handlers, context propagation | `hooks/registry.rs` |
| 10 | CLI Interface | Local command-line interface for agent interaction | `octo-cli/` |

## How to Read

Each bounded context defines:

- **Ubiquitous Language** — Terms with precise meanings used in both code and conversation
- **Aggregates** — Clusters of objects that enforce business rules
- **Value Objects** — Immutable data with meaning
- **Domain Events** — Things that happened that other contexts may care about
- **Invariants** — Rules that must always be true

## How Agents Use DDD

### Current Mechanism

1. **Manual Reference**: Agents can reference DDD_DOMAIN_ANALYSIS.md when writing code
2. **Change Log**: DDD_CHANGE_LOG.md automatically tracks architecture changes

### Future Mechanism (ADR-012)

According to ADR-012, future implementation will include **ConstraintInjector** to automatically inject relevant DDD constraints into Agent context.

---

## Relationship with ADRs

- [Architecture Decision Records](../adr/README.md) — Why each technical choice was made
- ADRs define boundaries that DDD models must follow
- DDD models define the language that ADRs reference
