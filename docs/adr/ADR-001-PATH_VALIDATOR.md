# ADR-001: PathValidator Security Policy Injection

## Status
Accepted

## Context
`ToolContext` contains `path_validator: Option<Arc<dyn PathValidator>>`, and `SecurityPolicy` implements the `PathValidator` trait for workspace boundary checks. However, before this refactoring, `AgentRuntime` did not hold a `SecurityPolicy` instance, nor was it injected when creating `ToolContext`, resulting in `path_validator` always being `None`.

Consequences:
- File tools (`file_read`, `file_write`, `file_edit`) had no security restrictions
- Agents could access any filesystem path including sensitive directories like `~/.ssh`, `/etc/passwd`
- Security policies defined in `SecurityPolicy` were ineffective

## Decision
Create `SecurityPolicy` instance in `AgentRuntime::new()` with the current working directory as workspace, and inject it into `ToolContext` at all creation points.

**Implementation Changes:**

1. **`runtime.rs`**: Add `security_policy: Arc<SecurityPolicy>` field, initialized in constructor step 15

2. **`executor.rs`**: Receive `path_validator: Option<Arc<dyn PathValidator>>` parameter in `AgentExecutor::new()`, inject when building `ToolContext`

3. **`runtime.rs`** (`start_primary`): Pass `security_policy` down to `AgentExecutor::new()`

4. **`runtime_scheduler.rs`** (`execute_scheduled_task`): Inject `path_validator` to scheduled task `ToolContext`

## Consequences

### Positive
- File tools now enforce workspace directory boundaries, preventing path traversal attacks
- System directories declared in `SecurityPolicy::forbidden_paths` are actually blocked
- Security policy bound to workspace, clear tenant isolation boundaries in multi-tenant scenarios
- Scheduled tasks use the same path validation logic as interactive sessions

### Negative
- Breaking impact on existing callers using absolute paths outside workspace
- Misconfiguration of `working_dir` may cause false positives
- `workspace_only: true` is default, test environments need extra configuration

### Neutral
- Publicly expose `security_policy()` getter for API layer to read current security config
- `PathValidator` trait injected as `dyn Trait`, maintains interface decoupling for test replacement

## References
- Code paths: `crates/octo-engine/src/agent/executor.rs`, `crates/octo-engine/src/agent/runtime.rs`, `crates/octo-engine/src/security/policy.rs`
- Related: ADR-002, ADR-003
