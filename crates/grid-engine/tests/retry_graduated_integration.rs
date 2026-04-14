//! Integration tests for S1.T7 — graduated retry + FailoverReason routing.
//!
//! These tests wrap a mock provider in `RetryProvider::new(…, RetryPolicy::graduated())`
//! and verify end-to-end that:
//!   * transient 429 rate-limit recovers after a couple of retries,
//!   * permanent auth errors (`invalid_api_key`) short-circuit without retrying,
//!   * repeated 529 "overloaded" exhausts `max_retries + 1` attempts,
//!   * context-length errors propagate without retrying (compression signal).

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use grid_types::message::ContentBlock;
use grid_types::provider::TokenUsage;
use grid_types::{CompletionRequest, CompletionResponse};

use grid_engine::providers::pipeline::RetryProvider;
use grid_engine::providers::retry::ProviderError;
use grid_engine::providers::traits::{CompletionStream, Provider};
use grid_engine::providers::RetryPolicy;

// ---------------------------------------------------------------------------
// MockProvider — returns a queue of pre-built ProviderErrors, then a success.
// ---------------------------------------------------------------------------

/// Build a `ProviderError` with the given status and body.
fn err(status: u16, body: &str) -> ProviderError {
    ProviderError::from_http_response("mock", status, None, body.to_string())
}

/// Configurable mock provider. Each call pops one entry from `outcomes`:
///   * `Some(ProviderError)` → returned as `Err(anyhow::Error)`.
///   * `None`                → returns a canned `Ok(CompletionResponse)`.
/// When the queue is empty, the mock returns `Ok` (sentinel to catch over-call).
struct MockProvider {
    id: String,
    outcomes: Mutex<Vec<Option<ProviderError>>>,
    call_count: AtomicU32,
}

impl MockProvider {
    fn new(outcomes: Vec<Option<ProviderError>>) -> Self {
        // We pop from the end for O(1), so reverse once up front so call N
        // gets outcomes[N].
        let mut rev = outcomes;
        rev.reverse();
        Self {
            id: "mock".to_string(),
            outcomes: Mutex::new(rev),
            call_count: AtomicU32::new(0),
        }
    }

    fn calls(&self) -> u32 {
        self.call_count.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl Provider for MockProvider {
    fn id(&self) -> &str {
        &self.id
    }

    async fn complete(&self, _request: CompletionRequest) -> Result<CompletionResponse> {
        let n = self.call_count.fetch_add(1, Ordering::SeqCst);
        let next = {
            let mut q = self.outcomes.lock().unwrap();
            q.pop()
        };
        match next {
            Some(Some(pe)) => Err(pe.into()),
            Some(None) | None => Ok(CompletionResponse {
                id: format!("resp-{}", n),
                content: vec![ContentBlock::Text { text: "ok".into() }],
                stop_reason: None,
                usage: TokenUsage {
                    input_tokens: 0,
                    output_tokens: 0,
                },
            }),
        }
    }

    async fn stream(&self, _request: CompletionRequest) -> Result<CompletionStream> {
        Err(anyhow::anyhow!("stream not implemented in mock"))
    }
}

/// Graduated policy tuned for fast unit tests: keep `max_retries=3` and jitter
/// behaviour, but collapse the delay so the tests run in milliseconds.
fn fast_graduated() -> RetryPolicy {
    RetryPolicy {
        base_delay: Duration::from_millis(1),
        max_delay: Duration::from_millis(5),
        ..RetryPolicy::graduated()
    }
}

fn default_request() -> CompletionRequest {
    CompletionRequest::default()
}

// ---------------------------------------------------------------------------
// 1. Rate-limit (429) recovers after two retries.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn rate_limit_429_recovers_after_two_retries() {
    // Call 0 → 429, call 1 → 429, call 2 → Ok.
    let mock = MockProvider::new(vec![
        Some(err(429, r#"{"error":{"type":"rate_limit_error"}}"#)),
        Some(err(429, r#"{"error":{"type":"rate_limit_error"}}"#)),
        None,
    ]);
    // Keep the Arc so we can read call count after the provider is moved.
    let mock = std::sync::Arc::new(mock);
    let provider = RetryProvider::new(
        Box::new(ArcProvider(std::sync::Arc::clone(&mock))),
        fast_graduated(),
    );

    let resp = provider.complete(default_request()).await;
    assert!(
        resp.is_ok(),
        "graduated retry must recover from 2×429: {:?}",
        resp.err()
    );
    assert_eq!(
        mock.calls(),
        3,
        "expected exactly 3 calls (2 failures + 1 success)"
    );
}

// ---------------------------------------------------------------------------
// 2. Permanent auth error → no retry.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn auth_permanent_fails_immediately() {
    // 401 with invalid_api_key → FailoverReason::AuthPermanent → retryable=false.
    let mock = MockProvider::new(vec![Some(err(
        401,
        r#"{"error":{"code":"invalid_api_key"}}"#,
    ))]);
    let mock = std::sync::Arc::new(mock);
    let provider = RetryProvider::new(
        Box::new(ArcProvider(std::sync::Arc::clone(&mock))),
        fast_graduated(),
    );

    let resp = provider.complete(default_request()).await;
    assert!(resp.is_err(), "AuthPermanent must propagate as Err");
    assert_eq!(
        mock.calls(),
        1,
        "AuthPermanent must short-circuit after the first call, got {} calls",
        mock.calls()
    );
}

// ---------------------------------------------------------------------------
// 3. Persistent 529 Overloaded → retry exhausts after max_retries + 1 calls.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn overloaded_529_retries_and_exhausts() {
    // Always 529. With max_retries=3 graduated, we expect 1 initial + 3 retries = 4 calls.
    let outcomes = (0..10)
        .map(|_| Some(err(529, r#"{"error":{"type":"overloaded"}}"#)))
        .collect();
    let mock = MockProvider::new(outcomes);
    let mock = std::sync::Arc::new(mock);
    let provider = RetryProvider::new(
        Box::new(ArcProvider(std::sync::Arc::clone(&mock))),
        fast_graduated(),
    );

    let resp = provider.complete(default_request()).await;
    assert!(resp.is_err(), "persistent 529 must ultimately fail");
    assert_eq!(
        mock.calls(),
        4,
        "graduated policy must try exactly max_retries+1 = 4 times"
    );
}

// ---------------------------------------------------------------------------
// 4. Context overflow propagates without retry (compression signal).
// ---------------------------------------------------------------------------

#[tokio::test]
async fn context_overflow_propagates_without_retry() {
    // 400 body with `context_length_exceeded` → FailoverReason::ContextOverflow
    // → should_compress=true → propagate without retrying.
    let mock = MockProvider::new(vec![Some(err(
        400,
        r#"{"error":{"type":"context_length_exceeded"}}"#,
    ))]);
    let mock = std::sync::Arc::new(mock);
    let provider = RetryProvider::new(
        Box::new(ArcProvider(std::sync::Arc::clone(&mock))),
        fast_graduated(),
    );

    let resp = provider.complete(default_request()).await;
    assert!(resp.is_err(), "ContextOverflow must propagate as Err");
    assert_eq!(
        mock.calls(),
        1,
        "ContextOverflow compress-signal must short-circuit, got {} calls",
        mock.calls()
    );
}

// ---------------------------------------------------------------------------
// Helper: ArcProvider — thin wrapper so we can share MockProvider via Arc
// and still inspect its call_count after giving the RetryProvider ownership.
// ---------------------------------------------------------------------------

struct ArcProvider(std::sync::Arc<MockProvider>);

#[async_trait]
impl Provider for ArcProvider {
    fn id(&self) -> &str {
        self.0.id()
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        self.0.complete(request).await
    }

    async fn stream(&self, request: CompletionRequest) -> Result<CompletionStream> {
        self.0.stream(request).await
    }
}
