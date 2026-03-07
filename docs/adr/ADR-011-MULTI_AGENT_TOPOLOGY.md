# ADR-011: Multi-Agent Topology and Orchestration

## Status
Proposed

## Context

RuView uses hierarchical-mesh topology supporting up to 15 concurrent Agents. octo-platform as multi-tenant platform needs tenant-level multi-Agent coordination. This is **platform-specific** capability, not in engine.

## Decision

Implement in `octo-platform-server`:

```rust
// octo-platform-server/src/orchestration/
pub mod topology;     // Topology management (hierarchical, mesh, adaptive)
pub mod coordinator;  // Coordinator (task allocation, result aggregation)
pub mod consensus;    // Consensus protocol (Raft first, future Byzantine extension)
pub mod pool;         // Agent pool management (scaling, health checks)
```

**Uses from engine**:
- `AgentRouter` — Agent selection
- `HookRegistry` — Orchestration过程中的Hook
- `EventStore` — Orchestration event persistence
- `TaskOrchestrator` — Task decomposition

**Platform additionally provides**:
- Topology-aware message routing
- Tenant-level Agent pool isolation
- Coordinator state management

## Consequences

- Engine won't be polluted with topology/consensus logic
- Platform iterates orchestration capabilities independently
- Workbench completely unaffected

## References

- Related: ADR-006 (Three-Tier Architecture) — Clarify this is platform layer responsibility
- Related: ADR-010 (Agent Router) — Basic capability from engine
