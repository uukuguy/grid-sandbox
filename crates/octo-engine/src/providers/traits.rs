use std::pin::Pin;

use anyhow::Result;
use async_trait::async_trait;
use futures_util::Stream;
use octo_types::{CompletionRequest, CompletionResponse, StreamEvent};

pub type CompletionStream = Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>>;

#[async_trait]
pub trait Provider: Send + Sync {
    fn id(&self) -> &str;
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;
    async fn stream(&self, request: CompletionRequest) -> Result<CompletionStream>;

    /// Generate embeddings for the given texts.
    /// Default implementation returns an error (not all providers support embeddings).
    async fn embed(&self, _texts: &[String]) -> Result<Vec<Vec<f32>>> {
        Err(anyhow::anyhow!(
            "Provider '{}' does not support embeddings",
            self.id()
        ))
    }
}
