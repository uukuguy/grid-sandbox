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

use grid_types::SessionId;

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
// AU-D3: Redis Streams trigger source (feature-gated)
// ---------------------------------------------------------------------------

/// Redis Streams-based trigger source for enterprise MQ integration.
///
/// Reads trigger events from a Redis Stream using XREAD with consumer groups.
/// Feature-gated behind `trigger-redis` to avoid mandatory redis dependency.
///
/// Usage:
/// ```ignore
/// let source = RedisStreamTriggerSource::new(
///     "redis://localhost:6379",
///     "grid:autonomous:triggers",
///     "grid-consumer-group",
///     "consumer-1",
/// ).await?;
/// listener.register(Box::new(source));
/// ```
#[cfg(feature = "trigger-redis")]
pub struct RedisStreamTriggerSource {
    name: String,
    client: redis::Client,
    stream_key: String,
    group: String,
    consumer: String,
}

#[cfg(feature = "trigger-redis")]
impl RedisStreamTriggerSource {
    pub async fn new(
        redis_url: &str,
        stream_key: &str,
        group: &str,
        consumer: &str,
    ) -> anyhow::Result<Self> {
        let client = redis::Client::open(redis_url)?;

        // Ensure consumer group exists (XGROUP CREATE, ignore if already exists)
        let mut conn = client.get_multiplexed_async_connection().await?;
        let _: Result<(), _> = redis::cmd("XGROUP")
            .arg("CREATE")
            .arg(stream_key)
            .arg(group)
            .arg("0")
            .arg("MKSTREAM")
            .query_async(&mut conn)
            .await;

        Ok(Self {
            name: format!("redis-stream:{}", stream_key),
            client,
            stream_key: stream_key.to_string(),
            group: group.to_string(),
            consumer: consumer.to_string(),
        })
    }
}

#[cfg(feature = "trigger-redis")]
#[async_trait]
impl TriggerSource for RedisStreamTriggerSource {
    async fn next_trigger(&mut self) -> anyhow::Result<TriggerEvent> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;

        loop {
            // XREADGROUP GROUP <group> <consumer> BLOCK 5000 COUNT 1 STREAMS <key> >
            let result: redis::RedisResult<redis::Value> = redis::cmd("XREADGROUP")
                .arg("GROUP")
                .arg(&self.group)
                .arg(&self.consumer)
                .arg("BLOCK")
                .arg(5000_u64) // 5s block timeout
                .arg("COUNT")
                .arg(1_u64)
                .arg("STREAMS")
                .arg(&self.stream_key)
                .arg(">")
                .query_async(&mut conn)
                .await;

            match result {
                Ok(redis::Value::Array(streams)) if !streams.is_empty() => {
                    // Parse the Redis Stream entry into a TriggerEvent
                    if let Some(payload_str) = extract_stream_payload(&streams) {
                        let event: TriggerEvent = serde_json::from_str(&payload_str)
                            .unwrap_or(TriggerEvent {
                                session_id: None,
                                config_override: None,
                                payload: serde_json::json!({ "raw": payload_str }),
                            });
                        return Ok(event);
                    }
                }
                Ok(redis::Value::Nil) | Ok(_) => {
                    // Timeout or empty — continue polling
                    continue;
                }
                Err(e) => {
                    warn!(error = %e, "Redis XREADGROUP failed, retrying...");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Extract the payload field from a Redis Stream XREADGROUP response.
#[cfg(feature = "trigger-redis")]
fn extract_stream_payload(streams: &[redis::Value]) -> Option<String> {
    // Response structure: [[stream_name, [[entry_id, [field, value, ...]]]]]
    use redis::Value;
    if let Some(Value::Array(stream_data)) = streams.first() {
        if let Some(Value::Array(entries)) = stream_data.get(1) {
            if let Some(Value::Array(entry)) = entries.first() {
                if let Some(Value::Array(fields)) = entry.get(1) {
                    // Find "payload" field
                    for chunk in fields.chunks(2) {
                        if let [Value::BulkString(key), Value::BulkString(val)] = chunk {
                            if key == b"payload" {
                                return String::from_utf8(val.clone()).ok();
                            }
                        }
                    }
                    // Fallback: use first value
                    if let Some(Value::BulkString(val)) = fields.get(1) {
                        return String::from_utf8(val.clone()).ok();
                    }
                }
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// AU-D2: Cron-based trigger source
// ---------------------------------------------------------------------------

/// Cron-based trigger source that fires at specified intervals.
///
/// Uses a simple interval approach rather than full cron parsing to avoid
/// adding a cron-expression-parser dependency. For full cron support,
/// wrap the existing `scheduler/` module's CronScheduler.
pub struct CronTriggerSource {
    name: String,
    interval: Duration,
    config: AutonomousConfig,
}

impl CronTriggerSource {
    /// Create a cron trigger that fires at the given interval with the specified config.
    pub fn new(name: &str, interval: Duration, config: AutonomousConfig) -> Self {
        Self {
            name: name.to_string(),
            interval,
            config,
        }
    }

    /// Create from a cron-like interval string (e.g., "5m", "1h", "30s").
    pub fn from_interval_str(name: &str, interval_str: &str, config: AutonomousConfig) -> anyhow::Result<Self> {
        let interval = parse_duration_str(interval_str)?;
        Ok(Self::new(name, interval, config))
    }
}

/// Parse a simple duration string: "30s", "5m", "1h", "2h30m".
fn parse_duration_str(s: &str) -> anyhow::Result<Duration> {
    let s = s.trim().to_lowercase();
    let mut total_secs: u64 = 0;
    let mut num_buf = String::new();

    for ch in s.chars() {
        if ch.is_ascii_digit() {
            num_buf.push(ch);
        } else {
            let n: u64 = num_buf.parse().unwrap_or(0);
            num_buf.clear();
            match ch {
                's' => total_secs += n,
                'm' => total_secs += n * 60,
                'h' => total_secs += n * 3600,
                'd' => total_secs += n * 86400,
                _ => {}
            }
        }
    }
    // Handle bare number (treat as seconds)
    if !num_buf.is_empty() {
        total_secs += num_buf.parse::<u64>().unwrap_or(0);
    }

    if total_secs == 0 {
        anyhow::bail!("Invalid duration: {}", s);
    }
    Ok(Duration::from_secs(total_secs))
}

#[async_trait]
impl TriggerSource for CronTriggerSource {
    async fn next_trigger(&mut self) -> anyhow::Result<TriggerEvent> {
        tokio::time::sleep(self.interval).await;
        info!(name = %self.name, interval_secs = self.interval.as_secs(), "CronTrigger: firing");
        Ok(TriggerEvent {
            session_id: None, // create new session each time
            config_override: Some(self.config.clone()),
            payload: serde_json::json!({ "trigger": "cron", "source": self.name }),
        })
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

    #[test]
    fn test_parse_duration_str() {
        assert_eq!(super::parse_duration_str("30s").unwrap(), Duration::from_secs(30));
        assert_eq!(super::parse_duration_str("5m").unwrap(), Duration::from_secs(300));
        assert_eq!(super::parse_duration_str("1h").unwrap(), Duration::from_secs(3600));
        assert_eq!(super::parse_duration_str("2h30m").unwrap(), Duration::from_secs(9000));
        assert_eq!(super::parse_duration_str("1d").unwrap(), Duration::from_secs(86400));
        assert!(super::parse_duration_str("").is_err());
        assert!(super::parse_duration_str("abc").is_err());
    }

    #[tokio::test]
    async fn test_cron_trigger_fires() {
        let config = super::super::autonomous::AutonomousConfig {
            enabled: true,
            max_autonomous_rounds: 10,
            ..Default::default()
        };
        let mut source = super::CronTriggerSource::new("test-cron", Duration::from_millis(10), config);
        let event = source.next_trigger().await.unwrap();
        assert_eq!(event.payload["trigger"], "cron");
        assert!(event.config_override.is_some());
        assert!(event.session_id.is_none()); // creates new session each time
    }

    #[test]
    fn test_cron_trigger_from_interval_str() {
        let config = super::super::autonomous::AutonomousConfig::default();
        let source = super::CronTriggerSource::from_interval_str("every-5m", "5m", config).unwrap();
        assert_eq!(source.interval, Duration::from_secs(300));
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
