//! Emergency Stop (E-Stop) — cooperative kill switch for agent loops.
//!
//! `EmergencyStop` provides an atomic flag + broadcast channel that any
//! component can trigger to halt the agent loop immediately.  Multiple
//! subscribers can listen for the stop signal concurrently.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::Serialize;
use tokio::sync::broadcast;

/// Reason for triggering the emergency stop.
#[derive(Debug, Clone, Serialize)]
pub enum EStopReason {
    /// Explicit user-initiated stop.
    UserTriggered,
    /// A safety policy violation was detected.
    SafetyViolation(String),
    /// Token or cost budget has been exceeded.
    BudgetExceeded,
    /// The host system is shutting down.
    SystemShutdown,
}

impl std::fmt::Display for EStopReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UserTriggered => write!(f, "user triggered"),
            Self::SafetyViolation(msg) => write!(f, "safety violation: {msg}"),
            Self::BudgetExceeded => write!(f, "budget exceeded"),
            Self::SystemShutdown => write!(f, "system shutdown"),
        }
    }
}

/// Cooperative emergency stop mechanism for agent loops.
///
/// Designed for shared ownership (`Clone` yields the same underlying state).
/// Subscribers receive the `EStopReason` via a broadcast channel; pollers
/// can check `is_triggered()` cheaply via an atomic flag.
#[derive(Clone)]
pub struct EmergencyStop {
    triggered: Arc<AtomicBool>,
    notify_tx: broadcast::Sender<EStopReason>,
    reason: Arc<std::sync::Mutex<Option<EStopReason>>>,
}

impl std::fmt::Debug for EmergencyStop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmergencyStop")
            .field("triggered", &self.is_triggered())
            .finish()
    }
}

impl EmergencyStop {
    /// Create a new, un-triggered emergency stop.
    pub fn new() -> Self {
        let (notify_tx, _) = broadcast::channel(16);
        Self {
            triggered: Arc::new(AtomicBool::new(false)),
            notify_tx,
            reason: Arc::new(std::sync::Mutex::new(None)),
        }
    }

    /// Trigger the emergency stop with the given reason.
    ///
    /// Sets the atomic flag and broadcasts the reason to all subscribers.
    /// Calling `trigger` on an already-triggered stop is a no-op.
    pub fn trigger(&self, reason: EStopReason) {
        if self
            .triggered
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            {
                let mut guard = self.reason.lock().expect("estop reason mutex poisoned");
                *guard = Some(reason.clone());
            }
            // Ignore send error — means no active receivers, which is fine.
            let _ = self.notify_tx.send(reason);
        }
    }

    /// Returns `true` if the emergency stop has been triggered.
    pub fn is_triggered(&self) -> bool {
        self.triggered.load(Ordering::SeqCst)
    }

    /// Subscribe to emergency stop notifications.
    ///
    /// The returned receiver will yield the `EStopReason` when `trigger()` is called.
    pub fn subscribe(&self) -> broadcast::Receiver<EStopReason> {
        self.notify_tx.subscribe()
    }

    /// Reset the emergency stop to the un-triggered state.
    ///
    /// Clears the atomic flag and stored reason.  Existing subscribers are
    /// **not** affected — they keep their current receiver.
    pub fn reset(&self) {
        self.triggered.store(false, Ordering::SeqCst);
        let mut guard = self.reason.lock().expect("estop reason mutex poisoned");
        *guard = None;
    }

    /// Returns the reason for the emergency stop, if triggered.
    pub fn reason(&self) -> Option<EStopReason> {
        let guard = self.reason.lock().expect("estop reason mutex poisoned");
        guard.clone()
    }
}

impl Default for EmergencyStop {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_triggered_initially() {
        let estop = EmergencyStop::new();
        assert!(!estop.is_triggered());
        assert!(estop.reason().is_none());
    }

    #[test]
    fn test_trigger_sets_flag() {
        let estop = EmergencyStop::new();
        estop.trigger(EStopReason::UserTriggered);
        assert!(estop.is_triggered());
    }

    #[test]
    fn test_trigger_stores_reason() {
        let estop = EmergencyStop::new();
        estop.trigger(EStopReason::SafetyViolation("bad input".into()));
        let reason = estop.reason().expect("reason should be set");
        match reason {
            EStopReason::SafetyViolation(msg) => assert_eq!(msg, "bad input"),
            _ => panic!("unexpected reason variant"),
        }
    }

    #[test]
    fn test_reset_clears_state() {
        let estop = EmergencyStop::new();
        estop.trigger(EStopReason::BudgetExceeded);
        assert!(estop.is_triggered());

        estop.reset();
        assert!(!estop.is_triggered());
        assert!(estop.reason().is_none());
    }

    #[test]
    fn test_clone_shares_state() {
        let estop1 = EmergencyStop::new();
        let estop2 = estop1.clone();

        estop1.trigger(EStopReason::SystemShutdown);
        assert!(estop2.is_triggered());
    }

    #[test]
    fn test_double_trigger_is_noop() {
        let estop = EmergencyStop::new();
        estop.trigger(EStopReason::UserTriggered);
        estop.trigger(EStopReason::BudgetExceeded);

        // First reason wins
        match estop.reason().unwrap() {
            EStopReason::UserTriggered => {}
            _ => panic!("first reason should be preserved"),
        }
    }

    #[tokio::test]
    async fn test_subscriber_receives_notification() {
        let estop = EmergencyStop::new();
        let mut rx = estop.subscribe();

        estop.trigger(EStopReason::UserTriggered);

        let reason = rx.recv().await.expect("should receive reason");
        match reason {
            EStopReason::UserTriggered => {}
            _ => panic!("unexpected reason"),
        }
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let estop = EmergencyStop::new();
        let mut rx1 = estop.subscribe();
        let mut rx2 = estop.subscribe();

        estop.trigger(EStopReason::BudgetExceeded);

        let r1 = rx1.recv().await.expect("rx1 should receive");
        let r2 = rx2.recv().await.expect("rx2 should receive");

        assert!(matches!(r1, EStopReason::BudgetExceeded));
        assert!(matches!(r2, EStopReason::BudgetExceeded));
    }

    #[test]
    fn test_display_formatting() {
        assert_eq!(EStopReason::UserTriggered.to_string(), "user triggered");
        assert_eq!(EStopReason::BudgetExceeded.to_string(), "budget exceeded");
        assert_eq!(EStopReason::SystemShutdown.to_string(), "system shutdown");
        assert_eq!(
            EStopReason::SafetyViolation("test".into()).to_string(),
            "safety violation: test"
        );
    }
}
