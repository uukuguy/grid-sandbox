# ADR-019: Four-Layer Memory Architecture

## Status
Completed

## Context

System needs multi-layer memory system to support different scenarios:
- L0: Current conversation context (Working Memory)
- L1: Session-level memory (Session Memory)
- L2: Long-term memory (Persistent Memory)
- L3: Knowledge Graph

## Decision

Implement unified MemorySystem containing four layers:

```rust
pub struct MemorySystem {
    pub working: InMemoryWorkingMemory,      // L0
    pub session: SqliteSessionStore,        // L1
    pub persistent: SqliteMemoryStore,    // L2
    pub knowledge_graph: Arc<RwLock<KnowledgeGraph>>, // L3
}
```

**Layer design**:

| Layer | Storage | Lifetime | Access Pattern |
|-------|---------|----------|---------------|
| L0 Working | InMemory/HashMap | Current conversation | Instant read/write |
| L1 Session | SQLite | Session duration | Persistent |
| L2 Persistent | SQLite | Long-term | Search + retrieval |
| L3 Knowledge | Memory + SQLite | Permanent | Graph traversal |

## References

- Code paths: `src/memory/mod.rs`, `src/memory/working.rs`, `src/memory/sqlite_working.rs`, `src/memory/sqlite_store.rs`, `src/memory/traits.rs`, `src/memory/store_traits.rs`
