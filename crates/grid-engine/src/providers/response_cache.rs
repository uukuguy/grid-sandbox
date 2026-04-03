//! Response cache decorator — caches identical non-streaming requests.
//!
//! Uses an LRU cache keyed by a SHA-256 hash of (model + messages + tools).
//! Only `complete()` calls are cached; `stream()` always passes through.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::Result;
use async_trait::async_trait;
use lru::LruCache;
use sha2::{Digest, Sha256};
use std::num::NonZeroUsize;

use grid_types::{CompletionRequest, CompletionResponse};

use super::traits::{CompletionStream, Provider};

/// Cached entry with expiry timestamp.
struct CacheEntry {
    response: CompletionResponse,
    inserted_at: Instant,
}

/// Provider decorator that caches non-streaming completion responses.
pub struct ResponseCacheProvider {
    inner: Box<dyn Provider>,
    cache: Mutex<LruCache<String, CacheEntry>>,
    ttl: Duration,
}

impl ResponseCacheProvider {
    /// Create a new cache decorator with given capacity and TTL.
    pub fn new(inner: Box<dyn Provider>, capacity: usize, ttl: Duration) -> Self {
        Self {
            inner,
            cache: Mutex::new(LruCache::new(
                NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(128).unwrap()),
            )),
            ttl,
        }
    }

    /// Create with default settings (capacity=128, ttl=300s).
    pub fn with_defaults(inner: Box<dyn Provider>) -> Self {
        Self::new(inner, 128, Duration::from_secs(300))
    }

    /// Compute a cache key from the request by hashing model + serialized messages + tools.
    fn cache_key(request: &CompletionRequest) -> String {
        let mut hasher = Sha256::new();
        hasher.update(request.model.as_bytes());
        if let Some(ref system) = request.system {
            hasher.update(system.as_bytes());
        }
        if let Ok(msgs) = serde_json::to_string(&request.messages) {
            hasher.update(msgs.as_bytes());
        }
        if let Ok(tools) = serde_json::to_string(&request.tools) {
            hasher.update(tools.as_bytes());
        }
        hasher.update(request.max_tokens.to_le_bytes());
        if let Some(temp) = request.temperature {
            hasher.update(temp.to_le_bytes());
        }
        hex::encode(hasher.finalize())
    }

    /// Get a cached response if it exists and hasn't expired.
    fn get_cached(&self, key: &str) -> Option<CompletionResponse> {
        let mut cache = self.cache.lock().ok()?;
        if let Some(entry) = cache.get(key) {
            if entry.inserted_at.elapsed() < self.ttl {
                return Some(entry.response.clone());
            }
            // Expired — remove it.
            cache.pop(key);
        }
        None
    }

    /// Insert a response into the cache.
    fn put_cached(&self, key: String, response: &CompletionResponse) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.put(
                key,
                CacheEntry {
                    response: response.clone(),
                    inserted_at: Instant::now(),
                },
            );
        }
    }

    /// Returns the current number of entries in the cache.
    pub fn cache_len(&self) -> usize {
        self.cache.lock().map(|c| c.len()).unwrap_or(0)
    }
}

#[async_trait]
impl Provider for ResponseCacheProvider {
    fn id(&self) -> &str {
        self.inner.id()
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let key = Self::cache_key(&request);

        if let Some(cached) = self.get_cached(&key) {
            tracing::debug!("ResponseCache hit for provider '{}'", self.id());
            return Ok(cached);
        }

        let response = self.inner.complete(request).await?;
        self.put_cached(key, &response);
        Ok(response)
    }

    async fn stream(&self, request: CompletionRequest) -> Result<CompletionStream> {
        // Streaming responses are not cached — pass through.
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
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    struct CountingProvider {
        call_count: Arc<AtomicU32>,
    }

    #[async_trait]
    impl Provider for CountingProvider {
        fn id(&self) -> &str {
            "counting"
        }

        async fn complete(&self, _request: CompletionRequest) -> Result<CompletionResponse> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            Ok(CompletionResponse {
                id: "resp-1".into(),
                content: vec![ContentBlock::Text {
                    text: "cached".into(),
                }],
                stop_reason: None,
                usage: TokenUsage {
                    input_tokens: 10,
                    output_tokens: 5,
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
    async fn test_cache_hit() {
        let count = Arc::new(AtomicU32::new(0));
        let provider = ResponseCacheProvider::with_defaults(Box::new(CountingProvider {
            call_count: Arc::clone(&count),
        }));

        let req = CompletionRequest {
            model: "test".into(),
            messages: vec![ChatMessage::user("hello")],
            ..Default::default()
        };

        // First call: miss
        let _ = provider.complete(req.clone()).await.unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 1);

        // Second call: hit
        let _ = provider.complete(req.clone()).await.unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 1); // still 1
        assert_eq!(provider.cache_len(), 1);
    }

    #[tokio::test]
    async fn test_cache_miss_different_request() {
        let count = Arc::new(AtomicU32::new(0));
        let provider = ResponseCacheProvider::with_defaults(Box::new(CountingProvider {
            call_count: Arc::clone(&count),
        }));

        let req1 = CompletionRequest {
            model: "test".into(),
            messages: vec![ChatMessage::user("hello")],
            ..Default::default()
        };
        let req2 = CompletionRequest {
            model: "test".into(),
            messages: vec![ChatMessage::user("world")],
            ..Default::default()
        };

        let _ = provider.complete(req1).await.unwrap();
        let _ = provider.complete(req2).await.unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_cache_ttl_expiry() {
        let count = Arc::new(AtomicU32::new(0));
        let provider = ResponseCacheProvider::new(
            Box::new(CountingProvider {
                call_count: Arc::clone(&count),
            }),
            128,
            Duration::from_millis(1), // 1ms TTL
        );

        let req = CompletionRequest {
            model: "test".into(),
            messages: vec![ChatMessage::user("hello")],
            ..Default::default()
        };

        let _ = provider.complete(req.clone()).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        let _ = provider.complete(req).await.unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 2); // expired, called twice
    }
}
