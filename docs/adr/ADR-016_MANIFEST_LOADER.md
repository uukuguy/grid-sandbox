# ADR-016: ManifestLoader YAML Declarative Agent

## Status
Completed

## Context

Users need to define Agent capabilities in declarative YAML manner, not hardcoded.

## Decision

Implement `ManifestLoader` to load Agent definition from YAML files:

```yaml
name: coder
description: Code writing agent
capabilities:
  - code_generation
  - refactoring
  - bug_fix
model_preference: sonnet
max_tokens: 8192
```

## References

- Code paths: `src/agent/manifest_loader.rs`, `src/agent/config.rs` (AgentManifest struct)
