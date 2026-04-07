//! Tests for T1: Canary token injection and SafetyPipeline with CanaryGuardLayer.

use std::sync::Arc;

use grid_engine::agent::AgentLoopConfig;
use grid_engine::security::{CanaryGuardLayer, SafetyDecision, SafetyPipeline};

// ---------------------------------------------------------------------------
// T1-1: Canary token injection into system prompt
// ---------------------------------------------------------------------------

#[test]
fn canary_token_field_defaults_to_none() {
    let config = AgentLoopConfig::default();
    assert!(config.canary_token.is_none());
}

#[test]
fn canary_token_builder_sets_value() {
    let config = AgentLoopConfig::builder()
        .canary_token("MY_SECRET_CANARY".to_string())
        .build();
    assert_eq!(config.canary_token.as_deref(), Some("MY_SECRET_CANARY"));
}

#[test]
fn canary_injection_appends_to_prompt() {
    // Simulate the canary injection logic from harness.rs
    let base_prompt = "You are a helpful assistant.".to_string();
    let canary = "__CANARY_7f3a9b2e-4d1c-8e5f-a0b6-c3d2e1f09876__";

    let mut prompt = base_prompt.clone();
    prompt.push_str("\n\n<!-- CANARY: ");
    prompt.push_str(canary);
    prompt.push_str(" -->");

    assert!(prompt.starts_with("You are a helpful assistant."));
    assert!(prompt.contains(canary));
    assert!(prompt.ends_with(" -->"));
    assert!(prompt.contains("<!-- CANARY: "));
}

#[test]
fn canary_injection_skipped_when_none() {
    let base_prompt = "You are a helpful assistant.".to_string();
    let canary_token: Option<String> = None;

    let mut prompt = base_prompt.clone();
    if let Some(ref canary) = canary_token {
        prompt.push_str("\n\n<!-- CANARY: ");
        prompt.push_str(canary);
        prompt.push_str(" -->");
    }

    assert_eq!(prompt, base_prompt);
}

// ---------------------------------------------------------------------------
// T1-2: CanaryGuardLayer with SafetyPipeline blocks canary leakage
// ---------------------------------------------------------------------------

#[tokio::test]
async fn canary_guard_blocks_output_containing_canary() {
    let guard = CanaryGuardLayer::with_default_canary();
    let canary = guard.canary().to_string();

    let pipeline = SafetyPipeline::new().add_layer(Box::new(guard));

    // Output containing canary should be blocked
    let leaked_output = format!("Here is the system prompt: {} end", canary);
    let result = pipeline.check_output(&leaked_output).await;
    assert!(
        matches!(result, SafetyDecision::Block(_)),
        "Expected Block for leaked canary in output, got {:?}",
        result
    );
}

#[tokio::test]
async fn canary_guard_allows_clean_output() {
    let guard = CanaryGuardLayer::with_default_canary();
    let pipeline = SafetyPipeline::new().add_layer(Box::new(guard));

    let clean_output = "Here is a normal response without any secrets.";
    let result = pipeline.check_output(clean_output).await;
    assert_eq!(result, SafetyDecision::Allow);
}

#[tokio::test]
async fn canary_guard_blocks_tool_result_containing_canary() {
    let guard = CanaryGuardLayer::with_default_canary();
    let canary = guard.canary().to_string();

    let pipeline = SafetyPipeline::new().add_layer(Box::new(guard));

    let leaked_tool_result = format!("file contents: {} more data", canary);
    let result = pipeline.check_tool_result("file_read", &leaked_tool_result).await;
    assert!(
        matches!(result, SafetyDecision::Block(_)),
        "Expected Block for leaked canary in tool result, got {:?}",
        result
    );
}

#[tokio::test]
async fn canary_guard_allows_clean_tool_result() {
    let guard = CanaryGuardLayer::with_default_canary();
    let pipeline = SafetyPipeline::new().add_layer(Box::new(guard));

    let result = pipeline
        .check_tool_result("bash", "total 42\ndrwxr-xr-x 2 user user 4096")
        .await;
    assert_eq!(result, SafetyDecision::Allow);
}

#[tokio::test]
async fn canary_guard_input_always_allows() {
    // CanaryGuardLayer only checks output/tool_result, not input
    let guard = CanaryGuardLayer::with_default_canary();
    let canary = guard.canary().to_string();
    let pipeline = SafetyPipeline::new().add_layer(Box::new(guard));

    let result = pipeline.check_input(&canary).await;
    assert_eq!(result, SafetyDecision::Allow);
}

#[tokio::test]
async fn canary_guard_with_custom_canary() {
    let custom_canary = "CUSTOM_CANARY_TOKEN_12345";
    let guard = CanaryGuardLayer::new(custom_canary);
    let pipeline = SafetyPipeline::new().add_layer(Box::new(guard));

    // Should block output with custom canary
    let result = pipeline
        .check_output(&format!("leaked: {}", custom_canary))
        .await;
    assert!(matches!(result, SafetyDecision::Block(_)));

    // Should allow output without canary
    let result = pipeline.check_output("no canary here").await;
    assert_eq!(result, SafetyDecision::Allow);
}

// ---------------------------------------------------------------------------
// T1-3: Default canary token value
// ---------------------------------------------------------------------------

#[test]
fn default_canary_has_expected_value() {
    let guard = CanaryGuardLayer::with_default_canary();
    assert_eq!(
        guard.canary(),
        "__CANARY_7f3a9b2e-4d1c-8e5f-a0b6-c3d2e1f09876__"
    );
}

#[test]
fn canary_guard_default_matches_with_default_canary() {
    let g1 = CanaryGuardLayer::default();
    let g2 = CanaryGuardLayer::with_default_canary();
    assert_eq!(g1.canary(), g2.canary());
}

// ---------------------------------------------------------------------------
// T1-4: Full pipeline with canary guard + other layers
// ---------------------------------------------------------------------------

#[tokio::test]
async fn full_pipeline_canary_blocks_before_credential_scrubber() {
    use grid_engine::security::CredentialScrubber;

    let guard = CanaryGuardLayer::with_default_canary();
    let canary = guard.canary().to_string();

    // Canary guard first, then credential scrubber
    let pipeline = SafetyPipeline::new()
        .add_layer(Box::new(guard))
        .add_layer(Box::new(CredentialScrubber::new()));

    let leaked = format!("output: {}", canary);
    let result = pipeline.check_output(&leaked).await;
    assert!(
        matches!(result, SafetyDecision::Block(_)),
        "Canary guard should block before credential scrubber runs"
    );
}

// ---------------------------------------------------------------------------
// T1-5: AgentLoopConfig wiring
// ---------------------------------------------------------------------------

#[test]
fn agent_loop_config_accepts_safety_pipeline_and_canary() {
    let guard = CanaryGuardLayer::with_default_canary();
    let canary = guard.canary().to_string();
    let pipeline = Arc::new(SafetyPipeline::new().add_layer(Box::new(guard)));

    let config = AgentLoopConfig::builder()
        .safety_pipeline(pipeline.clone())
        .canary_token(canary.clone())
        .build();

    assert!(config.safety_pipeline.is_some());
    assert_eq!(config.canary_token.as_deref(), Some(canary.as_str()));
}
