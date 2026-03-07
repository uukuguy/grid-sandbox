# ADR-006: Three-Tier Architecture (Engine/Workbench/Platform)

## Status
Proposed

## Context

octo-sandbox is a mono-repo containing three product tiers:
- `octo-engine`: Core engine (shared library)
- `octo-workbench` (`octo-server` + `web/`): Single-user single-agent workbench
- `octo-platform` (`octo-platform-server` + `web-platform/`): Multi-tenant multi-agent platform

Analogy:
```
CC (Claude Code)    ←→  octo-engine (core capability layer)
RuFlo (framework)   ←→  orchestration module in octo-engine (framework capability layer)
RuView (application) ←→  octo-workbench / octo-platform (product configuration layer)
```

Current problem: Responsibility boundaries between three tiers are unclear, especially after introducing multi-agent orchestration capabilities. Need to clarify which capabilities belong to engine, workbench, and platform.

## Decision

Adopt the following three-tier responsibility division:

### octo-engine (Core Engine — Capability Provider)

**Positioning**: Provides all Agent capability primitives, contains no product logic.

| Module | Responsibility | New/Existing |
|--------|---------------|---------------|
| `agent/` | AgentRuntime, AgentExecutor, AgentLoop, **AgentRouter**, **Capability** | Existing + New |
| `memory/` | 4-layer memory, **VectorIndex (HNSW)**, **HybridQuery**, **Embedding** | Existing + New |
| `event/` | EventBus, **EventStore**, **Projection**, **StateReconstructor** | Existing + New |
| `hooks/` | **HookRegistry**, **HookPoint**, **HookHandler trait** | New |
| `orchestration/` | **TaskOrchestrator**, **AgentManifestLoader** | New |
| `tools/` | ToolRegistry, Tool trait, built-in tools | Existing |
| `mcp/` | McpManager, McpClient, McpToolBridge | Existing |
| `providers/` | Provider trait, ProviderChain | Existing |
| `context/` | SystemPromptBuilder, BudgetManager, Pruner | Existing |
| `security/` | SecurityPolicy, **AIDefence** | Existing + New |
| `session/` | SessionStore | Existing |
| `skills/` | SkillLoader, SkillRegistry | Existing |

**Key Principles**:
- All new modules are **general capabilities**, no workbench/platform-specific logic
- Hook engine only provides registration and execution mechanism, specific hooks configured by product layer
- Agent router only provides matching algorithm, Agent definitions loaded by product layer
- Vector index only provides index and search APIs, data injected by product layer

### octo-workbench (Single-User Workbench — Simplified Configuration)

**Positioning**: Developer-oriented single-user single-agent workbench, pursuing simplicity.

| Capability | Uses Engine | Configuration |
|-----------|-------------|---------------|
| Single Agent mode | AgentRuntime + AgentExecutor | Default Agent, no routing needed |
| Basic Hook | HookRegistry | Few hooks (tool_call, session) |
| Simple Memory | 4-layer memory (HNSW disabled) | Keyword + tag search |
| Event notification | EventBus (EventStore disabled) | Real-time stream to frontend |
| MCP management | McpManager | UI configuration |
| Skills system | SkillLoader | YAML skill definition |

**Not introduced**: Multi-Agent routing, topology management, consensus protocol, pattern learning

### octo-platform (Multi-Tenant Platform — Full Orchestration)

**Positioning**: Team/enterprise-oriented multi-tenant multi-agent platform, pursuing intelligence and scalability.

| Capability | Uses Engine | Configuration |
|-----------|-------------|---------------|
| **Multi-Agent Routing** | AgentRouter + Capability | Declarative Agent definition (YAML) |
| **Full Hook Chain** | HookRegistry | 8+ Hook points, config-driven |
| **Semantic Memory** | VectorIndex + HybridQuery | HNSW index + hybrid query |
| **Event Sourcing** | EventStore + Projection | Full audit trail + state replay |
| **Task Orchestration** | TaskOrchestrator | Task decomposition + Agent assignment |
| **Pattern Learning** | PatternStore (new) | Confidence decay + reward signals |
| **ADR/DDD Constraints** | ContextBuilder extension | Auto-inject relevant constraints to Agent context |
| **Background Workers** | Scheduler (existing extension) | Scheduled optimization, audit, consolidation |
| **Multi-Tenant Isolation** | TenantContext + JWT | Tenant-level Agent pool + memory isolation |

## Consequences

**Benefits**:
- Engine stays generic, both products use as needed
- Workbench stays simple, not burdened by orchestration complexity
- Platform gets full multi-agent capabilities
- New modules have zero impact on workbench (feature gate control)

**Risks**:
- Engine module count increases, needs stricter interface design
- Platform's dependency surface on engine expands

**Mitigation**:
- New modules controlled by Cargo feature flags (`feature = "orchestration"`)
- Engine internal modules decoupled via traits

## References

- Related: ADR-005 (AgentRuntime Modular Split) — prerequisite for this ADR
- Related: ADR-007 through ADR-012 — sub-decisions of this ADR
