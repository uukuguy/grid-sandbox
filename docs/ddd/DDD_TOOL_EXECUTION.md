# DDD Model: Tool Execution Context

**Project**: octo-sandbox
**Bounded Context**: Tool Execution
**Status**: Implemented

---

## Ubiquitous Language

| Term | Definition |
|------|------------|
| ToolRegistry | Repository for tool registration and lookup |
| Tool | Core trait for tool abstraction |
| ToolContext | Execution context with security and metadata |
| ToolResult | Execution result with output and error |
| ToolSpec | Tool specification for discovery |

---

## Aggregates

### ToolRegistry (Aggregate Root)

```rust
pub struct ToolRegistry {
    tools: DashMap<String, Arc<dyn Tool>>,
    built_in: Vec<Arc<dyn Tool>>,
    mcp_tools: HashMap<ServerId, Arc<McpToolBridge>>,
}
```

**Responsibilities**:
- Tool registration and discovery
- Built-in tool management
- MCP tool integration

---

## Value Objects

### ToolSpec

```rust
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub source: ToolSource,
}
```

### ToolSource

```rust
pub enum ToolSource {
    BuiltIn,
    Mcp { server_id: ServerId },
    Skill { skill_id: SkillId },
}
```

### ToolResult

```rust
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub metadata: HashMap<String, Value>,
}
```

---

## Domain Services

### BashTool

```rust
pub struct BashTool {
    security_policy: Arc<SecurityPolicy>,
    recorder: Arc<ToolExecutionRecorder>,
}
```

### ToolExecutionRecorder

```rust
pub struct ToolExecutionRecorder {
    db: Arc<Database>,
}
```

---

## Invariants

1. **Tool Name Uniqueness**: Each tool must have unique name
2. **Schema Validation**: Tool input must match specification
3. **Security Enforcement**: Path validation before execution

---

## Dependencies

- **Security Context**: Uses SecurityPolicy for execution control
- **MCP Context**: Integrates MCP tools via McpToolBridge

---

## References

- ADR-005: AgentRuntime Modular Split
- ADR-007-MCP: MCP call_mcp_tool Lock-Free I/O
