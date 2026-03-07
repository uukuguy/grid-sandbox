# ADR-036: Extension System

## Status

Accepted

## Date

2026-03-07

## Context

The system requires a plugin architecture for extensibility:
- Load WASM-based plugins at runtime
- Sandboxed plugin execution
- Host function interception
- Plugin lifecycle management

## Decision

Implement WASM-based extension system:

### Core Architecture

```rust
// Extension host
pub struct ExtensionHost {
    engine: WasmtimeEngine,
    plugins: Arc<RwLock<HashMap<PluginId, LoadedPlugin>>>,
    host_functions: HostFunctionRegistry,
}

// Loaded plugin
pub struct LoadedPlugin {
    id: PluginId,
    name: String,
    instance: WasmtimeInstance,
    exports: PluginExports,
}

// Host function registry
pub struct HostFunctionRegistry {
    functions: Arc<RwLock<HashMap<String, HostFunction>>>,
}
```

### Plugin Interface

```rust
// Plugin trait
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn initialize(&mut self, ctx: &PluginContext) -> Result<()>;
    fn execute(&self, input: &PluginInput) -> Result<PluginOutput>;
}

// Plugin context
pub struct PluginContext {
    pub config: PluginConfig,
    pub http_client: HttpClient,
    pub logger: Logger,
}
```

### Host Call Interception

- File system access
- Network requests
- Environment variables
- Tool execution

## Consequences

### Positive

- Runtime plugin loading
- Strong isolation via WASM
- Language-agnostic plugins

### Negative

- WASM compilation complexity
- Performance overhead
- Debugging challenges

## Related

- [ADR-035: Sandbox System](ADR-035-SANDBOX_SYSTEM.md)
