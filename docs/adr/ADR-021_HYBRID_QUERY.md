# ADR-021: Hybrid Query Engine

## Status
Completed

## Context

Single retrieval method cannot meet complex query needs, need to integrate vector search and full-text search.

## Decision

Implement `HybridQueryEngine` integrating multiple retrieval methods:

```rust
pub struct HybridQueryEngine {
    vector_backend: Arc<VectorBackend>,
    fts_store: Arc<FtsStore>,
}

pub enum QueryType {
    Vector { query: String, top_k: usize },
    Fts { query: String, limit: usize },
    Hybrid { query: String, vector_weight: f32, fts_weight: f32 },
}
```

## References

- Code paths: `src/memory/hybrid_query.rs`, `src/memory/fts.rs`
