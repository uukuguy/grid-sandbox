# DDD Model: Security Policy Context

**Project**: octo-sandbox
**Bounded Context**: Security Policy
**Status**: Implemented

---

## Ubiquitous Language

| Term | Definition |
|------|------------|
| SecurityPolicy | Aggregate root for security rules |
| AutonomyLevel | Enum: ReadOnly, Supervised, Autonomous |
| CommandRiskLevel | Enum: Low, Medium, High |
| ActionTracker | Tracks action frequency for rate limiting |
| ExecPolicy | Shell execution policy |
| PathValidator | Trait for path access validation |

---

## Aggregates

### SecurityPolicy (Aggregate Root)

```rust
pub struct SecurityPolicy {
    pub autonomy_level: AutonomyLevel,
    pub allowed_commands: Vec<String>,
    pub blocked_commands: Vec<String>,
    pub workspace_only: bool,
    pub workspace_dir: PathBuf,
    pub max_actions_per_hour: u32,
    pub block_high_risk_commands: bool,
}
```

**Responsibilities**:
- Central security rule enforcement
- Command whitelist/blacklist management
- Path access control

---

## Value Objects

### AutonomyLevel

```rust
pub enum AutonomyLevel {
    ReadOnly,     // No command execution
    Supervised,   // Medium/High risk requires approval
    Autonomous,   // Full execution with monitoring
}
```

### CommandRiskLevel

```rust
pub enum CommandRiskLevel {
    Low,      // Read operations, safe tools
    Medium,   // File operations, network calls
    High,     // System modifications, destructive commands
}
```

### ActionTracker

```rust
pub struct ActionTracker {
    actions: HashMap<String, u32>,
    window: Duration,
}
```

---

## Domain Services

### PathValidator

```rust
pub trait PathValidator {
    fn validate_path(&self, path: &Path) -> Result<()>;
    fn validate_command(&self, cmd: &str) -> Result<()>;
}
```

---

## Invariants

1. **ReadOnly Isolation**: No commands execute when autonomy_level = ReadOnly
2. **Approval Requirement**: Medium/High risk requires approval when autonomy_level = Supervised
3. **Path Containment**: Path access limited to workspace when workspace_only = true
4. **Rate Limiting**: Actions capped at max_actions_per_hour

---

## Dependencies

- **Tool Context**: Uses PathValidator for tool execution
- **Agent Context**: Uses AutonomyLevel for agent behavior control

---

## References

- ADR-001: PathValidator Security Policy Injection
- ADR-002: BashTool ExecPolicy Default Enabled
- ADR-006-HMAC: HMAC Secret Force Check (Fail-Fast)
- [Enterprise Agent Sandbox Auth Design](../design/ENTERPRISE_AGENT_SANDBOX_AUTH_DESIGN.md)
