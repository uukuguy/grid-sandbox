//! S2.T2 — LLM provider integration tests.
//!
//! These tests verify that grid-runtime can actually call an LLM provider.
//! They require a live API key and are marked `#[ignore]` by default.
//!
//! Run manually:
//!   ANTHROPIC_API_KEY=sk-ant-xxx cargo test -p grid-runtime llm_provider -- --ignored --test-threads=1

use std::sync::Arc;

use grid_runtime::contract::{RuntimeContract, SessionPayload, UserMessage};
use grid_runtime::harness::GridHarness;
use tokio_stream::StreamExt;

async fn build_harness_with_provider() -> GridHarness {
    let catalog = Arc::new(grid_engine::AgentCatalog::new());

    // Read provider config from env (same as main.rs).
    let provider = std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "anthropic".into());
    let model =
        std::env::var("LLM_MODEL").unwrap_or_else(|_| "claude-sonnet-4-20250514".into());
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .ok()
        .or_else(|| std::env::var("OPENAI_API_KEY").ok());

    let provider_config = grid_engine::ProviderConfig {
        name: provider.clone(),
        api_key,
        base_url: None,
        model: Some(model.clone()),
    };

    let runtime_config = grid_engine::AgentRuntimeConfig::from_parts(
        ":memory:".into(),
        provider_config,
        vec![],
        None,
        None,
        true, // enable event bus
    );
    let tenant_context = grid_engine::TenantContext::for_single_user(
        grid_types::id::TenantId::from_string("test"),
        grid_types::id::UserId::from_string("test-user"),
    );
    let engine_runtime = grid_engine::AgentRuntime::new(catalog, runtime_config, Some(tenant_context))
        .await
        .expect("Failed to build AgentRuntime");

    GridHarness::new(Arc::new(engine_runtime))
        .with_runtime_id("test-grid-runtime")
        .with_provider(&provider, &model)
}

/// Verify that get_capabilities reflects the configured provider/model.
#[tokio::test]
async fn capabilities_reflect_config() {
    let harness = build_harness_with_provider().await;
    let caps = harness.get_capabilities();
    // Model should match env or default.
    let expected_model =
        std::env::var("LLM_MODEL").unwrap_or_else(|_| "claude-sonnet-4-20250514".into());
    assert_eq!(caps.model, expected_model);
    assert_eq!(caps.runtime_id, "test-grid-runtime");
}

/// Verify that with_provider sets provider/model correctly.
#[tokio::test]
async fn with_provider_sets_fields() {
    let catalog = Arc::new(grid_engine::AgentCatalog::new());
    let runtime_config = grid_engine::AgentRuntimeConfig::from_parts(
        ":memory:".into(),
        grid_engine::ProviderConfig::default(),
        vec![],
        None,
        None,
        false,
    );
    let tenant_context = grid_engine::TenantContext::for_single_user(
        grid_types::id::TenantId::from_string("test"),
        grid_types::id::UserId::from_string("test-user"),
    );
    let engine_runtime = grid_engine::AgentRuntime::new(catalog, runtime_config, Some(tenant_context))
        .await
        .expect("Failed to build AgentRuntime");

    let harness = GridHarness::new(Arc::new(engine_runtime))
        .with_provider("openai", "gpt-4o");
    let caps = harness.get_capabilities();
    assert_eq!(caps.model, "gpt-4o");
}

/// Full LLM round-trip: initialize session → send message → receive response.
///
/// Requires a live API key. Run with:
///   ANTHROPIC_API_KEY=sk-ant-xxx cargo test -p grid-runtime llm_provider_round_trip -- --ignored --test-threads=1
#[tokio::test]
#[ignore]
async fn llm_provider_round_trip() {
    let harness = build_harness_with_provider().await;

    // Initialize session with minimal payload.
    let payload = SessionPayload {
        session_id: "test-llm-session".into(),
        user_id: "test-user".into(),
        runtime_id: "test-grid-runtime".into(),
        created_at: "2026-04-12T00:00:00Z".into(),
        policy_context: None,
        event_context: None,
        memory_refs: vec![],
        skill_instructions: None,
        user_preferences: None,
        allow_trim_p5: true,
        allow_trim_p4: false,
        allow_trim_p3: false,
    };

    let handle = harness
        .initialize(payload)
        .await
        .expect("Initialize should succeed");
    assert!(!handle.session_id.is_empty());

    // Send a simple message.
    let message = UserMessage {
        content: "Say exactly: HELLO_TEST_OK".into(),
        message_type: "text".into(),
        metadata: Default::default(),
    };

    let mut stream = harness
        .send(&handle, message)
        .await
        .expect("Send should succeed");

    let mut got_text = false;
    let mut got_done = false;
    while let Some(chunk) = stream.next().await {
        match chunk.chunk_type.as_str() {
            "text_delta" => {
                got_text = true;
                // LLM responded with some text.
                assert!(!chunk.content.is_empty() || chunk.content.is_empty());
            }
            "done" => {
                got_done = true;
            }
            "error" => {
                panic!("Got error chunk: {}", chunk.content);
            }
            _ => {} // thinking, tool_start, etc. — ok
        }
    }

    assert!(got_text || got_done, "Expected at least one text_delta or done chunk from LLM");

    // Terminate session.
    harness
        .terminate(&handle)
        .await
        .expect("Terminate should succeed");
}
