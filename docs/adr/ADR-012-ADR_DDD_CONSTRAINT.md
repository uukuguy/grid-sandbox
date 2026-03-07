# ADR-012: ADR/DDD Documents as Agent Constraint System

## Status
Proposed

## Context

RuView's 44 ADRs and 7 DDD domain models are not just documentation, but **behavioral constraints** for Agents. Before executing tasks, Agents search relevant ADRs to understand decision constraints, search DDDs to understand boundary definitions, avoid "AI-generated code tends to drift — reinventing patterns, contradicting earlier decisions".

Current octo-sandbox has 5 ADRs (security-related) and 1 DDD analysis report, but not yet used for Agent constraints.

## Decision

1. **ADR Indexing**: Maintain `README.md` index table in `docs/adr/` (RuView pattern)
2. **DDD Constraint Injection**: `SystemPromptBuilder` automatically searches relevant ADR/DDD fragments when building context and injects into Agent system prompts
3. **ADR Naming Convention**: Use `ADR-{NNN}-{kebab-case-title}.md` format
4. **ADR Status Tracking**: Each ADR marks Proposed / Accepted / Superseded

**Implementation path**:
```rust
// context/constraint_injector.rs
pub struct ConstraintInjector {
    adr_index: Vec<AdrEntry>,     // Scanned from docs/adr/
    ddd_index: Vec<DddContext>,   // Scanned from docs/ddd/
}

impl ConstraintInjector {
    /// Find relevant constraints based on task description
    pub fn find_constraints(&self, task: &str) -> Vec<Constraint>;

    /// Format constraints for system prompt fragments
    pub fn format_for_prompt(&self, constraints: &[Constraint]) -> String;
}
```

## Consequences

- ADR/DDD no longer just documentation, becomes **active constraint** on Agent behavior
- New architectural decisions automatically followed by subsequent Agents
- Need to maintain ADR/DDD update discipline (outdated constraints more dangerous than no constraints)

## References

- Related: ADR-006 (Three-Tier Architecture) — Constraint injection belongs to engine layer
- RuView `docs/adr/README.md` — Index format reference
- Existing `DDD_DOMAIN_ANALYSIS.md` — Existing domain model foundation
