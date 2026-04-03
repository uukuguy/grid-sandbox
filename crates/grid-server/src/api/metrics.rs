use std::fmt::Write;
use std::sync::Arc;

use axum::{
    extract::State,
    http::{header, HeaderValue},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
pub struct MetricsSnapshot {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub counters: Vec<CounterMetric>,
    pub gauges: Vec<GaugeMetric>,
    pub histograms: Vec<HistogramMetric>,
}

#[derive(Serialize)]
pub struct CounterMetric {
    pub name: String,
    pub value: u64,
}

#[derive(Serialize)]
pub struct GaugeMetric {
    pub name: String,
    pub value: i64,
}

#[derive(Serialize)]
pub struct HistogramMetric {
    pub name: String,
    pub count: u64,
    pub sum: f64,
    pub buckets: Vec<Bucket>,
}

#[derive(Serialize)]
pub struct Bucket {
    pub le: f64,
    pub count: u64,
}

pub async fn get_metrics(State(state): State<Arc<AppState>>) -> Json<MetricsSnapshot> {
    let registry = state.metrics_registry.read().await;

    let counters = registry
        .counters()
        .iter()
        .map(|e| CounterMetric {
            name: e.key().clone(),
            value: e.value().get(),
        })
        .collect();

    let gauges = registry
        .gauges()
        .iter()
        .map(|e| GaugeMetric {
            name: e.key().clone(),
            value: e.value().get(),
        })
        .collect();

    let histograms = registry
        .histograms()
        .iter()
        .map(|e| {
            let snapshot = e.value().snapshot();
            HistogramMetric {
                name: e.key().clone(),
                count: snapshot.count,
                sum: snapshot.sum,
                buckets: snapshot
                    .buckets
                    .iter()
                    .map(|b| Bucket {
                        le: b.le,
                        count: b.cumulative_count,
                    })
                    .collect(),
            }
        })
        .collect();

    Json(MetricsSnapshot {
        timestamp: chrono::Utc::now(),
        counters,
        gauges,
        histograms,
    })
}

/// Well-known core metrics with HELP/TYPE annotations for Prometheus.
const CORE_METRICS: &[(&str, &str, &str)] = &[
    (
        "grid_active_sessions",
        "gauge",
        "Number of currently active sessions",
    ),
    (
        "grid_request_duration_seconds",
        "histogram",
        "Request duration in seconds",
    ),
    (
        "grid_tool_invocations_total",
        "counter",
        "Total tool invocations",
    ),
    (
        "grid_llm_tokens_used_total",
        "counter",
        "Total LLM tokens used",
    ),
    (
        "grid_ws_connections_active",
        "gauge",
        "Number of active WebSocket connections",
    ),
    (
        "grid_max_concurrent_sessions",
        "gauge",
        "Maximum allowed concurrent sessions",
    ),
];

/// Sanitize a metric name for Prometheus (only [a-zA-Z0-9_:] allowed).
fn sanitize_metric_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == ':' { c } else { '_' })
        .collect()
}

/// Lookup HELP/TYPE for a core metric by name.
fn core_metric_info(name: &str) -> Option<(&'static str, &'static str)> {
    CORE_METRICS
        .iter()
        .find(|(n, _, _)| *n == name)
        .map(|(_, typ, help)| (*typ, *help))
}

/// Write HELP and TYPE comments for a metric if it is a known core metric.
fn write_help_type(buf: &mut String, name: &str, fallback_type: &str) {
    if let Some((typ, help)) = core_metric_info(name) {
        let _ = writeln!(buf, "# HELP {name} {help}");
        let _ = writeln!(buf, "# TYPE {name} {typ}");
    } else {
        let _ = writeln!(buf, "# TYPE {name} {fallback_type}");
    }
}

/// `GET /api/v1/metrics/prometheus` — Prometheus text exposition format.
pub async fn get_prometheus_metrics(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let registry = state.metrics_registry.read().await;
    let mut buf = String::with_capacity(4096);

    // ── Inject supervisor-derived gauges that may not exist in the registry ──

    let active_sessions = state.agent_supervisor.active_session_count();
    let max_sessions = state.agent_supervisor.max_concurrent_sessions();

    // octo_active_sessions
    {
        let name = "grid_active_sessions";
        write_help_type(&mut buf, name, "gauge");
        let _ = writeln!(buf, "{name} {active_sessions}");
    }

    // octo_max_concurrent_sessions
    {
        let name = "grid_max_concurrent_sessions";
        write_help_type(&mut buf, name, "gauge");
        let _ = writeln!(buf, "{name} {max_sessions}");
    }

    // ── Gauges from registry ──
    {
        let mut entries: Vec<_> = registry
            .gauges()
            .iter()
            .map(|e| (e.key().clone(), e.value().get()))
            .collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));

        for (raw_name, value) in entries {
            let name = sanitize_metric_name(&raw_name);
            // Skip if already emitted above
            if name == "grid_active_sessions" || name == "grid_max_concurrent_sessions" {
                continue;
            }
            write_help_type(&mut buf, &name, "gauge");
            let _ = writeln!(buf, "{name} {value}");
        }
    }

    // ── Counters from registry ──
    {
        let mut entries: Vec<_> = registry
            .counters()
            .iter()
            .map(|e| (e.key().clone(), e.value().get()))
            .collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));

        for (raw_name, value) in entries {
            let name = sanitize_metric_name(&raw_name);
            write_help_type(&mut buf, &name, "counter");
            let _ = writeln!(buf, "{name} {value}");
        }
    }

    // ── Histograms from registry ──
    {
        let mut entries: Vec<_> = registry
            .histograms()
            .iter()
            .map(|e| (e.key().clone(), e.value().snapshot()))
            .collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));

        for (raw_name, snapshot) in entries {
            let name = sanitize_metric_name(&raw_name);
            write_help_type(&mut buf, &name, "histogram");

            // Cumulative bucket lines
            for bucket in &snapshot.buckets {
                let le = bucket.le;
                let count = bucket.cumulative_count;
                let _ = writeln!(buf, "{name}_bucket{{le=\"{le}\"}} {count}");
            }
            // +Inf bucket = total count
            let _ = writeln!(buf, "{name}_bucket{{le=\"+Inf\"}} {}", snapshot.count);
            let _ = writeln!(buf, "{name}_sum {}", snapshot.sum);
            let _ = writeln!(buf, "{name}_count {}", snapshot.count);
        }
    }

    let content_type =
        HeaderValue::from_static("text/plain; version=0.0.4; charset=utf-8");

    ([(header::CONTENT_TYPE, content_type)], buf)
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/metrics", get(get_metrics))
        .route("/metrics/prometheus", get(get_prometheus_metrics))
}
