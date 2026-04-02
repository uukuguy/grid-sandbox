//! Autonomous mode trigger sources (AR-T5, AR-T6).
//!
//! Provides trait abstraction for different trigger mechanisms that can
//! start autonomous agent sessions. Includes:
//! - `ChannelTriggerSource` — for webhook HTTP → internal dispatch
//! - `PollingTriggerSource` — for MQ-style polling (Redis LPOP, NATS, file, etc.)
//! - `TriggerListener` — background listener that dispatches triggers to `AgentRuntime`

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use octo_types::SessionId;

use super::autonomous::AutonomousConfig;

/// A trigger event that initiates an autonomous agent session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerEvent {
    /// Optional session to use (creates new if None).
    #[serde(default)]
    pub session_id: Option<SessionId>,
    /// Override autonomous configuration.
    #[serde(default)]
    pub config_override: Option<AutonomousConfig>,
    /// Arbitrary payload data.
    #[serde(default)]
    pub payload: serde_json::Value,
}

/// Trait for sources that produce trigger events.
#[async_trait]
pub trait TriggerSource: Send + Sync {
    /// Wait for and return the next trigger event.
    async fn next_trigger(&mut self) -> anyhow::Result<TriggerEvent>;
    /// Human-readable name for logging.
    fn name(&self) -> &str;
}

// ---------------------------------------------------------------------------
// AR-T5: Channel-based trigger (for Webhook HTTP → internal dispatch)
// ---------------------------------------------------------------------------

/// Channel-based trigger source.
///
/// Webhook handlers send `TriggerEvent` into the channel sender;
/// the listener consumes them from the receiver.
pub struct ChannelTriggerSource {
    rx: mpsc::Receiver<TriggerEvent>,
    name: String,
}

impl ChannelTriggerSource {
    /// Create a new channel trigger. Returns `(source, sender)`.
    /// The sender is given to the HTTP handler; the source is registered
    /// with `TriggerListener`.
    pub fn new(name: &str) -> (Self, mpsc::Sender<TriggerEvent>) {
        let (tx, rx) = mpsc::channel(32);
        (
            Self {
                rx,
                name: name.to_string(),
            },
            tx,
        )
    }
}

#[async_trait]
impl TriggerSource for ChannelTriggerSource {
    async fn next_trigger(&mut self) -> anyhow::Result<TriggerEvent> {
        self.rx
            .recv()
            .await
            .ok_or_else(|| anyhow::anyhow!("channel closed"))
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// ---------------------------------------------------------------------------
// AR-T6: Polling-based trigger (for MQ adapters)
// ---------------------------------------------------------------------------

/// Generic polling trigger source.
///
/// Periodically calls a closure to check for new events. Suitable for
/// Redis LPOP, NATS subscribe, file-based queues, etc.
pub struct PollingTriggerSource {
    name: String,
    interval: Duration,
    poll_fn: Box<dyn Fn() -> Option<TriggerEvent> + Send + Sync>,
}

impl PollingTriggerSource {
    pub fn new(
        name: &str,
        interval: Duration,
        poll_fn: impl Fn() -> Option<TriggerEvent> + Send + Sync + 'static,
    ) -> Self {
        Self {
            name: name.to_string(),
            interval,
            poll_fn: Box::new(poll_fn),
        }
    }
}

#[async_trait]
impl TriggerSource for PollingTriggerSource {
    async fn next_trigger(&mut self) -> anyhow::Result<TriggerEvent> {
        loop {
            if let Some(event) = (self.poll_fn)() {
                return Ok(event);
            }
            tokio::time::sleep(self.interval).await;
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// ---------------------------------------------------------------------------
// TriggerListener — unified background listener
// ---------------------------------------------------------------------------

/// Background listener that monitors multiple trigger sources and dispatches
/// events to the agent runtime.
pub struct TriggerListener {
    sources: Vec<Box<dyn TriggerSource>>,
}

impl TriggerListener {
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    /// Register a trigger source.
    pub fn register(&mut self, source: Box<dyn TriggerSource>) {
        self.sources.push(source);
    }

    /// Number of registered sources.
    pub fn source_count(&self) -> usize {
        self.sources.len()
    }

    /// Start the listener loop. Each source is monitored in its own task.
    /// When a trigger fires, the callback is invoked with the event.
    pub fn start<F>(self, callback: Arc<F>) -> Vec<tokio::task::JoinHandle<()>>
    where
        F: Fn(TriggerEvent) + Send + Sync + 'static,
    {
        let mut handles = Vec::new();
        for mut source in self.sources {
            let cb = callback.clone();
            let handle = tokio::spawn(async move {
                loop {
                    match source.next_trigger().await {
                        Ok(event) => {
                            info!(
                                source = source.name(),
                                "TriggerListener: event received"
                            );
                            cb(event);
                        }
                        Err(e) => {
                            warn!(
                                source = source.name(),
                                error = %e,
                                "TriggerListener: source error, stopping"
                            );
                            break;
                        }
                    }
                }
                debug!(source = source.name(), "TriggerListener: source loop ended");
            });
            handles.push(handle);
        }
        handles
    }
}

impl Default for TriggerListener {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_channel_trigger_roundtrip() {
        let (mut source, tx) = ChannelTriggerSource::new("test-webhook");
        let event = TriggerEvent {
            session_id: None,
            config_override: None,
            payload: serde_json::json!({"action": "deploy"}),
        };
        tx.send(event.clone()).await.unwrap();
        let received = source.next_trigger().await.unwrap();
        assert_eq!(received.payload["action"], "deploy");
    }

    #[tokio::test]
    async fn test_polling_trigger() {
        use std::sync::atomic::{AtomicU32, Ordering};
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let mut source = PollingTriggerSource::new(
            "test-poll",
            Duration::from_millis(10),
            move || {
                let n = counter_clone.fetch_add(1, Ordering::SeqCst);
                if n >= 2 {
                    Some(TriggerEvent {
                        session_id: None,
                        config_override: None,
                        payload: serde_json::json!({"poll_count": n}),
                    })
                } else {
                    None
                }
            },
        );

        let event = source.next_trigger().await.unwrap();
        assert!(event.payload["poll_count"].as_u64().unwrap() >= 2);
    }

    #[test]
    fn test_trigger_listener_register() {
        let mut listener = TriggerListener::new();
        assert_eq!(listener.source_count(), 0);
        let (source, _tx) = ChannelTriggerSource::new("s1");
        listener.register(Box::new(source));
        assert_eq!(listener.source_count(), 1);
    }

    #[tokio::test]
    async fn test_trigger_listener_callback() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let called = Arc::new(AtomicBool::new(false));

        let (source, tx) = ChannelTriggerSource::new("cb-test");
        let mut listener = TriggerListener::new();
        listener.register(Box::new(source));

        let called_clone = called.clone();
        let _handles = listener.start(Arc::new(move |_event| {
            called_clone.store(true, Ordering::SeqCst);
        }));

        tx.send(TriggerEvent {
            session_id: None,
            config_override: None,
            payload: serde_json::json!({}),
        })
        .await
        .unwrap();

        // Give the spawned task a moment to process
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(called.load(Ordering::SeqCst));
    }
}
