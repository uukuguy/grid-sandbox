//! Action tracker for rate limiting and cost control.

use std::sync::Mutex;
use std::time::Instant;

/// Sliding-window action tracker for rate limiting.
#[derive(Debug)]
pub struct ActionTracker {
    /// Timestamps of recent actions (kept within the window).
    actions: Mutex<Vec<Instant>>,
    /// Window duration in seconds (default: 3600 = 1 hour)
    window_secs: u64,
}

impl ActionTracker {
    /// Create a new action tracker with default 1-hour window.
    pub fn new() -> Self {
        Self::with_window(3600)
    }

    /// Create a new action tracker with custom window duration.
    pub fn with_window(window_secs: u64) -> Self {
        Self {
            actions: Mutex::new(Vec::new()),
            window_secs,
        }
    }

    /// Record an action and return the current count within the window.
    pub fn record(&self) -> usize {
        let mut actions = self.actions.lock().unwrap_or_else(|e| e.into_inner());
        let cutoff = Instant::now()
            .checked_sub(std::time::Duration::from_secs(self.window_secs))
            .unwrap_or_else(Instant::now);
        actions.retain(|t| *t > cutoff);
        actions.push(Instant::now());
        actions.len()
    }

    /// Count of actions in the current window without recording.
    pub fn count(&self) -> usize {
        let mut actions = self.actions.lock().unwrap_or_else(|e| e.into_inner());
        let cutoff = Instant::now()
            .checked_sub(std::time::Duration::from_secs(self.window_secs))
            .unwrap_or_else(Instant::now);
        actions.retain(|t| *t > cutoff);
        actions.len()
    }

    /// Check if the action limit would be exceeded.
    /// Returns Ok(()) if under limit, Err(count) if over limit.
    pub fn check_limit(&self, limit: usize) -> Result<(), usize> {
        let count = self.count();
        if count >= limit {
            Err(count)
        } else {
            Ok(())
        }
    }

    /// Clear all tracked actions.
    pub fn clear(&self) {
        let mut actions = self.actions.lock().unwrap_or_else(|e| e.into_inner());
        actions.clear();
    }
}

impl Default for ActionTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ActionTracker {
    fn clone(&self) -> Self {
        let actions = self.actions.lock().unwrap_or_else(|e| e.into_inner());
        Self {
            actions: Mutex::new(actions.clone()),
            window_secs: self.window_secs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_and_count() {
        let tracker = ActionTracker::new();
        assert_eq!(tracker.count(), 0);

        tracker.record();
        assert_eq!(tracker.count(), 1);

        tracker.record();
        assert_eq!(tracker.count(), 2);
    }

    #[test]
    fn test_check_limit() {
        let tracker = ActionTracker::new();

        assert!(tracker.check_limit(5).is_ok());
        tracker.record();
        assert!(tracker.check_limit(5).is_ok());
        tracker.record();
        assert!(tracker.check_limit(2).is_err());
    }

    #[test]
    fn test_clear() {
        let tracker = ActionTracker::new();
        tracker.record();
        tracker.record();
        assert_eq!(tracker.count(), 2);

        tracker.clear();
        assert_eq!(tracker.count(), 0);
    }
}
