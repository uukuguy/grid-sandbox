# ADR-041: Skill System

## Status

Accepted

## Date

2026-03-07

## Context

The system requires a skill system for:
- Reusable agent capabilities
- Declarative skill definitions
- Skill discovery and matching
- Runtime skill invocation

## Decision

Implement skill system with YAML manifests:

### Core Components

```rust
// Skill manifest
pub struct SkillManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub triggers: Vec<Trigger>,
    pub actions: Vec<Action>,
    pub parameters: Vec<Parameter>,
}

// Skill loader
pub struct SkillLoader {
    fs: FileSystem,
    parser: YamlParser,
}

// Skill registry
pub struct SkillRegistry {
    skills: Arc<RwLock<HashMap<SkillId, SkillManifest>>>>,
}
```

### Skill Definition

```yaml
name: code_review
version: 1.0.0
description: Automated code review skill
triggers:
  - event: pr_created
actions:
  - type: llm_analysis
    model: claude-3-5-sonnet
  - type: post_comment
parameters:
  - name: pr_url
    required: true
```

### Trigger Types

| Trigger | Description |
|---------|-------------|
| Event | Based on system events |
| Intent | Based on user intent |
| Schedule | Cron-based execution |

## Consequences

### Positive

- Declarative skill definition
- Reusable capabilities
- Easy to add new skills

### Negative

- YAML parsing overhead
- Limited dynamic behavior

## Related

- [ADR-042: Skill Runtime](ADR-042-SKILL_SYSTEM.md)
