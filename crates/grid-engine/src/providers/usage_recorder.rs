//! Usage recorder decorator — tracks token usage statistics per provider call.
//!
//! Records input/output tokens, request counts, and per-model breakdowns.
//! Stats are stored in an `Arc<RwLock<UsageStats>>` for external querying.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::RwLock;

use grid_types::{CompletionRequest, CompletionResponse};

use super::traits::{CompletionStream, Provider};

/// Accumulated token usage statistics.
#[derive(Debug, Clone, Default)]
pub struct UsageStats {
    /// Total input tokens across all requests.
    pub total_input_tokens: u64,
    /// Total output tokens across all requests.
    pub total_output_tokens: u64,
    /// Total number of completed requests.
    pub request_count: u64,
    /// Per-model breakdown: model name → (input_tokens, output_tokens, count).
    pub by_model: HashMap<String, ModelUsage>,
}

/// Per-model usage breakdown.
#[derive(Debug, Clone, Default)]
pub struct ModelUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub request_count: u64,
}

impl UsageStats {
    /// Total tokens (input + output).
    pub fn total_tokens(&self) -> u64 {
        self.total_input_tokens + self.total_output_tokens
    }

    /// Get usage for a specific model, if any.
    pub fn for_model(&self, model: &str) -> Option<&ModelUsage> {
        self.by_model.get(model)
    }
}

/// Provider decorator that records token usage after each call.
pub struct UsageRecorderProvider {
    inner: Box<dyn Provider>,
    stats: Arc<RwLock<UsageStats>>,
}

impl UsageRecorderProvider {
    /// Create a new usage recorder wrapping the given provider.
    pub fn new(inner: Box<dyn Provider>) -> Self {
        Self {
            inner,
            stats: Arc::new(RwLock::new(UsageStats::default())),
        }
    }

    /// Create with a shared stats handle (for external access).
    pub fn with_shared_stats(inner: Box<dyn Provider>, stats: Arc<RwLock<UsageStats>>) -> Self {
        Self { inner, stats }
    }

    /// Get a clone of the stats handle for external querying.
    pub fn stats_handle(&self) -> Arc<RwLock<UsageStats>> {
        Arc::clone(&self.stats)
    }

    /// Record usage from a successful completion response.
    async fn record(&self, model: &str, response: &CompletionResponse) {
        let input = response.usage.input_tokens as u64;
        let output = response.usage.output_tokens as u64;

        let mut stats = self.stats.write().await;
        stats.total_input_tokens += input;
        stats.total_output_tokens += output;
        stats.request_count += 1;

        let entry = stats.by_model.entry(model.to_string()).or_default();
        entry.input_tokens += input;
        entry.output_tokens += output;
        entry.request_count += 1;
    }
}

#[async_trait]
impl Provider for UsageRecorderProvider {
    fn id(&self) -> &str {
        self.inner.id()
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let model = request.model.clone();
        let response = self.inner.complete(request).await?;
        self.record(&model, &response).await;
        Ok(response)
    }

    async fn stream(&self, request: CompletionRequest) -> Result<CompletionStream> {
        // Streaming — usage is not easily captured; pass through.
        self.inner.stream(request).await
    }

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        self.inner.embed(texts).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use grid_types::message::{ChatMessage, ContentBlock};
    use grid_types::provider::TokenUsage;
    use grid_types::StreamEvent;

    struct MockProvider;

    #[async_trait]
    impl Provider for MockProvider {
        fn id(&self) -> &str {
            "mock"
        }

        async fn complete(&self, _request: CompletionRequest) -> Result<CompletionResponse> {
            Ok(CompletionResponse {
                id: "r1".into(),
                content: vec![ContentBlock::Text {
                    text: "ok".into(),
                }],
                stop_reason: None,
                usage: TokenUsage {
                    input_tokens: 100,
                    output_tokens: 50,
                },
            })
        }

        async fn stream(
            &self,
            _request: CompletionRequest,
        ) -> Result<futures_util::stream::BoxStream<'static, Result<StreamEvent>>> {
            Err(anyhow::anyhow!("Not implemented"))
        }
    }

    #[tokio::test]
    async fn test_records_usage() {
        let provider = UsageRecorderProvider::new(Box::new(MockProvider));
        let handle = provider.stats_handle();

        let req = CompletionRequest {
            model: "gpt-4".into(),
            messages: vec![ChatMessage::user("hi")],
            ..Default::default()
        };

        let _ = provider.complete(req).await.unwrap();

        let stats = handle.read().await;
        assert_eq!(stats.total_input_tokens, 100);
        assert_eq!(stats.total_output_tokens, 50);
        assert_eq!(stats.request_count, 1);
        assert_eq!(stats.total_tokens(), 150);

        let model = stats.for_model("gpt-4").unwrap();
        assert_eq!(model.input_tokens, 100);
        assert_eq!(model.request_count, 1);
    }

    #[tokio::test]
    async fn test_multiple_models() {
        let provider = UsageRecorderProvider::new(Box::new(MockProvider));
        let handle = provider.stats_handle();

        for model in &["gpt-4", "gpt-4", "claude-3"] {
            let req = CompletionRequest {
                model: model.to_string(),
                messages: vec![ChatMessage::user("test")],
                ..Default::default()
            };
            let _ = provider.complete(req).await.unwrap();
        }

        let stats = handle.read().await;
        assert_eq!(stats.request_count, 3);
        assert_eq!(stats.for_model("gpt-4").unwrap().request_count, 2);
        assert_eq!(stats.for_model("claude-3").unwrap().request_count, 1);
    }
}
