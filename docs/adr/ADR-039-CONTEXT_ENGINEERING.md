# ADR-039: Context Engineering

## Status

Accepted

## Date

2026-03-07

## Context

The system requires context management for LLM interactions:
- System prompt building
- Context budget management
- Token budget optimization
- Context pruning strategies

## Decision

Implement context engineering system:

### Core Components

```rust
// System prompt builder
pub struct SystemPromptBuilder {
    templates: PromptTemplateRegistry,
    variables: HashMap<String, String>,
}

// Context budget manager
pub struct ContextBudgetManager {
    max_tokens: usize,
    pruning_strategy: PruningStrategy,
    priority_fn: MessagePriorityFn,
}

// Context pruner
pub struct ContextPruner {
    strategy: PruningStrategy,
    preserve_recent: usize,
}
```

### Pruning Strategies

| Strategy | Description | Use Case |
|----------|-------------|----------|
| Truncate | Remove oldest messages | Simple truncation |
| Summarize | Compress via LLM | Long conversations |
| Relevance | Keep relevant via embedding | Search-heavy conversations |
| Hybrid | Combine multiple | Production use |

### Budget Management

- **Token Budget**: Configurable max tokens
- **Priority Messages**: System prompt, recent messages have priority
- **Dynamic Adjustment**: Adapt based on response length

## Consequences

### Positive

- Optimize token usage
- Prevent context overflow
- Configurable strategies

### Negative

- Pruning may lose context
- Summarization adds latency

## References

- `crates/octo-engine/src/context/`
