/// D125 — TelemetryBus backpressure counter tests.
use std::sync::Arc;

use grid_engine::event::bus::{TelemetryBus, TelemetryEvent};
use grid_engine::metrics::MetricsRegistry;

fn make_event(turn: u32) -> TelemetryEvent {
    TelemetryEvent::LoopTurnStarted {
        session_id: "test-session".to_string(),
        turn,
    }
}

/// When the channel is not full, no backpressure counter increment occurs.
#[tokio::test]
async fn no_backpressure_below_capacity() {
    let metrics = Arc::new(MetricsRegistry::new());
    let bus = TelemetryBus::new(4, 100, metrics.clone());

    // Subscribe so messages are queued (not immediately dropped).
    let _rx = bus.subscribe();

    // Publish 3 events into a capacity-4 channel → should NOT trigger counter.
    for i in 0..3 {
        bus.publish(make_event(i)).await;
    }

    let count = metrics.counter("events_stream_backpressure_total").get();
    assert_eq!(count, 0u64, "no backpressure expected below capacity");
}

/// When queued messages reach channel capacity, the counter is incremented.
#[tokio::test]
async fn backpressure_counter_increments_at_capacity() {
    let capacity = 4usize;
    let metrics = Arc::new(MetricsRegistry::new());
    let bus = TelemetryBus::new(capacity, 100, metrics.clone());

    // Subscribe but do NOT consume — forces messages to queue.
    let _rx = bus.subscribe();

    // Fill the channel exactly to capacity.
    for i in 0..capacity {
        bus.publish(make_event(i as u32)).await;
    }

    // One more publish when len >= capacity should bump the counter.
    bus.publish(make_event(99)).await;

    let count = metrics.counter("events_stream_backpressure_total").get();
    assert!(count >= 1u64, "backpressure counter must be >= 1 when channel full");
}

/// Multiple overflows accumulate in the counter.
#[tokio::test]
async fn backpressure_counter_accumulates() {
    let capacity = 2usize;
    let metrics = Arc::new(MetricsRegistry::new());
    let bus = TelemetryBus::new(capacity, 100, metrics.clone());

    let _rx = bus.subscribe();

    // Fill to capacity.
    for i in 0..capacity {
        bus.publish(make_event(i as u32)).await;
    }

    // Publish several more — each should increment the counter.
    let overflow_count = 3u32;
    for i in 0..overflow_count {
        bus.publish(make_event(100 + i)).await;
    }

    let count = metrics.counter("events_stream_backpressure_total").get();
    assert!(
        count >= overflow_count as u64,
        "expected >= {overflow_count} backpressure increments, got {count}"
    );
}
