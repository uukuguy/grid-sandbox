# ADR-007-MCP: MCP call_mcp_tool Lock-Free I/O

## Status
Accepted

## Context

`AgentRuntime::add_mcp_server()` has already completed connect-outside-mutex refactoring (ADR-005): briefly hold lock to mark `Starting` status → execute slow connect/list_tools outside lock → acquire lock again to insert connected client.

However, `call_mcp_tool()` method still maintains the old pattern — executing full network I/O while holding `Mutex<McpManager>`:

```rust
pub async fn call_mcp_tool(&self, server_id: &str, tool_name: &str, arguments: serde_json::Value)
    -> Result<serde_json::Value, String>
{
    let guard = self.mcp_manager.lock().await;  // Hold lock
    guard.call_tool(server_id, tool_name, arguments).await  // Network I/O (possibly hundreds of ms)
}
```

`call_tool()` internally holds `RwLock<Box<dyn McpClient>>` read lock to execute `client.call_tool().await`. In concurrent scenarios (multiple tool calls triggered simultaneously), all callers are waiting for the same `Mutex<McpManager>` to release, actually serializing all MCP tool calls, throughput degrades to single-threaded.

Code review (architecture + quality agent) marked this as CRITICAL issue.

## Decision

Adopt the same "clone-under-lock, call-outside-lock" pattern as `add_mcp_server()`:

```rust
pub async fn call_mcp_tool(&self, server_id: &str, tool_name: &str, arguments: serde_json::Value)
    -> Result<serde_json::Value, String>
{
    // Briefly hold lock: only clone Arc<RwLock<...>>, no I/O
    let client = {
        let guard = self.mcp_manager.lock().await;
        guard.clients().get(server_id).cloned()
            .ok_or_else(|| format!("MCP server not found: {server_id}"))?
    };
    // Network I/O executes outside lock: concurrent calls can proceed simultaneously
    let client_guard = client.read().await;
    client_guard.call_tool(tool_name, arguments).await.map_err(|e| e.to_string())
}
```

`McpManager::clients()` returns `&HashMap<String, Arc<RwLock<Box<dyn McpClient>>>>`. Cloning `Arc` is O(1) atomic reference count operation; lock hold time is near zero.

## Consequences

### Positive

- N concurrent MCP tool calls can truly execute concurrently, no longer serialized
- `McpManager` mutex hold time reduced from "entire tool call duration" to "one HashMap lookup + Arc clone"
- Consistent with `add_mcp_server()` concurrent pattern, reduced cognitive load

### Negative

- Need `McpManager` new `clients()` accessor (already exists)
- Very short window, if server is removed after Arc clone but before call_tool, will still hold reference to deleted server's client (won't panic, will get error at I/O layer)

### Neutral

- `RwLock<Box<dyn McpClient>>` read lock still allows multiple concurrent reads (multiple tool calls using same client simultaneously)
- If a MCP server itself is single-threaded, concurrent call effect depends on server implementation, not client

## References

- Code paths: `crates/octo-engine/src/agent/runtime_mcp.rs`
- Related: ADR-005 (AgentRuntime Modular Split)
