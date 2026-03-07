# DDD Model: Memory Context

**Project**: octo-sandbox
**Bounded Context**: Memory Management
**Status**: Implemented

---

## Ubiquitous Language

| Term | Definition |
|------|------------|
| WorkingMemory | Short-term memory for current conversation (L0) |
| SessionMemory | Per-session persistent memory (L1) |
| MemoryStore | Long-term persistent storage (L2) |
| KnowledgeGraph | Entity-relation graph with FTS search |
| ContextInjector | Injects contextual memory into agent prompts |
| HybridQueryEngine | Combines keyword and semantic search |

---

## Aggregates

### MemoryStore (Aggregate Root)

```rust
pub struct MemoryStore {
    working: Arc<WorkingMemory>,
    session: Arc<SessionMemory>,
    persistent: Arc<PersistentMemory>,
    knowledge_graph: Arc<KnowledgeGraph>,
}
```

**Responsibilities**:
- Manages all memory layers
- Coordinates cross-layer queries
- Handles memory persistence

### WorkingMemory (Entity)

```rust
pub struct WorkingMemory {
    messages: Vec<Message>,
    budget: ContextBudget,
}
```

**Responsibilities**:
- Current conversation context
- Context budget management
- Message pruning

---

## Value Objects

### MemoryEntry

```rust
pub struct MemoryEntry {
    pub id: MemoryId,
    pub content: String,
    pub memory_type: MemoryType,
    pub created_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}
```

### MemoryType

```rust
pub enum MemoryType {
    Working,    // L0: Current conversation
    Session,    // L1: Per-session
    Persistent, // L2: Long-term
    Knowledge,  // Entity-relation
}
```

### ContextBudget

```rust
pub struct ContextBudget {
    pub max_tokens: usize,
    pub warning_threshold: f64,
    pub critical_threshold: f64,
}
```

---

## Domain Services

### HybridQueryEngine

```rust
pub struct HybridQueryEngine {
    vector_backend: VectorBackend,
    embedding_client: EmbeddingClient,
    fts_store: FtsStore,
}
```

**Methods**:
- `query(keyword, semantic, limit)` - Combined keyword + vector search
- `hybrid_search(query, threshold, top_k)` - Weighted scoring

### ContextInjector

```rust
pub struct ContextInjector {
    memory: Arc<MemoryStore>,
    zone_a: Vec<Message>,  // Static system prompt
    zone_b: Vec<Message>,  // Dynamic context
}
```

**Methods**:
- `inject(agent_id, task)` - Inject relevant memory into zone B
- `build_context(agent_id)` - Build full context for prompt

---

## Domain Events

| Event | Payload |
|-------|---------|
| MemoryStored | entry_id, memory_type |
| MemoryRecalled | query, results |
| MemoryForgotten | entry_id |
| ContextInjected | agent_id, entries |

---

## Invariants

1. **Budget Compliance**: Total context must not exceed max_tokens
2. **Memory Isolation**: Sessions must not access each other's working memory
3. **Vector Consistency**: Embeddings must be regenerated when content changes

---

## Dependencies

- **Agent Context**: Provides context for agent prompts
- **Event Context**: Emits memory events for auditing
- **Provider Context**: Uses embeddings for semantic search

---

## References

- ADR-019: Four-Layer Memory Architecture
- ADR-020: HNSW Vector Index
- ADR-021: Hybrid Query Engine
- ADR-022: ContextInjector Zone B Dynamic Context
- [Memory Plan](../design/MEMORY_PLAN.md)
