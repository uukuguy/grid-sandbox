# DDD Model: Provider Context

**Project**: octo-sandbox
**Bounded Context**: Provider Management
**Status**: Implemented

---

## Ubiquitous Language

| Term | Definition |
|------|------------|
| LlmProvider | Trait for LLM API abstraction |
| ProviderChain | Chain of providers with failover/load-balance |
| LlmInstance | Single provider instance with config |
| ChainProvider | Wraps multiple providers |

---

## Aggregates

### ProviderChain (Aggregate Root)

```rust
pub struct ProviderChain {
    instances: Vec<LlmInstance>,
    strategy: ChainStrategy,
    current_index: usize,
}
```

**Responsibilities**:
- Provider failover management
- Load balancing across providers
- Request routing

---

## Value Objects

### ChainStrategy

```rust
pub enum ChainStrategy {
    Failover,       // Try next on failure
    LoadBalance,    // Round-robin or weighted
    Priority,       // Use primary, fallback to others
    Adaptive,       // Dynamic based on performance
}
```

### LlmConfig

```rust
pub struct LlmConfig {
    pub provider: ProviderType,
    pub model: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub max_retries: u32,
    pub timeout: Duration,
}
```

---

## Domain Services

### AnthropicProvider

```rust
pub struct AnthropicProvider {
    client: reqwest::Client,
    config: LlmConfig,
}
```

### OpenAIProvider

```rust
pub struct OpenAIProvider {
    client: reqwest::Client,
    config: LlmConfig,
}
```

---

## Invariants

1. **Failover Consistency**: Failed provider marked unavailable until recovery
2. **Configuration Isolation**: Each instance has isolated API keys
3. **Timeout Handling**: Requests timeout after configured duration

---

## Dependencies

- **Agent Context**: Used for LLM calls in AgentLoop

---

## References

- [Provider Chain Design](../design/PHASE_2_6_PROVIDER_CHAIN_DESIGN.md)
