# Architecture Decision Records

This folder contains Architecture Decision Records (ADRs) that document every significant technical choice in the octo-sandbox project.

## Why ADRs?

ADRs capture the **context**, **options considered**, **decision made**, and **consequences** for each architectural choice. They serve three purposes:

1. **Institutional memory** — Anyone (human or AI) can read *why* we chose a particular approach, not just see the code.

2. **AI-assisted development** — When an AI agent works on this codebase, ADRs give it the constraints and rationale it needs to make changes that align with the existing architecture.

3. **Review checkpoints** — Each ADR is a reviewable artifact. When a proposed change touches the architecture, the ADR forces the author to articulate tradeoffs *before* writing code.

### ADRs and Domain-Driven Design

The project uses [Domain-Driven Design](../ddd/) (DDD) to organize code into bounded contexts. ADRs and DDD work together:

- **ADRs define boundaries**: Each ADR establishes architectural decisions that bounded contexts must follow.
- **DDD models define the language**: Domain models define terms that ADRs reference precisely.
- **Together they prevent drift**: An AI agent reading ADR-010 knows how the AgentRouter works, because the ADR documents it.

### ADR File Structure

Each ADR follows this format (one file per ADR):

```markdown
# ADR-XXX: Title

## Status
Accepted | Completed | Proposed | Superseded

## Context
What problem or gap prompted this decision?

## Decision
What we chose to do and how (with code examples if applicable)

## Consequences
### Positive
- What improved

### Negative
- What got harder or what risks remain

## References
- Related ADRs
- External papers or documentation
- Code paths
```

### Status Definitions

| Status | Meaning |
|--------|---------|
| **Proposed** | Under discussion, not yet approved |
| **Accepted** | Approved, awaiting implementation |
| **Completed** | Implemented and verified |
| **Superseded** | Replaced by a later ADR |

---

## ADR Index

### Security

| ADR | Title | Status |
|-----|-------|--------|
| ADR-001 | PathValidator Security Policy Injection | Accepted |
| ADR-002 | BashTool ExecPolicy Default Enabled | Accepted |
| ADR-003 | API Key Hash Algorithm Upgrade to HMAC-SHA256 | Accepted |
| ADR-004 | Middleware Execution Order Fix (LIFO) | Accepted |
| ADR-005 | AgentRuntime Modular Split | Accepted |
| ADR-006 | HMAC Secret Force Check (Fail-Fast) | Accepted |
| ADR-007 | MCP call_mcp_tool Lock-Free I/O | Accepted |

### Multi-Agent Orchestration

| ADR | Title | Status |
|-----|-------|--------|
| ADR-006 (Three-Tier) | Three-Tier Architecture (Engine/Workbench/Platform) | Accepted |
| ADR-007 (Hook) | Hook Engine Introduction | Proposed |
| ADR-009 | HNSW Vector Index Introduction | Proposed |
| ADR-010 | Agent Router Introduction | Proposed |
| ADR-011 | Multi-Agent Topology and Orchestration | Proposed |
| ADR-012 | ADR/DDD Documents as Agent Constraint System | Proposed |

### Agent Architecture

| ADR | Title | Status |
|-----|-------|--------|
| ADR-014 | AgentRuntime Modularization | Completed |
| ADR-015 | AgentRouter Routing Decision | Completed |
| ADR-016 | ManifestLoader YAML Declarative Agent | Completed |

### MCP Integration

| ADR | Title | Status |
|-----|-------|--------|
| ADR-013 | MCP Manager Lifecycle Management | Completed |
| ADR-017 | MCP Client Multi-Protocol Support | Completed |
| ADR-018 | MCP Tool Bridge Unified Interface | Completed |

### Memory System

| ADR | Title | Status |
|-----|-------|--------|
| ADR-019 | Four-Layer Memory Architecture | Completed |
| ADR-020 | HNSW Vector Index | Completed |
| ADR-021 | Hybrid Query Engine | Completed |
| ADR-022 | ContextInjector Zone B Dynamic Context | Completed |

### Hooks System

| ADR | Title | Status |
|-----|-------|--------|
| ADR-023 | HookRegistry Global Hook Registration | Completed |
| ADR-024 | HookHandler Event Processing Mechanism | Completed |
| ADR-025 | HookContext Context Propagation | Completed |

### Event Sourcing

| ADR | Title | Status |
|-----|-------|--------|
| ADR-026 | EventBus Event Bus | Completed |
| ADR-027 | EventStore Event Persistence | Completed |
| ADR-028 | ProjectionEngine Projection Engine | Completed |
| ADR-029 | StateReconstructor State Replay | Completed |

### Engine Subsystems

| ADR | Title | Status |
|-----|-------|--------|
| ADR-030 | Hooks System | Accepted |
| ADR-031 | Event System | Accepted |
| ADR-032 | Scheduler System | Accepted |
| ADR-033 | Secret Manager | Accepted |
| ADR-034 | Observability | Accepted |
| ADR-035 | Sandbox System | Accepted |
| ADR-036 | Extension System | Accepted |
| ADR-037 | Session Management | Accepted |
| ADR-038 | Audit System | Accepted |
| ADR-039 | Context Engineering | Accepted |
| ADR-040 | Logging System | Accepted |
| ADR-041 | Skill System | Accepted |
| ADR-042 | Skill Runtime | Accepted |
| ADR-043 | Tools System | Accepted |
| ADR-044 | Database Layer | Accepted |

### CLI & Interface

| ADR | Title | Status |
|-----|-------|--------|
| ADR-045 | CLI Interface (octo-cli) | Accepted |

---

## How Agents Use ADR/DDD

### Current Mechanism

1. **Manual Reference**: Agents can reference ADR documents when writing code
2. **DDD Change Log**: `docs/ddd/DDD_CHANGE_LOG.md` tracks architecture changes

### Future Mechanism (ADR-012)

According to ADR-012 design, future implementation will include **ConstraintInjector**:

```rust
// context/constraint_injector.rs
pub struct ConstraintInjector {
    adr_index: Vec<AdrEntry>,     // Scanned from docs/adr/
    ddd_index: Vec<DddContext>,   // Scanned from docs/ddd/
}

impl ConstraintInjector {
    /// Find relevant constraints based on task description
    pub fn find_constraints(&self, task: &str) -> Vec<Constraint>;

    /// Format constraints for system prompt
    pub fn format_for_prompt(&self, constraints: &[Constraint]) -> String;
}
```

This will allow agents to automatically search and inject relevant ADR/DDD constraints into system prompts during task execution.

---

## Related Links

- [DDD Domain Models](../ddd/) — Bounded context definitions and ubiquitous language
- [CLAUDE.md](../../CLAUDE.md) — Project instructions for AI agents
- [Design Documents](../design/) — Technical design documents

---

## References

### External
- [Michael Nygard's ADR Template](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions)
- [RuView ADR Structure](https://github.com/ruvnet/wifi-densepose-rs/tree/main/docs/adr)

### Internal
- [ADR-012: ADR/DDD Documents as Agent Constraint System](./ADR-012-ADR_DDD_CONSTRAINT.md) — Future ConstraintInjector design
- [DDD Domain Models](../ddd/) — Bounded context definitions
