//! Metering module for tracking token usage and request metrics.
//!
//! This module provides atomic counters for tracking LLM usage including
//! input/output tokens, request counts, errors, and duration.
//!
//! Sub-modules:
//! - [`pricing`] -- Model pricing table for cost estimation.
//! - [`storage`] -- SQLite-backed persistent metering records.

pub mod cost_tracker;
pub mod pricing;
pub mod storage;

use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc;

use crate::metering::storage::MeteringRecord;

/// A snapshot of metering data at a point in time.
#[derive(Debug, Clone, Default)]
pub struct MeteringSnapshot {
    /// Total input tokens used.
    pub input_tokens: u64,
    /// Total output tokens generated.
    pub output_tokens: u64,
    /// Total number of requests made.
    pub requests: u64,
    /// Total number of errors encountered.
    pub errors: u64,
    /// Total duration of all requests in milliseconds.
    pub duration_ms: u64,
}

impl MeteringSnapshot {
    /// Calculate total tokens (input + output).
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }

    /// Calculate average tokens per request.
    pub fn avg_tokens_per_request(&self) -> f64 {
        if self.requests == 0 {
            return 0.0;
        }
        self.total_tokens() as f64 / self.requests as f64
    }

    /// Calculate average duration per request in milliseconds.
    pub fn avg_duration_ms(&self) -> f64 {
        if self.requests == 0 {
            return 0.0;
        }
        self.duration_ms as f64 / self.requests as f64
    }
}

/// Metering struct for tracking LLM usage with atomic counters.
///
/// Optionally holds a channel sender (`persist_tx`) to forward records
/// to a background persistence task (see [`storage::MeteringStorage`]).
pub struct Metering {
    /// Total input tokens used.
    pub input_tokens: AtomicU64,
    /// Total output tokens generated.
    pub output_tokens: AtomicU64,
    /// Total number of requests made.
    pub requests: AtomicU64,
    /// Total number of errors encountered.
    pub errors: AtomicU64,
    /// Total duration of all requests in milliseconds.
    pub duration_ms: AtomicU64,
    /// Optional channel for persisting individual metering records.
    persist_tx: Option<mpsc::Sender<MeteringRecord>>,
}

impl Default for Metering {
    fn default() -> Self {
        Self::new()
    }
}

impl Metering {
    /// Create a new Metering instance without persistence.
    pub fn new() -> Self {
        Self {
            input_tokens: AtomicU64::new(0),
            output_tokens: AtomicU64::new(0),
            requests: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            duration_ms: AtomicU64::new(0),
            persist_tx: None,
        }
    }

    /// Create a new Metering instance with a persistence channel.
    pub fn with_persist_tx(persist_tx: mpsc::Sender<MeteringRecord>) -> Self {
        Self {
            input_tokens: AtomicU64::new(0),
            output_tokens: AtomicU64::new(0),
            requests: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            duration_ms: AtomicU64::new(0),
            persist_tx: Some(persist_tx),
        }
    }

    /// Record a successful request with input/output tokens and duration.
    pub fn record_request(&self, input: usize, output: usize, duration_ms: u64) {
        self.input_tokens.fetch_add(input as u64, Ordering::Relaxed);
        self.output_tokens
            .fetch_add(output as u64, Ordering::Relaxed);
        self.requests.fetch_add(1, Ordering::Relaxed);
        self.duration_ms.fetch_add(duration_ms, Ordering::Relaxed);
    }

    /// Extended record_request that also captures model and session_id for persistence.
    /// Updates atomic counters via record_request() and sends a MeteringRecord
    /// through the persist channel if configured. Uses try_send() to avoid blocking.
    pub fn record_request_ext(
        &self,
        input: usize,
        output: usize,
        duration_ms: u64,
        model: &str,
        session_id: &str,
    ) {
        self.record_request(input, output, duration_ms);
        if let Some(ref tx) = self.persist_tx {
            let record = MeteringRecord {
                session_id: session_id.to_string(),
                model: model.to_string(),
                input_tokens: input as u64,
                output_tokens: output as u64,
                duration_ms,
                is_error: false,
            };
            let _ = tx.try_send(record);
        }
    }

    /// Record an error (increments error counter only).
    pub fn record_error(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Take a snapshot of current metering values.
    pub fn snapshot(&self) -> MeteringSnapshot {
        MeteringSnapshot {
            input_tokens: self.input_tokens.load(Ordering::Relaxed),
            output_tokens: self.output_tokens.load(Ordering::Relaxed),
            requests: self.requests.load(Ordering::Relaxed),
            errors: self.errors.load(Ordering::Relaxed),
            duration_ms: self.duration_ms.load(Ordering::Relaxed),
        }
    }

    /// Reset all counters to zero.
    pub fn reset(&self) {
        self.input_tokens.store(0, Ordering::Relaxed);
        self.output_tokens.store(0, Ordering::Relaxed);
        self.requests.store(0, Ordering::Relaxed);
        self.errors.store(0, Ordering::Relaxed);
        self.duration_ms.store(0, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metering_record_request() {
        let metering = Metering::new();
        metering.record_request(100, 50, 1000);
        metering.record_request(200, 75, 500);
        let snapshot = metering.snapshot();
        assert_eq!(snapshot.input_tokens, 300);
        assert_eq!(snapshot.output_tokens, 125);
        assert_eq!(snapshot.requests, 2);
        assert_eq!(snapshot.duration_ms, 1500);
        assert_eq!(snapshot.errors, 0);
    }

    #[test]
    fn test_metering_record_error() {
        let metering = Metering::new();
        metering.record_request(100, 50, 1000);
        metering.record_error();
        let snapshot = metering.snapshot();
        assert_eq!(snapshot.requests, 1);
        assert_eq!(snapshot.errors, 1);
    }

    #[test]
    fn test_metering_snapshot_calculations() {
        let metering = Metering::new();
        metering.record_request(100, 50, 1000);
        metering.record_request(200, 100, 2000);
        metering.record_request(300, 150, 3000);
        let snapshot = metering.snapshot();
        assert_eq!(snapshot.total_tokens(), 900);
        assert_eq!(snapshot.avg_tokens_per_request(), 300.0);
        assert_eq!(snapshot.avg_duration_ms(), 2000.0);
    }

    #[test]
    fn test_metering_reset() {
        let metering = Metering::new();
        metering.record_request(100, 50, 1000);
        metering.record_error();
        metering.reset();
        let snapshot = metering.snapshot();
        assert_eq!(snapshot.input_tokens, 0);
        assert_eq!(snapshot.output_tokens, 0);
        assert_eq!(snapshot.requests, 0);
        assert_eq!(snapshot.errors, 0);
        assert_eq!(snapshot.duration_ms, 0);
    }

    #[test]
    fn test_default_metering() {
        let metering = Metering::default();
        let snapshot = metering.snapshot();
        assert_eq!(snapshot.input_tokens, 0);
        assert_eq!(snapshot.output_tokens, 0);
        assert_eq!(snapshot.requests, 0);
        assert_eq!(snapshot.errors, 0);
        assert_eq!(snapshot.duration_ms, 0);
    }

    #[tokio::test]
    async fn test_record_request_ext_basic() {
        let metering = Metering::new();
        metering.record_request_ext(500, 200, 1500, "claude-sonnet-4", "session-1");
        let snapshot = metering.snapshot();
        assert_eq!(snapshot.input_tokens, 500);
        assert_eq!(snapshot.output_tokens, 200);
        assert_eq!(snapshot.requests, 1);
        assert_eq!(snapshot.duration_ms, 1500);
    }

    #[tokio::test]
    async fn test_persist_channel_receive() {
        let (tx, mut rx) = mpsc::channel::<MeteringRecord>(16);
        let metering = Metering::with_persist_tx(tx);
        metering.record_request_ext(1000, 400, 2000, "gpt-4o", "sess-abc");
        let snapshot = metering.snapshot();
        assert_eq!(snapshot.input_tokens, 1000);
        let record = rx.try_recv().expect("should receive a MeteringRecord");
        assert_eq!(record.model, "gpt-4o");
        assert_eq!(record.session_id, "sess-abc");
        assert_eq!(record.input_tokens, 1000);
        assert_eq!(record.output_tokens, 400);
        assert_eq!(record.duration_ms, 2000);
        assert!(!record.is_error);
    }

    #[tokio::test]
    async fn test_no_persist_when_disabled() {
        let metering = Metering::new();
        assert!(metering.persist_tx.is_none());
        metering.record_request_ext(100, 50, 500, "test-model", "test-session");
        let snapshot = metering.snapshot();
        assert_eq!(snapshot.requests, 1);
    }
}
