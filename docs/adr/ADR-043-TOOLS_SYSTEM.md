# ADR-043: Tools System

## Status

Accepted

## Date

2026-03-07

## Context

The system requires tool execution for:
- File system operations
- Command execution
- HTTP requests
- Data processing

## Decision

Implement tool registry and execution system:

### Core Components

```rust
// Tool registry
pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<ToolName, ToolDefinition>>>,
    executor: ToolExecutor,
}

// Tool definition
pub struct ToolDefinition {
    pub name: ToolName,
    pub description: String,
    pub parameters: ParameterSchema,
    pub handler: Box<dyn ToolHandler>,
}

// Tool executor
pub struct ToolExecutor {
    sandbox: SandboxManager,
    recorder: ExecutionRecorder,
}
```

### Built-in Tools

| Tool | Description | Risk Level |
|------|-------------|-------------|
| bash | Execute shell commands | High |
| file_read | Read file contents | Medium |
| file_write | Write to files | High |
| http_request | Make HTTP requests | Medium |
| search | Search the web | Low |

### Execution Recording

- **Parameters**: Tool input capture
- **Output**: Result logging
- **Duration**: Execution time tracking
- **Error**: Error details

## Consequences

### Positive

- Unified tool interface
- Risk assessment integration
- Execution audit trail

### Negative

- Tool definition maintenance
- Security risk management

## Related

- [ADR-002: Bash Tool Exec Policy](ADR-002-BASH_TOOL_EXEC_POLICY.md)
