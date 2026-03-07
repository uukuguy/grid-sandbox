# ADR-020: HNSW Vector Index

## Status
Completed

## Context

Semantic search requires efficient large-scale vector approximate nearest neighbor search capability.

## Decision

Implement HNSW index using hnsw_rs, supporting optional feature:

```rust
#[cfg(feature = "hnsw")]
pub struct HnswIndex {
    index: Arc<Mutex<Hnsw>>,
    config: HnswConfig,
}

pub struct HnswConfig {
    pub max_elements: usize,
    pub m: usize,
    pub ef_construction: usize,
    pub ef: usize,
}
```

Support multiple backends via `VectorBackend` abstraction:

```rust
pub enum VectorBackend {
    #[cfg(feature = "hnsw")]
    Hnsw(HnswIndex),
    BruteForce(BruteForceIndex),
}
```

## References

- Code paths: `src/memory/vector_index.rs`, `src/memory/embedding.rs`
