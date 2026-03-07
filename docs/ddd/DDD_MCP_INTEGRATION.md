# DDD Model: MCP Integration Context

**Project**: octo-sandbox
**Bounded Context**: MCP Integration
**Status**: Implemented

---

## Ubiquitous Language

| Term | Definition |
|------|------------|
| McpManager | Manages MCP server lifecycle (start/stop) |
| McpClient | Client for MCP protocol (stdio/SSE) |
| McpToolBridge | Wraps MCP tools into unified Tool trait |
| McpStorage | SQLite persistence for MCP server configs |
| SseMcpClient | Server-Sent Events transport for MCP |

---

## Aggregates

### McpManager (Aggregate Root)

```rust
pub struct McpManager {
    clients: HashMap<ServerId, McpClient>,
    storage: McpStorage,
    runtime: Runtime,
}
```

**Responsibilities**:
- MCP server lifecycle management
- Server process spawning and termination
- Tool bridge integration

---

## Value Objects

### McpServer

```rust
pub struct McpServer {
    pub id: ServerId,
    pub name: String,
    pub transport: Transport,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}
```

### Transport

```rust
pub enum Transport {
    Stdio { command: String, args: Vec<String> },
    Sse { url: Url },
}
```

---

## Domain Services

### McpToolBridge

```rust
pub struct McpToolBridge {
    client: Arc<McpClient>,
}

impl Tool for McpToolBridge {
    async fn execute(&self, params: Value) -> Result<ToolResult>;
}
```

**Responsibilities**:
- Convert MCP tools to unified Tool trait
- Handle protocol translation

---

## Domain Events

| Event | Payload |
|-------|---------|
| McpServerStarted | server_id, name |
| McpServerStopped | server_id |
| McpToolCalled | server_id, tool_name, params |

---

## Invariants

1. **Lifecycle Isolation**: Each server runs in isolated process
2. **Connection Reuse**: Client connections should be reused when possible
3. **Graceful Shutdown**: Servers must clean up resources on stop

---

## Dependencies

- **Tool Context**: Registers MCP tools in ToolRegistry
- **Session Context**: Associates MCP servers with sessions

---

## References

- ADR-013: MCP Manager Lifecycle Management
- ADR-017: MCP Client Multi-Protocol Support
- ADR-018: MCP Tool Bridge Unified Interface
- [MCP Workbench Design](../design/MCP_WORKBENCH_DESIGN.md)
