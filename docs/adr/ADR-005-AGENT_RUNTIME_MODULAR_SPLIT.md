# ADR-005: AgentRuntime Modular Split

## Status
Accepted

## Context

`AgentRuntime` is the core component of the entire system, responsible for:

- Agent lifecycle management (start/stop/pause/resume)
- MCP Server management (add/remove/list/call)
- Scheduled task execution (execute_scheduled_task)
- Providing various getter methods

Before refactoring, all `impl` methods for the above responsibilities were piled in the same `runtime.rs` file. As features grew, this file exceeded 500 lines (project guideline limit), violating the Single Responsibility Principle and forming a typical **God Object anti-pattern**:

- File too long, navigation and location difficult
- Code of different responsibilities mixed, modifying MCP features requires locating code in agent lifecycle
- High merge conflict probability: parallel development easily causes conflicts in the same file
- Testing difficult to isolate: hard to test one responsibility without loading other implementations

## Decision

Split `impl AgentRuntime` blocks in `runtime.rs` by responsibility into three separate sub-modules:

| Sub-module file | Responsibility | Contains methods |
|-----------------|-----------------|-------------------|
| `runtime.rs` | Core struct definition, constructor, getters | `new()`, `with_*()`, getters |
| `runtime_lifecycle.rs` | Agent lifecycle management | `start()`, `stop()`, `pause()`, `resume()` |
| `runtime_mcp.rs` | MCP Server management | `add_mcp_server()`, `remove_mcp_server()`, `list_mcp_servers()`, `call_mcp_tool()`, etc. |
| `runtime_scheduler.rs` | Scheduled task execution | `execute_scheduled_task()` |

**Module declaration** (`mod.rs`):

```rust
mod runtime;
mod runtime_lifecycle;
mod runtime_mcp;
mod runtime_scheduler;
```

**Implementation dispersion principle**: Each sub-module uses `impl AgentRuntime { ... }` form. Rust allows `impl` blocks for the same type to be scattered across different files; struct definition remains in `runtime.rs`.

**Field access principle**: Methods in sub-modules access `AgentRuntime` fields via `pub(crate)` visibility. All fields declared as `pub(crate)` in `runtime.rs`, limited to crate visibility.

Example (`runtime_lifecycle.rs`):

```rust
use super::runtime::AgentRuntime;

impl AgentRuntime {
    pub async fn start(&self, ...) -> Result<AgentExecutorHandle, AgentError> {
        // Directly access self.catalog, self.primary_handle, etc. pub(crate) fields
    }
}
```

## Consequences

### Positive

- Each file focuses on single responsibility, code navigation efficiency improved
- `runtime.rs` reduced from 500+ lines to ~250 lines
- Each responsibility can evolve independently: MCP management logic changes don't affect agent lifecycle code git history
- Reduced merge conflicts during parallel development
- Can write tests for `runtime_scheduler.rs` independently without depending on MCP code

### Negative

- Each sub-module file needs `use super::runtime::AgentRuntime` import at the top, adding some boilerplate
- `pub(crate)` field exposure increases crate-internal visibility scope, depends on code review to maintain encapsulation
- Readers need to jump between multiple files for complete view when first reading code

### Neutral

- Rust's `impl` dispersion mechanism is a native language feature, no new architectural abstraction introduced
- `AgentRuntime` public API (method signatures) has no changes, completely transparent to external callers
- All sub-module files use `//!` doc comments to declare module responsibilities for quick navigation

## References

- Code paths: `crates/octo-engine/src/agent/runtime.rs`, `runtime_lifecycle.rs`, `runtime_mcp.rs`, `runtime_scheduler.rs`, `mod.rs`
- Related: ADR-007-MCP (MCP call_mcp_tool Lock-Free I/O)
