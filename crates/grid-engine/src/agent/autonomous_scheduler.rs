use dashmap::DashMap;
use grid_types::SessionId;
use tracing::{info, warn};

use super::autonomous::{AutonomousState, AutonomousStatus};

/// Manages multiple autonomous agent sessions.
///
/// Provides a registry of active autonomous sessions with lifecycle management.
/// Lives as a field on `AgentRuntime` (Phase AU-G2).
pub struct AutonomousScheduler {
    /// Active autonomous sessions indexed by session ID.
    sessions: DashMap<SessionId, AutonomousState>,
}

impl AutonomousScheduler {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
        }
    }

    /// Register a session as autonomous.
    pub fn register(&self, state: AutonomousState) {
        let session_id = state.session_id.clone();
        info!(session_id = %session_id.as_str(), "AutonomousScheduler: registering session");
        self.sessions.insert(session_id, state);
    }

    /// Unregister a session.
    pub fn unregister(&self, session_id: &SessionId) -> Option<AutonomousState> {
        let removed = self.sessions.remove(session_id).map(|(_, v)| v);
        if removed.is_some() {
            info!(session_id = %session_id.as_str(), "AutonomousScheduler: unregistered session");
        } else {
            warn!(session_id = %session_id.as_str(), "AutonomousScheduler: session not found for unregister");
        }
        removed
    }

    /// Get a clone of session state.
    pub fn get(&self, session_id: &SessionId) -> Option<AutonomousState> {
        self.sessions.get(session_id).map(|r| r.clone())
    }

    /// List all active autonomous sessions.
    pub fn list(&self) -> Vec<AutonomousState> {
        self.sessions.iter().map(|r| r.value().clone()).collect()
    }

    /// Update session status.
    pub fn update_status(&self, session_id: &SessionId, status: AutonomousStatus) {
        if let Some(mut entry) = self.sessions.get_mut(session_id) {
            entry.status = status;
        }
    }

    /// Number of active sessions.
    pub fn active_count(&self) -> usize {
        self.sessions.len()
    }
}

impl Default for AutonomousScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::autonomous::AutonomousConfig;

    fn make_state(id: &str) -> AutonomousState {
        AutonomousState::new(
            SessionId::from_string(id),
            AutonomousConfig::default(),
        )
    }

    #[test]
    fn test_scheduler_register_unregister() {
        let sched = AutonomousScheduler::new();
        let state = make_state("s1");

        sched.register(state);
        assert_eq!(sched.active_count(), 1);
        assert!(sched.get(&SessionId::from_string("s1")).is_some());

        let removed = sched.unregister(&SessionId::from_string("s1"));
        assert!(removed.is_some());
        assert_eq!(sched.active_count(), 0);
    }

    #[test]
    fn test_scheduler_list_and_count() {
        let sched = AutonomousScheduler::new();
        sched.register(make_state("s1"));
        sched.register(make_state("s2"));
        sched.register(make_state("s3"));

        assert_eq!(sched.active_count(), 3);
        let list = sched.list();
        assert_eq!(list.len(), 3);
    }

    #[test]
    fn test_scheduler_update_status() {
        let sched = AutonomousScheduler::new();
        sched.register(make_state("s1"));

        sched.update_status(&SessionId::from_string("s1"), AutonomousStatus::Paused);
        let state = sched.get(&SessionId::from_string("s1")).unwrap();
        assert_eq!(state.status, AutonomousStatus::Paused);
    }

    #[test]
    fn test_scheduler_unregister_nonexistent() {
        let sched = AutonomousScheduler::new();
        let removed = sched.unregister(&SessionId::from_string("nonexistent"));
        assert!(removed.is_none());
    }
}
