//! Self-repair system for detecting stuck agents and attempting automatic recovery.
//!
//! When an agent repeatedly fails on the same tool, the `SelfRepairManager` detects
//! the stuck state and returns repair actions that the harness can use to either
//! replace the tool output or signal the LLM to change strategy.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Result of a repair check.
#[derive(Debug, Clone)]
pub enum RepairResult {
    /// Repair succeeded — use this replacement output instead of the error.
    Repaired(String),
    /// The tool was rebuilt (e.g. MCP reconnect) and should be retried.
    ToolRebuilt { tool_name: String },
    /// Cannot be repaired — the caller should warn the LLM.
    Unrecoverable { reason: String },
    /// No repair needed — the tool output is fine.
    NotNeeded,
}

/// Why the agent is considered stuck.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StuckReason {
    /// The same tool failed too many times in a row.
    ConsecutiveFailures { tool_name: String, count: usize },
    /// No successful tool call for too long.
    NoProgressTimeout { elapsed: Duration },
    /// Not stuck.
    NotStuck,
}

/// Detects when an agent is stuck based on consecutive failures and timeouts.
pub struct StuckDetector {
    /// Maximum consecutive failures before declaring stuck.
    pub max_consecutive_failures: usize,
    /// Maximum time without a successful tool call.
    pub no_progress_timeout: Duration,
    /// Per-tool consecutive failure counts.
    tool_failure_counts: HashMap<String, usize>,
    /// Timestamp of the last successful tool call.
    last_progress_at: Instant,
}

impl StuckDetector {
    pub fn new(max_consecutive_failures: usize, no_progress_timeout: Duration) -> Self {
        Self {
            max_consecutive_failures,
            no_progress_timeout,
            tool_failure_counts: HashMap::new(),
            last_progress_at: Instant::now(),
        }
    }

    /// Record a successful tool call — resets the failure count for that tool.
    pub fn record_success(&mut self, tool_name: &str) {
        self.tool_failure_counts.remove(tool_name);
        self.last_progress_at = Instant::now();
    }

    /// Record a failed tool call — increments the failure count.
    pub fn record_failure(&mut self, tool_name: &str) {
        let count = self.tool_failure_counts.entry(tool_name.to_string()).or_insert(0);
        *count += 1;
    }

    /// Check whether the agent is stuck.
    pub fn is_stuck(&self, tool_name: &str) -> StuckReason {
        // Check consecutive failures for this specific tool
        if let Some(&count) = self.tool_failure_counts.get(tool_name) {
            if count >= self.max_consecutive_failures {
                return StuckReason::ConsecutiveFailures {
                    tool_name: tool_name.to_string(),
                    count,
                };
            }
        }

        // Check no-progress timeout
        let elapsed = self.last_progress_at.elapsed();
        if elapsed >= self.no_progress_timeout {
            return StuckReason::NoProgressTimeout { elapsed };
        }

        StuckReason::NotStuck
    }

    /// Reset all state.
    pub fn reset(&mut self) {
        self.tool_failure_counts.clear();
        self.last_progress_at = Instant::now();
    }
}

/// Manages self-repair logic for the agent loop.
pub struct SelfRepairManager {
    detector: StuckDetector,
    /// Maximum number of repair attempts per tool before giving up.
    max_repair_attempts: usize,
    /// Tracks how many times we have attempted repair per tool.
    repair_attempts: HashMap<String, usize>,
}

impl SelfRepairManager {
    pub fn new(max_consecutive_failures: usize, no_progress_timeout: Duration, max_repair_attempts: usize) -> Self {
        Self {
            detector: StuckDetector::new(max_consecutive_failures, no_progress_timeout),
            max_repair_attempts,
            repair_attempts: HashMap::new(),
        }
    }

    /// Create with sensible defaults (3 consecutive failures, 60s timeout, 2 repair attempts).
    pub fn with_defaults() -> Self {
        Self::new(3, Duration::from_secs(60), 2)
    }

    /// Check a tool result and decide whether repair is needed.
    ///
    /// `is_error` indicates whether the tool execution returned an error.
    pub fn check_and_repair(&mut self, tool_name: &str, is_error: bool) -> RepairResult {
        if !is_error {
            self.detector.record_success(tool_name);
            self.repair_attempts.remove(tool_name);
            return RepairResult::NotNeeded;
        }

        // Record the failure
        self.detector.record_failure(tool_name);

        // Check if stuck
        let stuck = self.detector.is_stuck(tool_name);
        if stuck == StuckReason::NotStuck {
            return RepairResult::NotNeeded;
        }

        // Check if we've exhausted repair attempts
        let attempts = self.repair_attempts.entry(tool_name.to_string()).or_insert(0);
        *attempts += 1;

        if *attempts > self.max_repair_attempts {
            return RepairResult::Unrecoverable {
                reason: format!(
                    "Tool '{}' has failed {} consecutive times and {} repair attempts were exhausted",
                    tool_name,
                    self.detector.tool_failure_counts.get(tool_name).copied().unwrap_or(0),
                    self.max_repair_attempts,
                ),
            };
        }

        // Generate a repair hint
        let hint = self.generate_fallback_hint(tool_name);
        RepairResult::Repaired(hint)
    }

    /// Generate a fallback hint telling the LLM to try a different approach.
    pub fn generate_fallback_hint(&self, tool_name: &str) -> String {
        let failure_count = self.detector.tool_failure_counts.get(tool_name).copied().unwrap_or(0);
        format!(
            "[Self-Repair] Tool '{}' has failed {} consecutive times. \
             Please try a different approach: use an alternative tool, \
             simplify the input, or break the task into smaller steps.",
            tool_name, failure_count
        )
    }

    /// Reset all state (useful for testing or session restart).
    pub fn reset(&mut self) {
        self.detector.reset();
        self.repair_attempts.clear();
    }

    /// Access the underlying detector (for advanced queries).
    pub fn detector(&self) -> &StuckDetector {
        &self.detector
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_success_does_not_trigger_repair() {
        let mut mgr = SelfRepairManager::with_defaults();
        let result = mgr.check_and_repair("bash", false);
        assert!(matches!(result, RepairResult::NotNeeded));
    }

    #[test]
    fn test_single_failure_does_not_trigger_stuck() {
        let mut mgr = SelfRepairManager::with_defaults();
        let result = mgr.check_and_repair("bash", true);
        // Only 1 failure, threshold is 3
        assert!(matches!(result, RepairResult::NotNeeded));
    }

    #[test]
    fn test_consecutive_failures_trigger_stuck() {
        let mut mgr = SelfRepairManager::with_defaults(); // threshold = 3
        mgr.check_and_repair("bash", true); // 1
        mgr.check_and_repair("bash", true); // 2
        let result = mgr.check_and_repair("bash", true); // 3 -> stuck
        assert!(matches!(result, RepairResult::Repaired(_)));
    }

    #[test]
    fn test_repair_exhaustion_returns_unrecoverable() {
        let mut mgr = SelfRepairManager::new(2, Duration::from_secs(60), 2);
        mgr.check_and_repair("bash", true); // 1
        let r1 = mgr.check_and_repair("bash", true); // 2 -> stuck, repair attempt 1
        assert!(matches!(r1, RepairResult::Repaired(_)));

        let r2 = mgr.check_and_repair("bash", true); // 3 -> repair attempt 2
        assert!(matches!(r2, RepairResult::Repaired(_)));

        let r3 = mgr.check_and_repair("bash", true); // 4 -> exhausted
        assert!(matches!(r3, RepairResult::Unrecoverable { .. }));
    }

    #[test]
    fn test_success_resets_failure_count() {
        let mut mgr = SelfRepairManager::with_defaults();
        mgr.check_and_repair("bash", true); // 1
        mgr.check_and_repair("bash", true); // 2
        mgr.check_and_repair("bash", false); // success -> resets
        let result = mgr.check_and_repair("bash", true); // 1 again
        assert!(matches!(result, RepairResult::NotNeeded));
    }

    #[test]
    fn test_no_progress_timeout() {
        let mut mgr = SelfRepairManager::new(100, Duration::from_millis(0), 2);
        // With 0ms timeout, even a single failure triggers stuck via timeout
        // Need at least 1 failure for the tool to be in failure state
        // But is_stuck checks timeout regardless of failure count
        // Actually, is_stuck checks consecutive failures first, then timeout
        // With max_consecutive_failures=100 and 0ms timeout, the timeout path fires
        let result = mgr.check_and_repair("bash", true);
        // 1 failure < 100, but 0ms timeout fires
        assert!(matches!(result, RepairResult::Repaired(_)));
    }

    #[test]
    fn test_generate_fallback_hint_nonempty() {
        let mut mgr = SelfRepairManager::with_defaults();
        mgr.check_and_repair("file_read", true);
        let hint = mgr.generate_fallback_hint("file_read");
        assert!(!hint.is_empty());
        assert!(hint.contains("file_read"));
    }

    #[test]
    fn test_reset_clears_state() {
        let mut mgr = SelfRepairManager::with_defaults();
        mgr.check_and_repair("bash", true);
        mgr.check_and_repair("bash", true);
        mgr.check_and_repair("bash", true);
        mgr.reset();

        // After reset, failure count should be 0
        let result = mgr.check_and_repair("bash", true);
        assert!(matches!(result, RepairResult::NotNeeded));
    }

    #[test]
    fn test_different_tools_tracked_independently() {
        let mut mgr = SelfRepairManager::with_defaults();
        mgr.check_and_repair("bash", true); // bash: 1
        mgr.check_and_repair("bash", true); // bash: 2
        mgr.check_and_repair("file_read", true); // file_read: 1
        let result = mgr.check_and_repair("file_read", true); // file_read: 2
        // Neither tool has reached threshold of 3
        assert!(matches!(result, RepairResult::NotNeeded));
    }

    #[test]
    fn test_stuck_detector_reset() {
        let mut detector = StuckDetector::new(2, Duration::from_secs(60));
        detector.record_failure("bash");
        detector.record_failure("bash");
        assert!(matches!(detector.is_stuck("bash"), StuckReason::ConsecutiveFailures { .. }));

        detector.reset();
        assert_eq!(detector.is_stuck("bash"), StuckReason::NotStuck);
    }
}
