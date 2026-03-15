//! Session lifecycle events for real-time notifications
//!
//! Provides a [`SessionEvent`] enum for broadcasting session state changes
//! and a [`SessionEventBus`] wrapper around `tokio::sync::broadcast`.

use chrono::{DateTime, Utc};
use tokio::sync::broadcast;

/// Events emitted during session lifecycle changes.
///
/// Consumers (e.g. TUI, WebSocket relay) subscribe via [`SessionEventBus`]
/// to receive these notifications in real time.
#[derive(Debug, Clone)]
pub enum SessionEvent {
    /// A new session was created
    Created {
        session_id: String,
        agent_id: Option<String>,
        at: DateTime<Utc>,
    },
    /// A message was added to a session
    MessageAdded {
        session_id: String,
        role: String,
        at: DateTime<Utc>,
    },
    /// Session context usage was updated
    ContextUpdated {
        session_id: String,
        token_count: usize,
        at: DateTime<Utc>,
    },
    /// A session was closed / deleted
    Closed {
        session_id: String,
        at: DateTime<Utc>,
    },
}

/// Wraps a `tokio::sync::broadcast` channel for [`SessionEvent`].
///
/// The bus is cheaply cloneable (inner `Sender` is `Arc`-based) so it can be
/// shared across multiple session store implementations and subscribers.
#[derive(Debug, Clone)]
pub struct SessionEventBus {
    tx: broadcast::Sender<SessionEvent>,
}

impl SessionEventBus {
    /// Create a new bus with the given channel capacity.
    ///
    /// A capacity of 64-256 is usually sufficient; slow receivers that fall
    /// behind will see `RecvError::Lagged`.
    pub fn new(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity);
        Self { tx }
    }

    /// Emit a session event. Silently ignores the case where no receivers
    /// are currently subscribed.
    pub fn emit(&self, event: SessionEvent) {
        // send returns Err only when there are zero active receivers — that is
        // perfectly fine (nobody listening yet).
        let _ = self.tx.send(event);
    }

    /// Subscribe to session events. Returns a new receiver handle.
    pub fn subscribe(&self) -> broadcast::Receiver<SessionEvent> {
        self.tx.subscribe()
    }

    /// Return the current number of active receivers.
    pub fn receiver_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

impl Default for SessionEventBus {
    fn default() -> Self {
        Self::new(128)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_event_created_fields() {
        let event = SessionEvent::Created {
            session_id: "s-1".to_string(),
            agent_id: Some("agent-0".to_string()),
            at: Utc::now(),
        };
        if let SessionEvent::Created {
            session_id,
            agent_id,
            ..
        } = &event
        {
            assert_eq!(session_id, "s-1");
            assert_eq!(agent_id.as_deref(), Some("agent-0"));
        } else {
            panic!("expected Created variant");
        }
    }

    #[test]
    fn session_event_message_added() {
        let event = SessionEvent::MessageAdded {
            session_id: "s-2".to_string(),
            role: "user".to_string(),
            at: Utc::now(),
        };
        if let SessionEvent::MessageAdded { role, .. } = &event {
            assert_eq!(role, "user");
        } else {
            panic!("expected MessageAdded variant");
        }
    }

    #[test]
    fn session_event_context_updated() {
        let event = SessionEvent::ContextUpdated {
            session_id: "s-3".to_string(),
            token_count: 4096,
            at: Utc::now(),
        };
        if let SessionEvent::ContextUpdated { token_count, .. } = &event {
            assert_eq!(*token_count, 4096);
        } else {
            panic!("expected ContextUpdated variant");
        }
    }

    #[test]
    fn session_event_closed() {
        let event = SessionEvent::Closed {
            session_id: "s-4".to_string(),
            at: Utc::now(),
        };
        if let SessionEvent::Closed { session_id, .. } = &event {
            assert_eq!(session_id, "s-4");
        } else {
            panic!("expected Closed variant");
        }
    }

    #[test]
    fn event_bus_new_has_zero_receivers() {
        let bus = SessionEventBus::new(16);
        assert_eq!(bus.receiver_count(), 0);
    }

    #[test]
    fn event_bus_default_creates_bus() {
        let bus = SessionEventBus::default();
        assert_eq!(bus.receiver_count(), 0);
    }

    #[test]
    fn event_bus_emit_without_receivers_does_not_panic() {
        let bus = SessionEventBus::new(16);
        bus.emit(SessionEvent::Closed {
            session_id: "s-x".to_string(),
            at: Utc::now(),
        });
        // no panic = success
    }

    #[tokio::test]
    async fn event_bus_subscribe_and_receive() {
        let bus = SessionEventBus::new(16);
        let mut rx = bus.subscribe();
        assert_eq!(bus.receiver_count(), 1);

        bus.emit(SessionEvent::Created {
            session_id: "s-10".to_string(),
            agent_id: None,
            at: Utc::now(),
        });

        let event = rx.recv().await.expect("should receive event");
        if let SessionEvent::Created { session_id, .. } = event {
            assert_eq!(session_id, "s-10");
        } else {
            panic!("expected Created");
        }
    }

    #[tokio::test]
    async fn event_bus_multiple_subscribers() {
        let bus = SessionEventBus::new(16);
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();
        assert_eq!(bus.receiver_count(), 2);

        bus.emit(SessionEvent::Closed {
            session_id: "s-11".to_string(),
            at: Utc::now(),
        });

        let e1 = rx1.recv().await.expect("rx1 should receive");
        let e2 = rx2.recv().await.expect("rx2 should receive");

        if let (SessionEvent::Closed { session_id: id1, .. }, SessionEvent::Closed { session_id: id2, .. }) = (&e1, &e2) {
            assert_eq!(id1, "s-11");
            assert_eq!(id2, "s-11");
        } else {
            panic!("expected Closed on both receivers");
        }
    }

    #[test]
    fn event_bus_clone_shares_channel() {
        let bus1 = SessionEventBus::new(16);
        let _rx = bus1.subscribe();
        let bus2 = bus1.clone();
        // Both clones see the same receiver count
        assert_eq!(bus2.receiver_count(), 1);
    }
}
