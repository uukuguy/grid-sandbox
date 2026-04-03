//! Simple per-tool rate limiter using a sliding window.
//!
//! Tracks invocation timestamps per tool name and rejects calls that exceed
//! the tool's declared `rate_limit()` (max invocations per 60-second window).

use std::collections::HashMap;
use std::time::Instant;

/// Sliding-window rate limiter for tool invocations.
///
/// Each tool name maps to a list of recent invocation timestamps.
/// Before execution, stale entries (older than 60 s) are pruned and the
/// current count is compared against the tool's declared limit.
pub struct ToolRateLimiter {
    /// tool_name -> list of invocation timestamps within the current window.
    invocations: HashMap<String, Vec<Instant>>,
}

/// Duration of the sliding window.
const WINDOW_SECS: u64 = 60;

impl ToolRateLimiter {
    pub fn new() -> Self {
        Self {
            invocations: HashMap::new(),
        }
    }

    /// Check whether the tool may be invoked and, if so, record the invocation.
    ///
    /// * `tool_name` — name of the tool being invoked.
    /// * `limit` — maximum allowed invocations per 60-second window.
    ///   A value of `0` means unlimited (always returns `true`).
    ///
    /// Returns `true` if the call is allowed, `false` if the rate limit is
    /// exceeded.
    pub fn check_and_record(&mut self, tool_name: &str, limit: u32) -> bool {
        if limit == 0 {
            return true;
        }

        let now = Instant::now();
        let window = std::time::Duration::from_secs(WINDOW_SECS);

        let entries = self.invocations.entry(tool_name.to_string()).or_default();

        // Prune entries older than the window.
        entries.retain(|&t| now.duration_since(t) < window);

        if entries.len() >= limit as usize {
            return false;
        }

        entries.push(now);
        true
    }
}

impl Default for ToolRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unlimited_always_allowed() {
        let mut limiter = ToolRateLimiter::new();
        for _ in 0..1000 {
            assert!(limiter.check_and_record("any_tool", 0));
        }
    }

    #[test]
    fn test_limit_enforced() {
        let mut limiter = ToolRateLimiter::new();
        // Allow up to 3 calls.
        assert!(limiter.check_and_record("tool_a", 3));
        assert!(limiter.check_and_record("tool_a", 3));
        assert!(limiter.check_and_record("tool_a", 3));
        // 4th call should be rejected.
        assert!(!limiter.check_and_record("tool_a", 3));
    }

    #[test]
    fn test_different_tools_independent() {
        let mut limiter = ToolRateLimiter::new();
        assert!(limiter.check_and_record("tool_a", 1));
        assert!(!limiter.check_and_record("tool_a", 1)); // exhausted
        // tool_b should still be available.
        assert!(limiter.check_and_record("tool_b", 1));
    }

    #[test]
    fn test_default_trait() {
        let limiter = ToolRateLimiter::default();
        assert!(limiter.invocations.is_empty());
    }
}
