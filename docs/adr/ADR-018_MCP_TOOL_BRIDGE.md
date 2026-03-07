# ADR-018: MCP Tool Bridge Unified Interface

## Status
Completed

## Context

Tools exposed by MCP servers need unified integration into system tool registry.

## Decision

Implement `McpToolBridge` to convert MCP tools to system tools:

```rust
pub struct McpToolBridge {
    clients: HashMap<SandboxId, McpClient>,
}

impl ToolBridge for McpToolBridge {
    fn get_tools(&self, sandbox_id: &SandboxId) -> Result<Vec<Tool>>;
}
```

## References

- Code paths: `src/mcp/bridge.rs`
