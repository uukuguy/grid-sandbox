# ADR-013: MCP Manager Lifecycle Management

## Status
Completed

## Context

MCP (Model Context Protocol) servers need dynamic runtime management:
- Start and stop MCP server processes
- Process health monitoring
- Log collection
- Tool discovery and synchronization

## Decision

Implement `McpManager` responsible for full MCP server lifecycle:

```rust
pub struct McpManager {
    processes: HashMap<SandboxId, Child>,
    storage: Arc<McpStorage>,
}

impl McpManager {
    pub async fn start_server(&self, config: &McpServerConfig) -> Result<()>;
    pub async fn stop_server(&self, sandbox_id: &SandboxId) -> Result<()>;
    pub async fn get_tools(&self, sandbox_id: &SandboxId) -> Result<Vec<Tool>>;
}
```

## References

- Code paths: `src/mcp/manager.rs`, `src/mcp/storage.rs`
