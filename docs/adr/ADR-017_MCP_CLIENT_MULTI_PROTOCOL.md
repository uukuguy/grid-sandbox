# ADR-017: MCP Client Multi-Protocol Support

## Status
Completed

## Context

MCP supports multiple transport protocols: stdio (local process) and SSE (remote HTTP).

## Decision

Implement unified `McpClient` interface supporting multiple transports:

```rust
pub enum McpClient {
    Stdio(McpStdioClient),
    Sse(McpSseClient),
}

pub trait McpTransport: Send {
    async fn send(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse>;
    async fn subscribe(&self, handler: Box<dyn NotificationHandler>) -> Result<()>;
}
```

## References

- Code paths: `src/mcp/stdio.rs`, `src/mcp/sse.rs`, `src/mcp/client.rs`
