# ADR-009: HNSW Vector Index Introduction

## Status
Proposed

## Context

Current `memory/` module has FTS full-text search but no vector semantic search. RuView's swarm DB uses 768-dimension HNSW vectors (M=16, efConstruction=200, cosine distance), search 150x-12,500x faster than brute force. This is the foundation for semantic memory retrieval and pattern learning.

## Decision

Add vector search capability to `memory/` module:

```rust
// memory/vector_index.rs
pub struct HnswIndex {
    // Using hnsw_rs or usearch crate
    index: /* ... */,
    config: HnswConfig,
}

pub struct HnswConfig {
    pub dimensions: usize,      // 384 or 768
    pub m: usize,               // Default 16
    pub ef_construction: usize, // Default 200
    pub ef_search: usize,       // Default 100
    pub metric: DistanceMetric, // Cosine | Euclidean | DotProduct
}

impl HnswIndex {
    pub fn insert(&mut self, id: &str, vector: &[f32]) -> Result<()>;
    pub fn search(&self, query: &[f32], k: usize) -> Result<Vec<SearchResult>>;
    pub fn delete(&mut self, id: &str) -> Result<()>;
}

// memory/hybrid_query.rs — Hybrid query routing
pub struct HybridQueryEngine {
    sqlite: Arc<SqliteMemoryStore>,
    vector: Arc<RwLock<HnswIndex>>,
}

impl HybridQueryEngine {
    pub async fn query(&self, q: MemoryQuery) -> Result<Vec<MemoryEntry>> {
        match q.query_type() {
            QueryType::Semantic => self.vector_search(q).await,
            QueryType::Structured => self.sqlite_search(q).await,
            QueryType::Hybrid => self.merged_search(q).await,
        }
    }
}
```

**Embedding generation**:
- Option A: Generate via LLM Provider API (simple but has latency)
- Option B: Local ONNX Runtime (fast but increases binary size)
- Recommended: Provider API first, ONNX as optional feature

## Consequences

- Existing MemoryStore trait unchanged
- HybridQueryEngine is new high-level API,封装结构 + 语义查询
- Workbench: Optional (disabled by default)
- Platform: Enabled by default

## References

- Related: ADR-006 (Three-Tier Architecture) — VectorIndex belongs to engine layer
- RuView `.swarm/schema.sql` vector_indexes — Schema reference
