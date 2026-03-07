# ADR-002: BashTool ExecPolicy Default Enabled

## Status
Accepted

## Context

`BashTool` contains an `exec_policy: Option<ExecPolicy>` field. `ExecPolicy` defines three security modes:

- `Deny`: Block all shell execution
- `Allowlist` (default): Only allow whitelisted commands, block shell metacharacters
- `Full`: Allow all commands (development mode)

The problem is that `BashTool::new()` sets `exec_policy` to `Some(ExecPolicy::default())`, but when callers create tools via `default_tools()`, the actual value was previously set to `None` before this fix.

Additionally, `ExecPolicy::is_allowed()` has incomplete metacharacter detection — the original version only checks for `;`, `|`, `&&`, `||`, `$(`, and backticks, missing these high-risk characters:

- `>`: Output redirection, can overwrite any file
- `<`: Input redirection, can read any file content
- `\n` (newline): Multi-command injection, can bypass single-line whitelist checks
- `\0` (null byte): Command truncation, can bypass string comparisons

All these metacharacters can be exploited by malicious LLM outputs to bypass the whitelist.

## Decision

**Decision 1**: `BashTool::new()` initializes `exec_policy` as `Some(ExecPolicy::default())`, ensuring all `BashTool` instances created via `default_tools()` have Allowlist mode enabled by default:

```rust
pub fn new() -> Self {
    Self {
        exec_policy: Some(ExecPolicy::default()),
        // ...
    }
}
```

**Decision 2**: Add detection for `>`, `<`, `\n`, `\0` in `ExecPolicy::is_allowed()` Allowlist branch:

```rust
ExecSecurityMode::Allowlist => {
    if command.contains(';')
        || command.contains('|')
        || command.contains("&&")
        || command.contains("||")
        || command.contains("$(")
        || command.contains('`')
        || command.contains('>')   // NEW: output redirection
        || command.contains('<')   // NEW: input redirection
        || command.contains('\n')  // NEW: newline injection
        || command.contains('\0')  // NEW: null byte truncation
    {
        return false;
    }
    // ...
}
```

## Consequences

### Positive

- All bash commands executed by agents are controlled by Allowlist by default, significantly reducing attack surface
- Redirection attacks (overwriting `/etc/passwd`, etc.) are blocked by metacharacter detection
- Newline injection (`cmd1\ncmd2`) cannot bypass single-command whitelist
- Null byte truncation attacks are prevented

### Negative

- Legitimate output redirection commands (e.g., `echo hello > /tmp/test`) will be rejected; use `file_write` tool instead for file writing
- Pipe commands (`ls | grep foo`) are blocked, some debugging scenarios are limited
- To use `>` or `<` operations, must explicitly use `ExecSecurityMode::Full` or extend the whitelist

### Neutral

- `BashTool::with_policy(policy)` method is preserved for custom policy scenarios
- `ExecSecurityMode::Full` mode still exists for developers to explicitly enable in trusted environments

## References

- Code paths: `crates/octo-engine/src/tools/bash.rs`
- Related: ADR-001 (PathValidator)
