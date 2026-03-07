//! EmbeddingClient — converts text to embedding vectors via external APIs.
//!
//! Supports OpenAI (`text-embedding-3-small`) and Anthropic Voyage
//! (`voyage-3-lite`). Results are cached in-memory (LRU-style, max 1000).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Which embedding API to call.
#[derive(Debug, Clone)]
pub enum EmbeddingProvider {
    OpenAI,
    /// Anthropic's Voyage embedding API
    Anthropic,
}

/// Configuration for EmbeddingClient.
#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    pub provider: EmbeddingProvider,
    pub api_key: String,
    /// Model name: "text-embedding-3-small" (OpenAI) or "voyage-3-lite" (Voyage).
    pub model: String,
    /// Output dimension: 1536 (OpenAI) or 1024 (Voyage).
    pub dimensions: usize,
    /// Max texts per API call.
    pub batch_size: usize,
}

impl EmbeddingConfig {
    /// Default OpenAI config (text-embedding-3-small, 1536 dims).
    pub fn openai(api_key: impl Into<String>) -> Self {
        Self {
            provider: EmbeddingProvider::OpenAI,
            api_key: api_key.into(),
            model: "text-embedding-3-small".to_string(),
            dimensions: 1536,
            batch_size: 100,
        }
    }

    /// Default Anthropic Voyage config (voyage-3-lite, 1024 dims).
    pub fn anthropic(api_key: impl Into<String>) -> Self {
        Self {
            provider: EmbeddingProvider::Anthropic,
            api_key: api_key.into(),
            model: "voyage-3-lite".to_string(),
            dimensions: 1024,
            batch_size: 8,
        }
    }
}

// ── API request/response types ─────────────────────────────────────────────

#[derive(Serialize)]
struct OpenAiRequest<'a> {
    input: &'a [&'a str],
    model: &'a str,
}

#[derive(Deserialize)]
struct OpenAiResponse {
    data: Vec<OpenAiEmbedding>,
}

#[derive(Deserialize)]
struct OpenAiEmbedding {
    embedding: Vec<f32>,
}

#[derive(Serialize)]
struct VoyageRequest<'a> {
    input: &'a [&'a str],
    model: &'a str,
}

#[derive(Deserialize)]
struct VoyageResponse {
    data: Vec<VoyageEmbedding>,
}

#[derive(Deserialize)]
struct VoyageEmbedding {
    embedding: Vec<f32>,
}

// ── EmbeddingClient ────────────────────────────────────────────────────────

/// HTTP client for embedding APIs with in-memory LRU-style caching.
pub struct EmbeddingClient {
    config: EmbeddingConfig,
    http: reqwest::Client,
    /// text → embedding cache (max `cache_max` entries)
    cache: Arc<RwLock<HashMap<String, Vec<f32>>>>,
    cache_max: usize,
}

impl EmbeddingClient {
    pub fn new(config: EmbeddingConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("failed to build HTTP client");
        Self {
            config,
            http,
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_max: 1_000,
        }
    }

    /// Embed a single text, using cache if available.
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Cache hit
        if let Some(v) = self.cache.read().await.get(text) {
            debug!("embedding cache hit");
            return Ok(v.clone());
        }

        let result = self.call_api(&[text]).await?;
        let vec = result.into_iter().next().context("empty embedding response")?;

        // Cache store (evict oldest if at capacity — simple FIFO)
        let mut cache = self.cache.write().await;
        if cache.len() >= self.cache_max {
            if let Some(key) = cache.keys().next().cloned() {
                cache.remove(&key);
            }
        }
        cache.insert(text.to_string(), vec.clone());

        Ok(vec)
    }

    /// Embed multiple texts, batching API calls as needed.
    pub async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let mut results = Vec::with_capacity(texts.len());
        for chunk in texts.chunks(self.config.batch_size) {
            let mut batch_results = self.call_api(chunk).await?;
            results.append(&mut batch_results);
        }
        Ok(results)
    }

    async fn call_api(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        match self.config.provider {
            EmbeddingProvider::OpenAI => self.call_openai(texts).await,
            EmbeddingProvider::Anthropic => self.call_voyage(texts).await,
        }
    }

    async fn call_openai(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let resp = self
            .http
            .post("https://api.openai.com/v1/embeddings")
            .bearer_auth(&self.config.api_key)
            .json(&OpenAiRequest {
                input: texts,
                model: &self.config.model,
            })
            .send()
            .await
            .context("OpenAI embedding request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            warn!("OpenAI embedding error {}: {}", status, body);
            anyhow::bail!("OpenAI embedding API error {}: {}", status, body);
        }

        let parsed: OpenAiResponse = resp
            .json()
            .await
            .context("failed to parse OpenAI embedding response")?;
        Ok(parsed.data.into_iter().map(|e| e.embedding).collect())
    }

    async fn call_voyage(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let resp = self
            .http
            .post("https://api.voyageai.com/v1/embeddings")
            .bearer_auth(&self.config.api_key)
            .json(&VoyageRequest {
                input: texts,
                model: &self.config.model,
            })
            .send()
            .await
            .context("Voyage embedding request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            warn!("Voyage embedding error {}: {}", status, body);
            anyhow::bail!("Voyage embedding API error {}: {}", status, body);
        }

        let parsed: VoyageResponse = resp
            .json()
            .await
            .context("failed to parse Voyage embedding response")?;
        Ok(parsed.data.into_iter().map(|e| e.embedding).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_config_defaults() {
        let cfg = EmbeddingConfig::openai("key");
        assert_eq!(cfg.dimensions, 1536);
        assert_eq!(cfg.batch_size, 100);
        assert_eq!(cfg.model, "text-embedding-3-small");
    }

    #[test]
    fn test_anthropic_config_defaults() {
        let cfg = EmbeddingConfig::anthropic("key");
        assert_eq!(cfg.dimensions, 1024);
        assert_eq!(cfg.batch_size, 8);
        assert_eq!(cfg.model, "voyage-3-lite");
    }

    #[tokio::test]
    async fn test_cache_hit_does_not_panic() {
        let client = EmbeddingClient::new(EmbeddingConfig::openai("fake"));
        // Manually seed cache
        client
            .cache
            .write()
            .await
            .insert("hello".to_string(), vec![0.1, 0.2, 0.3]);
        let result = client.embed("hello").await.unwrap();
        assert_eq!(result, vec![0.1f32, 0.2, 0.3]);
    }
}
