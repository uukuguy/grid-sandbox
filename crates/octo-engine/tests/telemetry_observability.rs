//! Tests for T3: TelemetryBus observability publish points in harness.
//!
//! These tests verify that the TelemetryBus events (ContextDegraded,
//! TokenBudgetUpdated, LoopGuardTriggered) are correctly defined and
//! publishable through the TelemetryBus infrastructure.

use std::sync::Arc;

use octo_engine::event::TelemetryBus;
use octo_engine::metrics::MetricsRegistry;

#[tokio::test]
async fn telemetry_bus_publishes_context_degraded() {
    let metrics = Arc::new(MetricsRegistry::new());
    let bus = TelemetryBus::new(100, 100, metrics);

    bus.publish(octo_engine::event::TelemetryEvent::ContextDegraded {
        session_id: "test-session".to_string(),
        level: "SoftTrim".to_string(),
    })
    .await;

    let events = bus.recent_events(10).await;
    assert_eq!(events.len(), 1);
    match &events[0] {
        octo_engine::event::TelemetryEvent::ContextDegraded { session_id, level } => {
            assert_eq!(session_id, "test-session");
            assert_eq!(level, "SoftTrim");
        }
        other => panic!("Expected ContextDegraded, got {:?}", other),
    }
}

#[tokio::test]
async fn telemetry_bus_publishes_token_budget_updated() {
    let metrics = Arc::new(MetricsRegistry::new());
    let bus = TelemetryBus::new(100, 100, metrics);

    bus.publish(octo_engine::event::TelemetryEvent::TokenBudgetUpdated {
        session_id: "test-session".to_string(),
        used: 50_000,
        total: 200_000,
        ratio: 0.25,
    })
    .await;

    let events = bus.recent_events(10).await;
    assert_eq!(events.len(), 1);
    match &events[0] {
        octo_engine::event::TelemetryEvent::TokenBudgetUpdated {
            session_id,
            used,
            total,
            ratio,
        } => {
            assert_eq!(session_id, "test-session");
            assert_eq!(*used, 50_000);
            assert_eq!(*total, 200_000);
            assert!((ratio - 0.25).abs() < f64::EPSILON);
        }
        other => panic!("Expected TokenBudgetUpdated, got {:?}", other),
    }
}

#[tokio::test]
async fn telemetry_bus_publishes_loop_guard_triggered() {
    let metrics = Arc::new(MetricsRegistry::new());
    let bus = TelemetryBus::new(100, 100, metrics);

    bus.publish(octo_engine::event::TelemetryEvent::LoopGuardTriggered {
        session_id: "test-session".to_string(),
        reason: "Repetitive tool calls detected".to_string(),
    })
    .await;

    let events = bus.recent_events(10).await;
    assert_eq!(events.len(), 1);
    match &events[0] {
        octo_engine::event::TelemetryEvent::LoopGuardTriggered { session_id, reason } => {
            assert_eq!(session_id, "test-session");
            assert_eq!(reason, "Repetitive tool calls detected");
        }
        other => panic!("Expected LoopGuardTriggered, got {:?}", other),
    }
}

#[tokio::test]
async fn telemetry_bus_subscriber_receives_all_event_types() {
    let metrics = Arc::new(MetricsRegistry::new());
    let bus = TelemetryBus::new(100, 100, metrics);
    let mut rx = bus.subscribe();

    // Publish all three new event types
    bus.publish(octo_engine::event::TelemetryEvent::ContextDegraded {
        session_id: "s1".to_string(),
        level: "AutoCompaction".to_string(),
    })
    .await;

    bus.publish(octo_engine::event::TelemetryEvent::TokenBudgetUpdated {
        session_id: "s1".to_string(),
        used: 100_000,
        total: 200_000,
        ratio: 0.5,
    })
    .await;

    bus.publish(octo_engine::event::TelemetryEvent::LoopGuardTriggered {
        session_id: "s1".to_string(),
        reason: "circuit break".to_string(),
    })
    .await;

    // Receive all three
    let e1 = rx.recv().await.unwrap();
    assert!(matches!(
        e1,
        octo_engine::event::TelemetryEvent::ContextDegraded { .. }
    ));

    let e2 = rx.recv().await.unwrap();
    assert!(matches!(
        e2,
        octo_engine::event::TelemetryEvent::TokenBudgetUpdated { .. }
    ));

    let e3 = rx.recv().await.unwrap();
    assert!(matches!(
        e3,
        octo_engine::event::TelemetryEvent::LoopGuardTriggered { .. }
    ));
}

#[tokio::test]
async fn telemetry_metrics_recorded_for_new_events() {
    let metrics = Arc::new(MetricsRegistry::new());
    let bus = TelemetryBus::new(100, 100, metrics.clone());

    bus.publish(octo_engine::event::TelemetryEvent::ContextDegraded {
        session_id: "s".to_string(),
        level: "SoftTrim".to_string(),
    })
    .await;

    bus.publish(octo_engine::event::TelemetryEvent::LoopGuardTriggered {
        session_id: "s".to_string(),
        reason: "test".to_string(),
    })
    .await;

    bus.publish(octo_engine::event::TelemetryEvent::TokenBudgetUpdated {
        session_id: "s".to_string(),
        used: 1000,
        total: 10000,
        ratio: 0.1,
    })
    .await;

    // Verify metrics counters were incremented
    let degradation_count = metrics.counter("octo.context.degradations.total").get();
    assert_eq!(degradation_count, 1);

    let guard_count = metrics
        .counter("octo.sessions.guards.triggered.total")
        .get();
    assert_eq!(guard_count, 1);

    // Token budget updates gauge values
    let used_gauge = metrics.gauge("octo.context.tokens.used").get();
    assert_eq!(used_gauge, 1000);

    let total_gauge = metrics.gauge("octo.context.tokens.total").get();
    assert_eq!(total_gauge, 10000);
}
