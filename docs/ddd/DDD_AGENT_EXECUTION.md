# DDD Model: Agent Execution Context

**Project**: octo-sandbox
**Bounded Context**: Agent Execution
**Status**: Implemented

---

## Ubiquitous Language

| Term | Definition |
|------|------------|
| Agent | An autonomous AI entity with lifecycle, configuration, and execution capabilities |
| AgentRuntime | The core runtime that orchestrates agent execution, owns all components |
| AgentExecutor | Per-session agent instance with cancellation support |
| AgentLoop | Single conversation turn: context build → LLM call → tool execution → repeat |
| AgentCatalog | Multi-index agent registry with state machine (Created → Running → Paused → Stopped) |
| AgentManifest | YAML-defined agent specification (role, goal, backstory, system_prompt) |
| AgentStatus | State machine: Created → Running → Paused → Stopped (or Error) |
| CancellationToken | Signal for cancelling ongoing agent operations |

---

## Aggregates

### AgentRuntime (Aggregate Root)

```rust
pub struct AgentRuntime {
    provider: Arc<dyn LlmProvider>,
    memory: Arc<MemoryStore>,
    tool_registry: Arc<ToolRegistry>,
    mcp_manager: Arc<McpManager>,
    session_store: Arc<SessionStore>,
    config: AgentRuntimeConfig,
}
```

**Responsibilities**:
- Owns all components (provider, memory, tools, MCP, sessions)
- Manages agent lifecycle coordination

### AgentExecutor (Entity)

```rust
pub struct AgentExecutor {
    id: AgentId,
    session_id: SessionId,
    runtime: Arc<AgentRuntime>,
    cancellation: CancellationToken,
}
```

**Responsibilities**:
- Per-session agent instance execution
- Cancellation handling

### AgentCatalog (Entity/Repository)

```rust
pub struct AgentCatalog {
    agents: DashMap<AgentId, AgentState>,
    manifest_loader: ManifestLoader,
    store: AgentStore,
}
```

**Responsibilities**:
- Agent registry with multi-index (by_id, by_status, by_manifest)
- SQLite persistence for agent manifests
- State machine management

---

## Value Objects

### AgentManifest

```rust
pub struct AgentManifest {
    pub id: AgentId,
    pub name: String,
    pub role: String,
    pub goal: String,
    pub backstory: String,
    pub system_prompt: Option<String>,
    pub tools: Vec<String>,
}
```

### AgentStatus

```rust
pub enum AgentStatus {
    Created,
    Running,
    Paused,
    Stopped,
    Error(String),
}
```

---

## Domain Events

| Event | Payload |
|-------|---------|
| AgentCreated | agent_id, manifest |
| AgentStarted | agent_id, session_id |
| AgentPaused | agent_id |
| AgentStopped | agent_id |
| AgentError | agent_id, error |

---

## Invariants

1. **State Transition Validity**: Agent can only transition through valid state paths
2. **Cancellation Isolation**: Cancellation must not affect other sessions
3. **Manifest Validation**: All required fields must be present before activation

---

## Dependencies

- **Memory Context**: Uses MemoryStore for context building
- **Tool Context**: Uses ToolRegistry for tool execution
- **MCP Context**: Uses McpManager for external tool integration
- **Provider Context**: Uses LlmProvider for LLM calls

---

## References

- ADR-014: AgentRuntime Modularization
- ADR-015: AgentRouter Routing Decision
- ADR-016: ManifestLoader YAML Declarative Agent
- [AgentRuntime Design](../design/AGENT_RUNTIME_DESIGN.md)
