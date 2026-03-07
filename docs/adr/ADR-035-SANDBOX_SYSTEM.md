# ADR-035: Sandbox System

## Status

Accepted

## Date

2026-03-07

## Context

The system requires sandboxed execution environments for untrusted code:
- Code execution isolation
- Resource limits
- System call filtering
- Network access control

## Decision

Implement multi-runtime sandbox system supporting:

### Runtime Adapters

```rust
// Sandbox runtime trait
pub trait RuntimeAdapter: Send + Sync {
    fn execute(&self, code: &str, config: &SandboxConfig) -> Result<SandboxOutput>;
    fn terminate(&self, execution_id: &ExecutionId) -> Result<()>;
    fn get_status(&self, execution_id: &ExecutionId) -> Result<ExecutionStatus>;
}

// Sandbox configuration
pub struct SandboxConfig {
    pub timeout_ms: u64,
    pub memory_limit_mb: u64,
    pub allowed_syscalls: Vec<String>,
    pub network_policy: NetworkPolicy,
}
```

### Runtime Implementations

| Runtime | Use Case | Isolation Level |
|---------|----------|----------------|
| Native | Trusted local execution | Process |
| WASM | Lightweight sandboxing | Wasmtime |
| Docker | Full isolation | Container |

### Network Policy

```rust
pub enum NetworkPolicy {
    AllowAll,
    DenyAll,
    AllowList(Vec<IpCidr>),
    AllowInternal,
}
```

## Consequences

### Positive

- Multiple isolation levels for different trust levels
- Resource limits prevent runaway execution
- Portable WASM runtime

### Negative

- Docker requires container runtime
- Performance overhead for strong isolation

## Related

- `crates/octo-sandbox/` - Sandbox runtime adapters
