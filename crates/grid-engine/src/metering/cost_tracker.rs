//! Per-session / per-model cumulative cost tracking with cache token granularity.
//!
//! Builds on top of [`super::pricing::ModelPricing`] which provides model lookup
//! and base cost estimation. This module adds cumulative tracking, cache read/write
//! token accounting, and summary aggregation.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::pricing::ModelPricing;

/// Per-model cost breakdown.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct ModelCostEntry {
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub total_usd: f64,
    pub requests: u64,
}

/// Summary of all costs across models.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct CostSummary {
    pub per_model: Vec<ModelCostEntry>,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cache_read_tokens: u64,
    pub total_cache_write_tokens: u64,
    pub total_usd: f64,
    pub total_requests: u64,
}

/// Tracks cumulative LLM costs per model with cache token granularity.
///
/// Thread-safe via interior Mutex. Designed to be stored in `AgentLoopConfig`
/// and called after each LLM response in the harness.
#[derive(Debug, Clone)]
pub struct CostTracker {
    inner: Arc<Mutex<CostTrackerInner>>,
}

#[derive(Debug, Default)]
struct CostTrackerInner {
    entries: HashMap<String, ModelCostEntry>,
}

impl Default for CostTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl CostTracker {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(CostTrackerInner::default())),
        }
    }

    /// Record a single LLM call's token usage and compute cost.
    ///
    /// Uses `ModelPricing::lookup()` from the existing pricing module.
    /// Cache tokens use reduced rates: cache_read = 10% of input price,
    /// cache_write = 25% of input price (Anthropic convention).
    pub fn record(
        &self,
        model: &str,
        input_tokens: u64,
        output_tokens: u64,
        cache_read_tokens: u64,
        cache_write_tokens: u64,
    ) -> f64 {
        let pricing = ModelPricing::lookup(model);

        // Standard input/output cost
        let base_cost = pricing.estimate_cost(input_tokens, output_tokens);

        // Cache costs: read = 10% of input price, write = 25% of input price
        let cache_read_cost =
            (cache_read_tokens as f64 / 1_000_000.0) * pricing.input_per_million * 0.1;
        let cache_write_cost =
            (cache_write_tokens as f64 / 1_000_000.0) * pricing.input_per_million * 0.25;

        let total_cost = base_cost + cache_read_cost + cache_write_cost;

        let mut inner = self.inner.lock().unwrap();
        let entry = inner
            .entries
            .entry(model.to_string())
            .or_insert_with(|| ModelCostEntry {
                model: model.to_string(),
                ..Default::default()
            });
        entry.input_tokens += input_tokens;
        entry.output_tokens += output_tokens;
        entry.cache_read_tokens += cache_read_tokens;
        entry.cache_write_tokens += cache_write_tokens;
        entry.total_usd += total_cost;
        entry.requests += 1;

        total_cost
    }

    /// Get a summary of all accumulated costs.
    pub fn summary(&self) -> CostSummary {
        let inner = self.inner.lock().unwrap();
        let mut summary = CostSummary::default();
        for entry in inner.entries.values() {
            summary.total_input_tokens += entry.input_tokens;
            summary.total_output_tokens += entry.output_tokens;
            summary.total_cache_read_tokens += entry.cache_read_tokens;
            summary.total_cache_write_tokens += entry.cache_write_tokens;
            summary.total_usd += entry.total_usd;
            summary.total_requests += entry.requests;
            summary.per_model.push(entry.clone());
        }
        // Sort by cost descending for readability
        summary
            .per_model
            .sort_by(|a, b| b.total_usd.partial_cmp(&a.total_usd).unwrap_or(std::cmp::Ordering::Equal));
        summary
    }

    /// Get the total cost in USD.
    pub fn total_usd(&self) -> f64 {
        let inner = self.inner.lock().unwrap();
        inner.entries.values().map(|e| e.total_usd).sum()
    }

    /// Reset all tracked costs.
    pub fn reset(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_tracker_new() {
        let tracker = CostTracker::new();
        let summary = tracker.summary();
        assert_eq!(summary.total_requests, 0);
        assert!((summary.total_usd - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_record_single_model() {
        let tracker = CostTracker::new();
        let cost = tracker.record("claude-sonnet-4-20250514", 1_000_000, 100_000, 0, 0);
        // sonnet: $3/M input + $15/M output = $3 + $1.5 = $4.5
        assert!((cost - 4.5).abs() < 0.01);
        let summary = tracker.summary();
        assert_eq!(summary.total_requests, 1);
        assert_eq!(summary.total_input_tokens, 1_000_000);
        assert_eq!(summary.total_output_tokens, 100_000);
    }

    #[test]
    fn test_record_with_cache_tokens() {
        let tracker = CostTracker::new();
        // 100K input, 10K output, 500K cache_read, 200K cache_write
        let cost = tracker.record("claude-sonnet-4-20250514", 100_000, 10_000, 500_000, 200_000);
        // base: (100K/1M)*3 + (10K/1M)*15 = 0.3 + 0.15 = 0.45
        // cache_read: (500K/1M)*3*0.1 = 0.15
        // cache_write: (200K/1M)*3*0.25 = 0.15
        // total: 0.45 + 0.15 + 0.15 = 0.75
        assert!((cost - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_multiple_models() {
        let tracker = CostTracker::new();
        tracker.record("claude-sonnet-4-20250514", 1000, 500, 0, 0);
        tracker.record("gpt-4o-2024-05-13", 2000, 1000, 0, 0);
        tracker.record("claude-sonnet-4-20250514", 3000, 1500, 0, 0);
        let summary = tracker.summary();
        assert_eq!(summary.total_requests, 3);
        assert_eq!(summary.per_model.len(), 2);
        // Find sonnet entry
        let sonnet = summary
            .per_model
            .iter()
            .find(|e| e.model.contains("sonnet"))
            .unwrap();
        assert_eq!(sonnet.requests, 2);
        assert_eq!(sonnet.input_tokens, 4000);
        assert_eq!(sonnet.output_tokens, 2000);
    }

    #[test]
    fn test_total_usd() {
        let tracker = CostTracker::new();
        tracker.record("claude-sonnet-4-20250514", 1_000_000, 0, 0, 0);
        assert!((tracker.total_usd() - 3.0).abs() < 0.01);
    }

    #[test]
    fn test_reset() {
        let tracker = CostTracker::new();
        tracker.record("claude-sonnet-4-20250514", 1000, 500, 0, 0);
        tracker.reset();
        let summary = tracker.summary();
        assert_eq!(summary.total_requests, 0);
        assert!((summary.total_usd - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_unknown_model_fallback() {
        let tracker = CostTracker::new();
        let cost = tracker.record("some-unknown-model", 1_000_000, 1_000_000, 0, 0);
        // unknown: $3/M input + $15/M output = $3 + $15 = $18
        assert!((cost - 18.0).abs() < 0.01);
    }

    #[test]
    fn test_summary_sorted_by_cost() {
        let tracker = CostTracker::new();
        // Record cheap model first
        tracker.record("gpt-4o-mini-2024-07-18", 1_000_000, 0, 0, 0);
        // Record expensive model second
        tracker.record("claude-opus-4-20250514", 1_000_000, 0, 0, 0);
        let summary = tracker.summary();
        assert_eq!(summary.per_model.len(), 2);
        // Opus should be first (more expensive)
        assert!(summary.per_model[0].model.contains("opus"));
    }
}
