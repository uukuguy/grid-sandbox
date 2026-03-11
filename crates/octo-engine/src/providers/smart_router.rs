//! Smart routing -- query complexity classification + model routing.
//!
//! Automatically selects the optimal LLM model based on input complexity:
//! - Simple  -> lightweight model (e.g., Haiku)
//! - Medium  -> mid-tier model (e.g., Sonnet)
//! - Complex -> heavyweight model (e.g., Opus)
//!
//! The [`QueryAnalyzer`] is a pure CPU heuristic classifier (<1us).
//! [`SmartRouterProvider`] wraps an inner [`Provider`] and overrides
//! the request model based on the analyzed complexity tier.
//!
//! ## V2 Cross-Provider Routing
//!
//! When [`TierConfig`] includes a `provider` field, each complexity tier can
//! route to a *different* provider instance entirely (e.g. Simple -> OpenAI,
//! Complex -> Anthropic). Tiers without a `provider` field fall back to the
//! default inner provider (V1 behavior).

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::debug;

use octo_types::{CompletionRequest, CompletionResponse, MessageRole};

use super::traits::{CompletionStream, Provider};

// ---------------------------------------------------------------------------
// Query Complexity
// ---------------------------------------------------------------------------

/// Complexity tier for a given query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QueryComplexity {
    /// Short text, no tools, simple greetings.
    Simple,
    /// Moderate text/tools, typical conversation.
    Medium,
    /// Long text, many tools, architect-level keywords.
    Complex,
}

impl std::fmt::Display for QueryComplexity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Simple => write!(f, "simple"),
            Self::Medium => write!(f, "medium"),
            Self::Complex => write!(f, "complex"),
        }
    }
}

// ---------------------------------------------------------------------------
// Query Analyzer
// ---------------------------------------------------------------------------

/// Configurable thresholds for the complexity scoring system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzerThresholds {
    #[serde(default = "default_text_length_medium")]
    pub text_length_medium: usize,
    #[serde(default = "default_text_length_complex")]
    pub text_length_complex: usize,
    #[serde(default = "default_tool_count_complex")]
    pub tool_count_complex: usize,
    #[serde(default = "default_system_length_medium")]
    pub system_length_medium: usize,
    #[serde(default = "default_system_length_complex")]
    pub system_length_complex: usize,
    #[serde(default = "default_max_tokens_boost")]
    pub max_tokens_boost: u32,
}

fn default_text_length_medium() -> usize { 500 }
fn default_text_length_complex() -> usize { 3000 }
fn default_tool_count_complex() -> usize { 5 }
fn default_system_length_medium() -> usize { 2000 }
fn default_system_length_complex() -> usize { 5000 }
fn default_max_tokens_boost() -> u32 { 8192 }

impl Default for AnalyzerThresholds {
    fn default() -> Self {
        Self {
            text_length_medium: default_text_length_medium(),
            text_length_complex: default_text_length_complex(),
            tool_count_complex: default_tool_count_complex(),
            system_length_medium: default_system_length_medium(),
            system_length_complex: default_system_length_complex(),
            max_tokens_boost: default_max_tokens_boost(),
        }
    }
}

const COMPLEX_KEYWORDS: &[&str] = &[
    "architect", "architecture", "design", "refactor", "refactoring",
    "security", "audit", "optimize", "performance", "migration",
];

const SIMPLE_KEYWORDS: &[&str] = &[
    "hello", "hi", "thanks", "thank you", "ok", "bye", "yes", "no",
];

fn contains_word(text: &str, word: &str) -> bool {
    for (idx, _) in text.match_indices(word) {
        let before_ok = idx == 0 || !text.as_bytes()[idx - 1].is_ascii_alphanumeric();
        let after_idx = idx + word.len();
        let after_ok =
            after_idx >= text.len() || !text.as_bytes()[after_idx].is_ascii_alphanumeric();
        if before_ok && after_ok {
            return true;
        }
    }
    false
}

/// Pure CPU heuristic classifier for query complexity.
pub struct QueryAnalyzer {
    thresholds: AnalyzerThresholds,
}

impl QueryAnalyzer {
    pub fn new(thresholds: AnalyzerThresholds) -> Self {
        Self { thresholds }
    }

    pub fn with_defaults() -> Self {
        Self::new(AnalyzerThresholds::default())
    }

    pub fn thresholds(&self) -> &AnalyzerThresholds {
        &self.thresholds
    }

    pub fn analyze(&self, request: &CompletionRequest) -> QueryComplexity {
        let score = self.score(request);
        if score <= 1 { QueryComplexity::Simple }
        else if score <= 4 { QueryComplexity::Medium }
        else { QueryComplexity::Complex }
    }

    pub fn score(&self, request: &CompletionRequest) -> i32 {
        let t = &self.thresholds;
        let mut score: i32 = 0;

        let total_text_len: usize = request.messages.iter()
            .filter(|m| m.role == MessageRole::User)
            .map(|m| m.text_content().len())
            .sum();
        if total_text_len >= t.text_length_complex { score += 2; }
        else if total_text_len >= t.text_length_medium { score += 1; }

        let turn_count = request.messages.len();
        if turn_count > 8 { score += 2; }
        else if turn_count >= 3 { score += 1; }

        let tool_count = request.tools.len();
        if tool_count > t.tool_count_complex { score += 2; }
        else if tool_count >= 1 { score += 1; }

        let system_len = request.system.as_ref().map(|s| s.len()).unwrap_or(0);
        if system_len > t.system_length_complex { score += 2; }
        else if system_len > t.system_length_medium { score += 1; }

        if let Some(last_user) = request.messages.iter().rev()
            .find(|m| m.role == MessageRole::User)
        {
            let text = last_user.text_content().to_lowercase();
            if COMPLEX_KEYWORDS.iter().any(|kw| contains_word(&text, kw)) { score += 2; }
            if SIMPLE_KEYWORDS.iter().any(|kw| contains_word(&text, kw)) { score -= 1; }
        }

        if request.max_tokens > t.max_tokens_boost { score += 1; }
        score
    }
}

// ---------------------------------------------------------------------------
// Smart Router Provider
// ---------------------------------------------------------------------------

/// Resolved routing decision: which provider and model to use.
#[derive(Clone)]
pub struct RouteDecision {
    pub provider: Option<Arc<dyn Provider>>,
    pub model: String,
}

impl std::fmt::Debug for RouteDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RouteDecision")
            .field("provider", &self.provider.as_ref().map(|p| p.id().to_string()))
            .field("model", &self.model)
            .finish()
    }
}

/// Provider decorator that overrides the model based on query complexity.
///
/// V1 mode: single inner Provider, model override only.
/// V2 mode (cross-provider): tier_providers maps tiers to different providers.
pub struct SmartRouterProvider {
    inner: Box<dyn Provider>,
    analyzer: QueryAnalyzer,
    tier_models: HashMap<QueryComplexity, String>,
    tier_providers: HashMap<QueryComplexity, Arc<dyn Provider>>,
    default_model: String,
}

impl SmartRouterProvider {
    /// V1 mode constructor.
    pub fn new(
        inner: Box<dyn Provider>,
        analyzer: QueryAnalyzer,
        tier_models: HashMap<QueryComplexity, String>,
        default_model: String,
    ) -> Self {
        Self { inner, analyzer, tier_models, tier_providers: HashMap::new(), default_model }
    }

    /// V2 cross-provider constructor.
    pub fn new_cross_provider(
        inner: Box<dyn Provider>,
        analyzer: QueryAnalyzer,
        tier_models: HashMap<QueryComplexity, String>,
        tier_providers: HashMap<QueryComplexity, Arc<dyn Provider>>,
        default_model: String,
    ) -> Self {
        Self { inner, analyzer, tier_models, tier_providers, default_model }
    }

    pub fn is_cross_provider(&self) -> bool {
        !self.tier_providers.is_empty()
    }

    fn route(&self, request: &CompletionRequest) -> RouteDecision {
        let complexity = self.analyzer.analyze(request);
        let model = self.tier_models.get(&complexity).cloned()
            .unwrap_or_else(|| self.default_model.clone());
        let provider = self.tier_providers.get(&complexity).cloned();
        debug!(%complexity, %model, cross_provider = provider.is_some(), "SmartRouter route decision");
        RouteDecision { provider, model }
    }
}

#[async_trait]
impl Provider for SmartRouterProvider {
    fn id(&self) -> &str { self.inner.id() }

    async fn complete(&self, mut request: CompletionRequest) -> Result<CompletionResponse> {
        let decision = self.route(&request);
        request.model = decision.model;
        match decision.provider {
            Some(provider) => provider.complete(request).await,
            None => self.inner.complete(request).await,
        }
    }

    async fn stream(&self, mut request: CompletionRequest) -> Result<CompletionStream> {
        let decision = self.route(&request);
        request.model = decision.model;
        match decision.provider {
            Some(provider) => provider.stream(request).await,
            None => self.inner.stream(request).await,
        }
    }
}

// ---------------------------------------------------------------------------
// Configuration types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartRoutingConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_tier")]
    pub default_tier: String,
    #[serde(default)]
    pub tiers: HashMap<String, TierConfig>,
    #[serde(default)]
    pub thresholds: Option<AnalyzerThresholds>,
}

fn default_tier() -> String { "medium".to_string() }

impl Default for SmartRoutingConfig {
    fn default() -> Self {
        Self { enabled: false, default_tier: default_tier(), tiers: HashMap::new(), thresholds: None }
    }
}

/// V1: model only. V2: model + optional provider/api_key/base_url.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierConfig {
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

fn parse_tier_name(name: &str) -> Option<QueryComplexity> {
    match name {
        "simple" => Some(QueryComplexity::Simple),
        "medium" => Some(QueryComplexity::Medium),
        "complex" => Some(QueryComplexity::Complex),
        _ => None,
    }
}

impl SmartRoutingConfig {
    pub fn has_cross_provider_tiers(&self) -> bool {
        self.tiers.values().any(|t| t.provider.is_some())
    }

    pub fn build_provider(&self, inner: Box<dyn Provider>) -> Option<Box<dyn Provider>> {
        if !self.enabled { return None; }

        let thresholds = self.thresholds.clone().unwrap_or_default();
        let analyzer = QueryAnalyzer::new(thresholds);
        let mut tier_models = HashMap::new();
        let mut tier_providers: HashMap<QueryComplexity, Arc<dyn Provider>> = HashMap::new();

        for (name, cfg) in &self.tiers {
            let Some(complexity) = parse_tier_name(name) else { continue };
            tier_models.insert(complexity, cfg.model.clone());
            if let Some(ref provider_name) = cfg.provider {
                let api_key = cfg.api_key.clone()
                    .unwrap_or_else(|| Self::resolve_provider_api_key(provider_name));
                let provider = super::create_provider(provider_name, api_key, cfg.base_url.clone());
                tier_providers.insert(complexity, Arc::from(provider));
            }
        }

        let default_model = self.tiers.get(&self.default_tier)
            .map(|c| c.model.clone())
            .unwrap_or_else(|| "claude-sonnet-4-20250514".to_string());

        if tier_providers.is_empty() {
            Some(Box::new(SmartRouterProvider::new(inner, analyzer, tier_models, default_model)))
        } else {
            Some(Box::new(SmartRouterProvider::new_cross_provider(
                inner, analyzer, tier_models, tier_providers, default_model,
            )))
        }
    }

    pub fn build_cross_provider(
        &self,
        inner: Box<dyn Provider>,
        tier_providers: HashMap<QueryComplexity, Arc<dyn Provider>>,
    ) -> Option<Box<dyn Provider>> {
        if !self.enabled { return None; }

        let thresholds = self.thresholds.clone().unwrap_or_default();
        let analyzer = QueryAnalyzer::new(thresholds);
        let mut tier_models = HashMap::new();
        for (name, cfg) in &self.tiers {
            let Some(complexity) = parse_tier_name(name) else { continue };
            tier_models.insert(complexity, cfg.model.clone());
        }
        let default_model = self.tiers.get(&self.default_tier)
            .map(|c| c.model.clone())
            .unwrap_or_else(|| "claude-sonnet-4-20250514".to_string());

        Some(Box::new(SmartRouterProvider::new_cross_provider(
            inner, analyzer, tier_models, tier_providers, default_model,
        )))
    }

    fn resolve_provider_api_key(provider_name: &str) -> String {
        match provider_name {
            "anthropic" => std::env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
            "openai" => std::env::var("OPENAI_API_KEY").unwrap_or_default(),
            _ => std::env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use octo_types::ChatMessage;

    fn make_request() -> CompletionRequest { CompletionRequest::default() }

    #[test]
    fn test_default_thresholds() {
        let t = AnalyzerThresholds::default();
        assert_eq!(t.text_length_medium, 500);
        assert_eq!(t.text_length_complex, 3000);
        assert_eq!(t.tool_count_complex, 5);
        assert_eq!(t.system_length_medium, 2000);
        assert_eq!(t.system_length_complex, 5000);
        assert_eq!(t.max_tokens_boost, 8192);
    }

    #[test]
    fn test_empty_request_is_simple() {
        let analyzer = QueryAnalyzer::with_defaults();
        assert_eq!(analyzer.analyze(&make_request()), QueryComplexity::Simple);
    }

    #[test]
    fn test_simple_hello() {
        let analyzer = QueryAnalyzer::with_defaults();
        let mut req = make_request();
        req.messages.push(ChatMessage::user("hello"));
        assert_eq!(analyzer.analyze(&req), QueryComplexity::Simple);
    }

    #[test]
    fn test_complexity_display() {
        assert_eq!(format!("{}", QueryComplexity::Simple), "simple");
        assert_eq!(format!("{}", QueryComplexity::Medium), "medium");
        assert_eq!(format!("{}", QueryComplexity::Complex), "complex");
    }

    #[test]
    fn test_tier_config_v1_compat() {
        let cfg: TierConfig = serde_yaml::from_str("model: claude-haiku").unwrap();
        assert_eq!(cfg.model, "claude-haiku");
        assert!(cfg.provider.is_none());
    }

    #[test]
    fn test_tier_config_v2_with_provider() {
        let cfg: TierConfig = serde_yaml::from_str("model: gpt-4o-mini\nprovider: openai\napi_key: sk-test\n").unwrap();
        assert_eq!(cfg.model, "gpt-4o-mini");
        assert_eq!(cfg.provider.as_deref(), Some("openai"));
    }

    #[test]
    fn test_has_cross_provider_tiers_false() {
        assert!(!SmartRoutingConfig::default().has_cross_provider_tiers());
    }

    #[test]
    fn test_parse_tier_name_valid() {
        assert_eq!(parse_tier_name("simple"), Some(QueryComplexity::Simple));
        assert_eq!(parse_tier_name("medium"), Some(QueryComplexity::Medium));
        assert_eq!(parse_tier_name("complex"), Some(QueryComplexity::Complex));
        assert_eq!(parse_tier_name("unknown"), None);
    }
}
