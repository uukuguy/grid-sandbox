# ADR-042: Skill Runtime

## Status

Accepted

## Date

2026-03-07

## Context

The system requires skill execution engine:
- Action execution pipeline
- Parameter validation
- Error handling
- Result handling

## Decision

Implement skill runtime:

### Core Components

```rust
// Skill runtime
pub struct SkillRuntime {
    executor: ActionExecutor,
    validator: ParameterValidator,
    context_builder: SkillContextBuilder,
}

// Skill context
pub struct SkillContext {
    pub skill_id: SkillId,
    pub parameters: HashMap<String, Value>,
    pub session: SessionRef,
    pub resources: ResourcePool,
}

// Action executor
pub struct ActionExecutor {
    handlers: Arc<RwLock<HashMap<ActionType, ActionHandler>>>,
}
```

### Action Types

| Action Type | Description |
|-------------|-------------|
| llm_analysis | LLM-based analysis |
| tool_call | Execute tool |
| http_request | Make HTTP request |
| post_comment | Post comment to PR |

### Execution Flow

1. **Validate**: Check parameter validity
2. **Build Context**: Create skill context
3. **Execute Actions**: Run action pipeline
4. **Handle Errors**: Error recovery
5. **Return Result**: Output processing

## Consequences

### Positive

- Flexible action pipeline
- Extensible action types
- Error recovery

### Negative

- Execution complexity
- Debugging challenges

## Related

- [ADR-041: Skill System](ADR-041-SKILL_SYSTEM.md)
