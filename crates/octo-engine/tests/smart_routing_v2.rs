//! Tests for SmartRouterProvider V2 — cross-provider routing.
//!
//! V2 extends V1 by allowing each complexity tier to route to a *different*
//! provider instance (e.g. Simple -> OpenAI, Complex -> Anthropic).

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use async_trait::async_trait;
use futures_util::Stream;
use octo_engine::providers::{
    Provider, QueryAnalyzer, QueryComplexity, SmartRouterProvider, SmartRoutingConfig, TierConfig,
};
use octo_types::{
    ChatMessage, CompletionRequest, CompletionResponse, ContentBlock, StreamEvent, StopReason,
    TokenUsage, ToolSpec,
};

// ---------------------------------------------------------------------------
// Mock Provider — records the provider id and model it receives
// ---------------------------------------------------------------------------

struct MockProvider {
    provider_id: String,
    last_model: Arc<Mutex<String>>,
    last_provider_id: Arc<Mutex<String>>,
}

impl MockProvider {
    fn new(id: &str) -> (Self, Arc<Mutex<String>>, Arc<Mutex<String>>) {
        let model = Arc::new(Mutex::new(String::new()));
        let provider_id = Arc::new(Mutex::new(String::new()));
        (
            Self {
                provider_id: id.to_string(),
                last_model: Arc::clone(&model),
                last_provider_id: Arc::clone(&provider_id),
            },
            model,
            provider_id,
        )
    }

    fn new_simple(id: &str) -> Self {
        Self {
            provider_id: id.to_string(),
            last_model: Arc::new(Mutex::new(String::new())),
            last_provider_id: Arc::new(Mutex::new(String::new())),
        }
    }
}

#[async_trait]
impl Provider for MockProvider {
    fn id(&self) -> &str {
        &self.provider_id
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        *self.last_model.lock().unwrap() = request.model.clone();
        *self.last_provider_id.lock().unwrap() = self.provider_id.clone();
        Ok(CompletionResponse {
            id: format!("{}-response", self.provider_id),
            content: vec![ContentBlock::Text {
                text: format!("from-{}", self.provider_id),
            }],
            stop_reason: Some(StopReason::EndTurn),
            usage: TokenUsage {
                input_tokens: 10,
                output_tokens: 5,
            },
        })
    }

    async fn stream(
        &self,
        _request: CompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>>> {
        Err(anyhow::anyhow!("stream not implemented for mock"))
    }
}

// ---------------------------------------------------------------------------
// Shared mock provider that tracks which provider handled the request
// ---------------------------------------------------------------------------

struct TrackingProvider {
    id_str: String,
    calls: Arc<Mutex<Vec<(String, String)>>>, // (provider_id, model)
}

impl TrackingProvider {
    fn new(id: &str, calls: Arc<Mutex<Vec<(String, String)>>>) -> Self {
        Self {
            id_str: id.to_string(),
            calls,
        }
    }
}

#[async_trait]
impl Provider for TrackingProvider {
    fn id(&self) -> &str {
        &self.id_str
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        self.calls
            .lock()
            .unwrap()
            .push((self.id_str.clone(), request.model.clone()));
        Ok(CompletionResponse {
            id: "tracking-response".to_string(),
            content: vec![ContentBlock::Text {
                text: format!("from-{}", self.id_str),
            }],
            stop_reason: Some(StopReason::EndTurn),
            usage: TokenUsage {
                input_tokens: 10,
                output_tokens: 5,
            },
        })
    }

    async fn stream(
        &self,
        _request: CompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>>> {
        Err(anyhow::anyhow!("stream not implemented"))
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn default_analyzer() -> QueryAnalyzer {
    QueryAnalyzer::with_defaults()
}

fn make_request() -> CompletionRequest {
    CompletionRequest::default()
}

fn dummy_tool() -> ToolSpec {
    ToolSpec {
        name: "test_tool".to_string(),
        description: "A test tool".to_string(),
        input_schema: serde_json::json!({}),
    }
}

// ---------------------------------------------------------------------------
// 1. V2 cross-provider: simple request routes to OpenAI provider
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_v2_simple_routes_to_openai() {
    let calls = Arc::new(Mutex::new(Vec::new()));

    let inner = TrackingProvider::new("anthropic", Arc::clone(&calls));
    let openai = TrackingProvider::new("openai", Arc::clone(&calls));

    let mut tier_models = HashMap::new();
    tier_models.insert(QueryComplexity::Simple, "gpt-4o-mini".to_string());
    tier_models.insert(QueryComplexity::Medium, "claude-sonnet".to_string());
    tier_models.insert(QueryComplexity::Complex, "claude-opus".to_string());

    let mut tier_providers = HashMap::new();
    tier_providers.insert(QueryComplexity::Simple, Arc::new(openai) as Arc<dyn Provider>);

    let router = SmartRouterProvider::new_cross_provider(
        Box::new(inner),
        default_analyzer(),
        tier_models,
        tier_providers,
        "claude-sonnet".to_string(),
    );

    // Simple request -> should route to openai provider with gpt-4o-mini
    let mut req = make_request();
    req.messages.push(ChatMessage::user("hello"));
    let _resp = router.complete(req).await.unwrap();

    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "openai");
    assert_eq!(calls[0].1, "gpt-4o-mini");
    
}

// ---------------------------------------------------------------------------
// 2. V2 cross-provider: complex request stays with inner (anthropic)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_v2_complex_stays_with_inner() {
    let calls = Arc::new(Mutex::new(Vec::new()));

    let inner = TrackingProvider::new("anthropic", Arc::clone(&calls));
    let openai = TrackingProvider::new("openai", Arc::clone(&calls));

    let mut tier_models = HashMap::new();
    tier_models.insert(QueryComplexity::Simple, "gpt-4o-mini".to_string());
    tier_models.insert(QueryComplexity::Complex, "claude-opus".to_string());

    let mut tier_providers = HashMap::new();
    tier_providers.insert(QueryComplexity::Simple, Arc::new(openai) as Arc<dyn Provider>);

    let router = SmartRouterProvider::new_cross_provider(
        Box::new(inner),
        default_analyzer(),
        tier_models,
        tier_providers,
        "claude-sonnet".to_string(),
    );

    // Complex request -> should stay with inner (anthropic)
    let mut req = make_request();
    req.messages.push(ChatMessage::user(&"a".repeat(4000)));
    for _ in 0..6 {
        req.tools.push(dummy_tool());
    }
    req.messages
        .push(ChatMessage::user("Please architect the new system"));
    let _resp = router.complete(req).await.unwrap();

    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "anthropic");
    assert_eq!(calls[0].1, "claude-opus");
    
}

// ---------------------------------------------------------------------------
// 3. V2 cross-provider: medium request with no tier_provider falls to inner
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_v2_medium_falls_to_inner() {
    let calls = Arc::new(Mutex::new(Vec::new()));

    let inner = TrackingProvider::new("anthropic", Arc::clone(&calls));

    let mut tier_models = HashMap::new();
    tier_models.insert(QueryComplexity::Simple, "gpt-4o-mini".to_string());
    tier_models.insert(QueryComplexity::Medium, "claude-sonnet".to_string());

    let tier_providers = HashMap::new(); // No cross-provider mappings

    let router = SmartRouterProvider::new_cross_provider(
        Box::new(inner),
        default_analyzer(),
        tier_models,
        tier_providers,
        "claude-sonnet".to_string(),
    );

    // Medium request -> falls to inner
    let mut req = make_request();
    req.messages.push(ChatMessage::user(&"a".repeat(600)));
    req.tools.push(dummy_tool());
    req.tools.push(dummy_tool());
    let _ = router.complete(req).await.unwrap();

    let calls = calls.lock().unwrap();
    assert_eq!(calls[0].0, "anthropic");
    assert_eq!(calls[0].1, "claude-sonnet");
}

// ---------------------------------------------------------------------------
// 4. V2: is_cross_provider returns correct state
// ---------------------------------------------------------------------------

#[test]
fn test_is_cross_provider() {
    let inner = MockProvider::new_simple("default");

    // V1 mode
    let v1 = SmartRouterProvider::new(
        Box::new(inner),
        default_analyzer(),
        HashMap::new(),
        "default".to_string(),
    );
    assert!(!v1.is_cross_provider());

    // V2 mode
    let inner2 = MockProvider::new_simple("default");
    let openai = MockProvider::new_simple("openai");
    let mut tier_providers = HashMap::new();
    tier_providers.insert(QueryComplexity::Simple, Arc::new(openai) as Arc<dyn Provider>);
    let v2 = SmartRouterProvider::new_cross_provider(
        Box::new(inner2),
        default_analyzer(),
        HashMap::new(),
        tier_providers,
        "default".to_string(),
    );
    assert!(v2.is_cross_provider());
}

// ---------------------------------------------------------------------------
// 5. V1 backward compatibility: new() still works as before
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_v1_backward_compat() {
    let (mock, last_model, _) = MockProvider::new("default");
    let mut tier_models = HashMap::new();
    tier_models.insert(QueryComplexity::Simple, "claude-haiku".to_string());
    tier_models.insert(QueryComplexity::Medium, "claude-sonnet".to_string());
    tier_models.insert(QueryComplexity::Complex, "claude-opus".to_string());

    let router = SmartRouterProvider::new(
        Box::new(mock),
        default_analyzer(),
        tier_models,
        "claude-sonnet".to_string(),
    );

    // Should behave exactly like V1
    let mut req = make_request();
    req.messages.push(ChatMessage::user("hello"));
    let _ = router.complete(req).await.unwrap();
    assert_eq!(*last_model.lock().unwrap(), "claude-haiku");
}

// ---------------------------------------------------------------------------
// 6. Config deserialization: V1 config without provider field
// ---------------------------------------------------------------------------

#[test]
fn test_config_v1_deserialization() {
    let yaml = r#"
enabled: true
default_tier: medium
tiers:
  simple:
    model: claude-haiku
  medium:
    model: claude-sonnet
  complex:
    model: claude-opus
"#;
    let config: SmartRoutingConfig = serde_yaml::from_str(yaml).unwrap();
    assert!(config.enabled);
    assert!(!config.has_cross_provider_tiers());
    assert_eq!(config.tiers.len(), 3);
    assert!(config.tiers["simple"].provider.is_none());
    assert!(config.tiers["medium"].provider.is_none());
}

// ---------------------------------------------------------------------------
// 7. Config deserialization: V2 config with provider fields
// ---------------------------------------------------------------------------

#[test]
fn test_config_v2_deserialization() {
    let yaml = r#"
enabled: true
default_tier: medium
tiers:
  simple:
    model: gpt-4o-mini
    provider: openai
    api_key: sk-test-openai
  medium:
    model: claude-sonnet
  complex:
    model: claude-opus
    provider: anthropic
    api_key: sk-ant-test
    base_url: https://custom.anthropic.com
"#;
    let config: SmartRoutingConfig = serde_yaml::from_str(yaml).unwrap();
    assert!(config.enabled);
    assert!(config.has_cross_provider_tiers());

    // Simple tier has provider
    let simple = &config.tiers["simple"];
    assert_eq!(simple.model, "gpt-4o-mini");
    assert_eq!(simple.provider.as_deref(), Some("openai"));
    assert_eq!(simple.api_key.as_deref(), Some("sk-test-openai"));
    assert!(simple.base_url.is_none());

    // Medium tier has no provider (V1 fallback)
    let medium = &config.tiers["medium"];
    assert_eq!(medium.model, "claude-sonnet");
    assert!(medium.provider.is_none());

    // Complex tier has provider with base_url
    let complex = &config.tiers["complex"];
    assert_eq!(complex.model, "claude-opus");
    assert_eq!(complex.provider.as_deref(), Some("anthropic"));
    assert_eq!(
        complex.base_url.as_deref(),
        Some("https://custom.anthropic.com")
    );
}

// ---------------------------------------------------------------------------
// 8. Config build_provider V1 mode (no cross-provider tiers)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_config_build_provider_v1() {
    let yaml = r#"
enabled: true
default_tier: medium
tiers:
  simple:
    model: claude-haiku
  medium:
    model: claude-sonnet
  complex:
    model: claude-opus
"#;
    let config: SmartRoutingConfig = serde_yaml::from_str(yaml).unwrap();
    let (mock, last_model, _) = MockProvider::new("default");
    let provider = config.build_provider(Box::new(mock)).unwrap();

    // Simple request -> haiku
    let mut req = make_request();
    req.messages.push(ChatMessage::user("hi"));
    let _ = provider.complete(req).await.unwrap();
    assert_eq!(*last_model.lock().unwrap(), "claude-haiku");
}

// ---------------------------------------------------------------------------
// 9. Config build_provider disabled returns None
// ---------------------------------------------------------------------------

#[test]
fn test_config_build_provider_disabled() {
    let config = SmartRoutingConfig::default();
    let mock = MockProvider::new_simple("default");
    assert!(config.build_provider(Box::new(mock)).is_none());
}

// ---------------------------------------------------------------------------
// 10. Config build_cross_provider with external providers
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_config_build_cross_provider() {
    let yaml = r#"
enabled: true
default_tier: medium
tiers:
  simple:
    model: gpt-4o-mini
  medium:
    model: claude-sonnet
  complex:
    model: claude-opus
"#;
    let config: SmartRoutingConfig = serde_yaml::from_str(yaml).unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));

    let inner = TrackingProvider::new("anthropic", Arc::clone(&calls));
    let openai = TrackingProvider::new("openai", Arc::clone(&calls));

    let mut tier_providers = HashMap::new();
    tier_providers.insert(
        QueryComplexity::Simple,
        Arc::new(openai) as Arc<dyn Provider>,
    );

    let provider = config
        .build_cross_provider(Box::new(inner), tier_providers)
        .unwrap();

    // Simple request -> openai
    let mut req = make_request();
    req.messages.push(ChatMessage::user("hello"));
    let _ = provider.complete(req).await.unwrap();

    let calls = calls.lock().unwrap();
    assert_eq!(calls[0].0, "openai");
    assert_eq!(calls[0].1, "gpt-4o-mini");
}

// ---------------------------------------------------------------------------
// 11. Config build_cross_provider disabled returns None
// ---------------------------------------------------------------------------

#[test]
fn test_config_build_cross_provider_disabled() {
    let config = SmartRoutingConfig::default();
    let mock = MockProvider::new_simple("default");
    assert!(config
        .build_cross_provider(Box::new(mock), HashMap::new())
        .is_none());
}

// ---------------------------------------------------------------------------
// 12. V2 cross-provider: all three tiers to different providers
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_v2_all_tiers_different_providers() {
    let calls = Arc::new(Mutex::new(Vec::new()));

    let inner = TrackingProvider::new("default-inner", Arc::clone(&calls));
    let openai = TrackingProvider::new("openai", Arc::clone(&calls));
    let anthropic = TrackingProvider::new("anthropic", Arc::clone(&calls));
    let deepseek = TrackingProvider::new("deepseek", Arc::clone(&calls));

    let mut tier_models = HashMap::new();
    tier_models.insert(QueryComplexity::Simple, "gpt-4o-mini".to_string());
    tier_models.insert(QueryComplexity::Medium, "claude-sonnet".to_string());
    tier_models.insert(QueryComplexity::Complex, "deepseek-v3".to_string());

    let mut tier_providers = HashMap::new();
    tier_providers.insert(QueryComplexity::Simple, Arc::new(openai) as Arc<dyn Provider>);
    tier_providers.insert(
        QueryComplexity::Medium,
        Arc::new(anthropic) as Arc<dyn Provider>,
    );
    tier_providers.insert(
        QueryComplexity::Complex,
        Arc::new(deepseek) as Arc<dyn Provider>,
    );

    let router = SmartRouterProvider::new_cross_provider(
        Box::new(inner),
        default_analyzer(),
        tier_models,
        tier_providers,
        "claude-sonnet".to_string(),
    );

    // 1. Simple -> openai
    let mut req = make_request();
    req.messages.push(ChatMessage::user("hello"));
    let _ = router.complete(req).await.unwrap();

    // 2. Medium -> anthropic
    let mut req2 = make_request();
    req2.messages.push(ChatMessage::user(&"a".repeat(600)));
    req2.tools.push(dummy_tool());
    req2.tools.push(dummy_tool());
    let _ = router.complete(req2).await.unwrap();

    // 3. Complex -> deepseek
    let mut req3 = make_request();
    req3.messages.push(ChatMessage::user(&"a".repeat(4000)));
    for _ in 0..6 {
        req3.tools.push(dummy_tool());
    }
    req3.messages
        .push(ChatMessage::user("Please architect the new system"));
    let _ = router.complete(req3).await.unwrap();

    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[0], ("openai".to_string(), "gpt-4o-mini".to_string()));
    assert_eq!(
        calls[1],
        ("anthropic".to_string(), "claude-sonnet".to_string())
    );
    assert_eq!(
        calls[2],
        ("deepseek".to_string(), "deepseek-v3".to_string())
    );
}

// ---------------------------------------------------------------------------
// 13. V2: fallback model for unmapped tier
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_v2_fallback_model_unmapped_tier() {
    let calls = Arc::new(Mutex::new(Vec::new()));

    let inner = TrackingProvider::new("default", Arc::clone(&calls));
    let openai = TrackingProvider::new("openai", Arc::clone(&calls));

    // Only map Simple tier
    let mut tier_models = HashMap::new();
    tier_models.insert(QueryComplexity::Simple, "gpt-4o-mini".to_string());

    let mut tier_providers = HashMap::new();
    tier_providers.insert(QueryComplexity::Simple, Arc::new(openai) as Arc<dyn Provider>);

    let router = SmartRouterProvider::new_cross_provider(
        Box::new(inner),
        default_analyzer(),
        tier_models,
        tier_providers,
        "fallback-model".to_string(),
    );

    // Medium request -> not in tier_models, uses fallback model + inner provider
    let mut req = make_request();
    req.messages.push(ChatMessage::user(&"a".repeat(600)));
    req.tools.push(dummy_tool());
    req.tools.push(dummy_tool());
    let _ = router.complete(req).await.unwrap();

    let calls = calls.lock().unwrap();
    assert_eq!(calls[0].0, "default");
    assert_eq!(calls[0].1, "fallback-model");
}

// ---------------------------------------------------------------------------
// 14. RouteDecision debug formatting
// ---------------------------------------------------------------------------

#[test]
fn test_route_decision_debug() {
    use octo_engine::providers::RouteDecision;

    let decision = RouteDecision {
        provider: None,
        model: "claude-sonnet".to_string(),
    };
    let debug_str = format!("{:?}", decision);
    assert!(debug_str.contains("claude-sonnet"));
    assert!(debug_str.contains("None"));
}

// ---------------------------------------------------------------------------
// 15. TierConfig serialization round-trip
// ---------------------------------------------------------------------------

#[test]
fn test_tier_config_roundtrip() {
    let cfg = TierConfig {
        model: "gpt-4o-mini".to_string(),
        provider: Some("openai".to_string()),
        api_key: None,
        base_url: Some("https://api.openai.com".to_string()),
    };
    let yaml = serde_yaml::to_string(&cfg).unwrap();
    let parsed: TierConfig = serde_yaml::from_str(&yaml).unwrap();
    assert_eq!(parsed.model, "gpt-4o-mini");
    assert_eq!(parsed.provider.as_deref(), Some("openai"));
    assert!(parsed.api_key.is_none());
    assert_eq!(
        parsed.base_url.as_deref(),
        Some("https://api.openai.com")
    );
}

// ---------------------------------------------------------------------------
// 16. TierConfig: skip_serializing_if works for None fields
// ---------------------------------------------------------------------------

#[test]
fn test_tier_config_skip_none_serialization() {
    let cfg = TierConfig {
        model: "claude-haiku".to_string(),
        provider: None,
        api_key: None,
        base_url: None,
    };
    let yaml = serde_yaml::to_string(&cfg).unwrap();
    // Should only contain "model" field
    assert!(yaml.contains("model"));
    assert!(!yaml.contains("provider"));
    assert!(!yaml.contains("api_key"));
    assert!(!yaml.contains("base_url"));
}

// ---------------------------------------------------------------------------
// 17. SmartRoutingConfig has_cross_provider_tiers with mixed tiers
// ---------------------------------------------------------------------------

#[test]
fn test_has_cross_provider_tiers_mixed() {
    let mut tiers = HashMap::new();
    tiers.insert(
        "simple".to_string(),
        TierConfig {
            model: "gpt-4o-mini".to_string(),
            provider: Some("openai".to_string()),
            api_key: None,
            base_url: None,
        },
    );
    tiers.insert(
        "medium".to_string(),
        TierConfig {
            model: "claude-sonnet".to_string(),
            provider: None,
            api_key: None,
            base_url: None,
        },
    );
    let config = SmartRoutingConfig {
        enabled: true,
        default_tier: "medium".to_string(),
        tiers,
        thresholds: None,
    };
    assert!(config.has_cross_provider_tiers());
}
