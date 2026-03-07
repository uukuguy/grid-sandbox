# ADR-010: Agent Router Introduction

## Status
Proposed

## Context

Current octo-engine is single-Agent mode, `AgentCatalog` has registration but no selection logic. RuView's `hook-handler.cjs route` implements keyword and semantic-based Agent routing, returns `{ agent, confidence, alternatives }`, entry point for multi-Agent coordination.

## Decision

```rust
// agent/capability.rs
pub struct AgentCapability {
    pub name: String,
    pub capabilities: Vec<String>,       // ["code_generation", "security_audit"]
    pub priority: Priority,              // Low, Normal, High, Critical
    pub max_concurrent_tasks: usize,
    pub system_prompt_template: String,
}

// agent/router.rs
pub struct AgentRouter {
    catalog: Arc<AgentCatalog>,
    // Future: can integrate VectorIndex for semantic matching
}

pub struct RouteResult {
    pub agent_id: AgentId,
    pub confidence: f64,
    pub reason: String,
    pub alternatives: Vec<(AgentId, f64)>,
}

impl AgentRouter {
    /// Route to best Agent based on task description
    pub fn route(&self, task_description: &str) -> Result<RouteResult>;
}
```

**Routing strategy evolution**:
1. V1: Keyword matching (MVP, no extra dependencies)
2. V2: TF-IDF weighted matching (lightweight semantic)
3. V3: HNSW vector matching (depends on ADR-009)

## Consequences

- Workbench: No routing enabled (single Agent direct execution)
- Platform: Routing enabled, supports multi-Agent coordination
- AgentCatalog extended with capability field, backward compatible

## References

- Related: ADR-006 (Three-Tier Architecture) — Router belongs to engine layer
- Related: ADR-009 (HNSW) — V3 routing strategy prerequisite
