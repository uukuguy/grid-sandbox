/// ADR-V2-020 — tool namespace contract tests (S1.T2)
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use grid_engine::tools::{Tool, ToolLayer, ToolRegistry};
use grid_types::{RiskLevel, ToolContext, ToolOutput, ToolSource};

// ── Minimal stubs ──────────────────────────────────────────────────────────

struct StubTool {
    name: &'static str,
    layer: ToolLayer,
}

#[async_trait]
impl Tool for StubTool {
    fn name(&self) -> &str {
        self.name
    }
    fn description(&self) -> &str {
        "stub"
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({})
    }
    async fn execute(&self, _p: serde_json::Value, _c: &ToolContext) -> Result<ToolOutput> {
        Ok(ToolOutput::success("stub"))
    }
    fn source(&self) -> ToolSource {
        ToolSource::BuiltIn
    }
    fn layer(&self) -> ToolLayer {
        self.layer
    }
}

fn make(name: &'static str, layer: ToolLayer) -> Arc<dyn Tool> {
    Arc::new(StubTool { name, layer })
}

// ── ToolLayer tests ────────────────────────────────────────────────────────

#[test]
fn test_tool_layer_prefixes() {
    assert_eq!(ToolLayer::L0.prefix(), "l0");
    assert_eq!(ToolLayer::L1.prefix(), "l1");
    assert_eq!(ToolLayer::L2.prefix(), "l2");
}

#[test]
fn test_tool_layer_from_prefix() {
    assert_eq!(ToolLayer::from_prefix("l0"), Some(ToolLayer::L0));
    assert_eq!(ToolLayer::from_prefix("l1"), Some(ToolLayer::L1));
    assert_eq!(ToolLayer::from_prefix("l2"), Some(ToolLayer::L2));
    assert_eq!(ToolLayer::from_prefix("l3"), None);
    assert_eq!(ToolLayer::from_prefix("L1"), None);
    assert_eq!(ToolLayer::from_prefix(""), None);
}

// ── ToolRegistry::register_layered + resolve ───────────────────────────────

#[test]
fn test_register_layered_resolve_qualified() {
    let mut reg = ToolRegistry::new();
    reg.register_layered(ToolLayer::L1, make("bash.execute", ToolLayer::L1));

    let tool = reg.resolve("l1:bash.execute").unwrap();
    assert_eq!(tool.name(), "bash.execute");
    assert_eq!(tool.layer(), ToolLayer::L1);
}

#[test]
fn test_register_layered_resolve_bare_name() {
    let mut reg = ToolRegistry::new();
    reg.register_layered(ToolLayer::L2, make("memory.search", ToolLayer::L2));

    // bare name still resolves (backward compat via direct map key)
    let tool = reg.resolve("memory.search").unwrap();
    assert_eq!(tool.name(), "memory.search");
}

#[test]
fn test_resolve_nonexistent_returns_none() {
    let reg = ToolRegistry::new();
    assert!(reg.resolve("l1:nonexistent").is_none());
}

// ── ToolRegistry::resolve_with_fallback ───────────────────────────────────

#[test]
fn test_fallback_l2_wins_over_l1() {
    let mut reg = ToolRegistry::new();
    reg.register_layered(ToolLayer::L1, make("memory.search", ToolLayer::L1));
    reg.register_layered(ToolLayer::L2, make("memory.search", ToolLayer::L2));

    // L2 must win (pre-Phase 3 fallback chain: l2 > l1 > l0)
    let found = reg.resolve_with_fallback("memory.search").unwrap();
    assert_eq!(found.layer(), ToolLayer::L2);
}

#[test]
fn test_fallback_l1_when_no_l2() {
    let mut reg = ToolRegistry::new();
    reg.register_layered(ToolLayer::L1, make("bash.execute", ToolLayer::L1));

    let found = reg.resolve_with_fallback("bash.execute").unwrap();
    assert_eq!(found.layer(), ToolLayer::L1);
}

#[test]
fn test_fallback_bare_name_final_fallback() {
    let mut reg = ToolRegistry::new();
    // registered via legacy register() (no layer key)
    reg.register(StubTool { name: "legacy_tool", layer: ToolLayer::L1 });

    let found = reg.resolve_with_fallback("legacy_tool").unwrap();
    assert_eq!(found.name(), "legacy_tool");
}

#[test]
fn test_fallback_returns_none_if_not_found() {
    let reg = ToolRegistry::new();
    assert!(reg.resolve_with_fallback("does_not_exist").is_none());
}

// ── Skill-declared namespace wins over runtime default ────────────────────

#[test]
fn test_explicit_l2_wins_over_l1_when_both_registered() {
    let mut reg = ToolRegistry::new();
    reg.register_layered(ToolLayer::L1, make("memory.search", ToolLayer::L1));
    reg.register_layered(ToolLayer::L2, make("memory.search", ToolLayer::L2));

    // skill declared "l2:memory.search" → direct resolve wins
    let tool = reg.resolve("l2:memory.search").unwrap();
    assert_eq!(tool.layer(), ToolLayer::L2);

    // skill declared "l1:memory.search" → direct resolve wins
    let tool = reg.resolve("l1:memory.search").unwrap();
    assert_eq!(tool.layer(), ToolLayer::L1);
}
