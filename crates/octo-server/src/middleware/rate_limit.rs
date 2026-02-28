//! Rate limiting middleware for Octo Server

use std::{
    collections::HashMap,
    sync::Arc,
    time::Instant,
};

use axum::{
    body::Body,
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use tokio::sync::RwLock;

/// Rate limiter for request throttling
///
/// Uses a sliding window algorithm to track requests per client IP
#[derive(Clone)]
pub struct RateLimiter {
    inner: Arc<RateLimiterInner>,
}

struct RateLimiterInner {
    requests: RwLock<HashMap<String, Vec<Instant>>>,
    max_requests: usize,
    window_secs: u64,
}

impl RateLimiter {
    /// Create a new rate limiter
    ///
    /// # Arguments
    /// * `max_requests` - Maximum number of requests allowed in the window
    /// * `window_secs` - Time window in seconds
    pub fn new(max_requests: usize, window_secs: u64) -> Self {
        Self {
            inner: Arc::new(RateLimiterInner {
                requests: RwLock::new(HashMap::new()),
                max_requests,
                window_secs,
            }),
        }
    }

    /// Check if a request is allowed and record it
    ///
    /// # Arguments
    /// * `key` - Unique identifier for the client (e.g., IP address)
    ///
    /// # Returns
    /// * `true` - Request is allowed
    /// * `false` - Request is rate limited
    pub async fn check(&self, key: &str) -> bool {
        let mut requests = self.inner.requests.write().await;
        let now = Instant::now();

        let timestamps = requests.entry(key.to_string()).or_insert_with(Vec::new);

        // Remove expired timestamps
        timestamps.retain(|t| now.duration_since(*t).as_secs() < self.inner.window_secs);

        // Check if limit exceeded
        if timestamps.len() >= self.inner.max_requests {
            return false;
        }

        // Record this request
        timestamps.push(now);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_allows_requests_within_limit() {
        let limiter = RateLimiter::new(5, 60);

        // First 5 requests should be allowed
        for _ in 0..5 {
            assert!(limiter.check("test_key").await);
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_blocks_requests_over_limit() {
        let limiter = RateLimiter::new(3, 60);

        // First 3 requests should be allowed
        for _ in 0..3 {
            assert!(limiter.check("test_key").await);
        }

        // 4th request should be blocked
        assert!(!limiter.check("test_key").await);
    }

    #[tokio::test]
    async fn test_rate_limiter_separate_keys() {
        let limiter = RateLimiter::new(2, 60);

        // Different keys should have separate limits
        assert!(limiter.check("key1").await);
        assert!(limiter.check("key1").await);
        assert!(!limiter.check("key1").await);

        // key2 should still have capacity
        assert!(limiter.check("key2").await);
        assert!(limiter.check("key2").await);
        assert!(!limiter.check("key2").await);
    }
}
