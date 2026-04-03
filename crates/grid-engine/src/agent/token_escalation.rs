//! TokenEscalation — tiered max_output_tokens auto-upgrader.
//!
//! When the LLM truncates output due to `max_tokens`, the escalator promotes
//! to the next tier and retries with a larger buffer, saving a full
//! ContinuationTracker round-trip.

use tracing::debug;

/// Tiered max_tokens escalator.
///
/// Default tiers: 4096 → 8192 → 16384 → 32768 → 65536.
/// On each `escalate()` call, advances to the next tier.
/// Returns `None` when the top tier is already reached.
pub struct TokenEscalation {
    tiers: Vec<u32>,
    current_tier: usize,
}

impl TokenEscalation {
    /// Create with default tier progression.
    pub fn new() -> Self {
        Self {
            tiers: vec![4096, 8192, 16384, 32768, 65536],
            current_tier: 0,
        }
    }

    /// Create with custom tiers. Panics if `tiers` is empty.
    pub fn with_tiers(tiers: Vec<u32>) -> Self {
        assert!(!tiers.is_empty(), "TokenEscalation requires at least one tier");
        Self { tiers, current_tier: 0 }
    }

    /// Current max_tokens value.
    pub fn current(&self) -> u32 {
        self.tiers[self.current_tier]
    }

    /// Try to escalate to the next tier.
    /// Returns `Some(new_max)` on success, `None` if already at top.
    pub fn escalate(&mut self) -> Option<u32> {
        if self.current_tier + 1 < self.tiers.len() {
            self.current_tier += 1;
            debug!(
                old_tier = self.current_tier - 1,
                new_tier = self.current_tier,
                new_max = self.tiers[self.current_tier],
                "TokenEscalation: upgraded"
            );
            Some(self.tiers[self.current_tier])
        } else {
            None
        }
    }

    /// Reset to the initial tier (call at the start of each new turn).
    pub fn reset(&mut self) {
        self.current_tier = 0;
    }

    /// Whether escalation has been used (current tier > 0).
    pub fn has_escalated(&self) -> bool {
        self.current_tier > 0
    }
}

impl Default for TokenEscalation {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escalate_chain() {
        let mut esc = TokenEscalation::new();
        assert_eq!(esc.current(), 4096);
        assert_eq!(esc.escalate(), Some(8192));
        assert_eq!(esc.current(), 8192);
        assert_eq!(esc.escalate(), Some(16384));
        assert_eq!(esc.escalate(), Some(32768));
        assert_eq!(esc.escalate(), Some(65536));
    }

    #[test]
    fn test_escalate_at_top_returns_none() {
        let mut esc = TokenEscalation::new();
        for _ in 0..4 {
            esc.escalate();
        }
        assert_eq!(esc.current(), 65536);
        assert_eq!(esc.escalate(), None);
        // Should stay at top
        assert_eq!(esc.current(), 65536);
    }

    #[test]
    fn test_reset() {
        let mut esc = TokenEscalation::new();
        esc.escalate();
        esc.escalate();
        assert!(esc.has_escalated());
        esc.reset();
        assert_eq!(esc.current(), 4096);
        assert!(!esc.has_escalated());
    }

    #[test]
    fn test_custom_tiers() {
        let mut esc = TokenEscalation::with_tiers(vec![1000, 2000]);
        assert_eq!(esc.current(), 1000);
        assert_eq!(esc.escalate(), Some(2000));
        assert_eq!(esc.escalate(), None);
    }
}
