use std::sync::Arc;

use axum::{
    extract::{Path, State},
    routing::{delete, get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use octo_engine::providers::{AttemptResult, LlmInstance};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

/// List response
#[derive(Serialize)]
pub struct ListProvidersResponse {
    pub policy: String,
    pub current_instance_id: Option<String>,
    pub instances: Vec<ProviderInstance>,
}

#[derive(Serialize)]
pub struct ProviderInstance {
    pub id: String,
    pub provider: String,
    pub model: String,
    pub priority: u8,
    pub health: String,
    pub enabled: bool,
}

/// Add instance request
#[derive(Deserialize)]
pub struct AddProviderRequest {
    pub id: String,
    pub provider: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub model: String,
    pub priority: u8,
    pub max_rpm: Option<u32>,
    pub enabled: Option<bool>,
}

/// List all instances
pub async fn list_providers(State(state): State<Arc<AppState>>) -> Json<ListProvidersResponse> {
    let chain = state.agent_supervisor.provider_chain();

    let policy = match chain {
        Some(c) => format!("{:?}", c.policy()),
        None => "none".to_string(),
    };

    let instances = match chain {
        Some(c) => {
            let instance_list = c.list_instances().await;
            let mut result = Vec::with_capacity(instance_list.len());
            for i in instance_list {
                let health = c.get_health(&i.id).await;
                result.push(ProviderInstance {
                    id: i.id,
                    provider: i.provider,
                    model: i.model,
                    priority: i.priority,
                    health: format!("{:?}", health),
                    enabled: i.enabled,
                });
            }
            result
        }
        None => vec![],
    };

    let current = match chain {
        Some(c) => c.get_current_selection().await,
        None => None,
    };

    Json(ListProvidersResponse {
        policy,
        current_instance_id: current,
        instances,
    })
}

/// Manually select an instance
pub async fn select_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<()>, String> {
    let chain = state.agent_supervisor.provider_chain();

    match chain {
        Some(c) => c.select_instance(&id).await.map_err(|e| e.to_string())?,
        None => return Err("Provider chain not configured".to_string()),
    };

    Ok(Json(()))
}

/// Reset instance health status
pub async fn reset_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<()>, String> {
    let chain = state.agent_supervisor.provider_chain();

    match chain {
        Some(c) => c.reset_health(&id).await.map_err(|e| e.to_string())?,
        None => return Err("Provider chain not configured".to_string()),
    };

    Ok(Json(()))
}

/// Clear selection
pub async fn clear_selection(State(state): State<Arc<AppState>>) -> Result<Json<()>, String> {
    let chain = state.agent_supervisor.provider_chain();

    match chain {
        Some(c) => c.clear_selection().await,
        None => return Err("Provider chain not configured".to_string()),
    };

    Ok(Json(()))
}

/// Add an instance
pub async fn add_provider(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AddProviderRequest>,
) -> Result<Json<()>, String> {
    let chain = state.agent_supervisor.provider_chain();

    let instance = LlmInstance {
        id: req.id,
        provider: req.provider,
        api_key: req.api_key,
        base_url: req.base_url,
        model: req.model,
        priority: req.priority,
        max_rpm: req.max_rpm,
        enabled: req.enabled.unwrap_or(true),
    };

    match chain {
        Some(c) => c.add_instance(instance).await,
        None => return Err("Provider chain not configured".to_string()),
    };

    Ok(Json(()))
}

/// Delete an instance
pub async fn delete_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<()>, String> {
    let chain = state.agent_supervisor.provider_chain();

    match chain {
        Some(c) => c.remove_instance(&id).await.map_err(|e| e.to_string())?,
        None => return Err("Provider chain not configured".to_string()),
    };

    Ok(Json(()))
}

// ── Provider status (rich monitoring) ──────────────────────────────────

#[derive(Serialize)]
pub struct ProviderStatusResponse {
    pub policy: String,
    pub current_instance_id: Option<String>,
    pub instances: Vec<ProviderInstanceStatus>,
    pub total_failovers: u64,
    pub recent_traces: Vec<TraceEntry>,
}

#[derive(Serialize)]
pub struct ProviderInstanceStatus {
    pub id: String,
    pub provider: String,
    pub model: String,
    pub priority: u8,
    pub enabled: bool,
    pub health: String,
    pub latency_p50_ms: Option<u64>,
    pub latency_p99_ms: Option<u64>,
    pub request_count: u64,
    pub error_count: u64,
    pub failover_count: u64,
}

#[derive(Serialize)]
pub struct TraceEntry {
    pub timestamp: DateTime<Utc>,
    pub instance_id: String,
    pub success: bool,
    pub duration_ms: u64,
}

/// GET /providers/status — detailed provider health and performance data
pub async fn get_provider_status(
    State(state): State<Arc<AppState>>,
) -> Json<ProviderStatusResponse> {
    let chain = state.agent_supervisor.provider_chain();

    let Some(chain) = chain else {
        // Single-provider mode — return minimal response
        return Json(ProviderStatusResponse {
            policy: "none".to_string(),
            current_instance_id: None,
            instances: vec![],
            total_failovers: 0,
            recent_traces: vec![],
        });
    };

    let policy = format!("{:?}", chain.policy());
    let current_instance_id = chain.get_current_selection().await;
    let instance_list = chain.list_instances().await;
    let all_health = chain.get_all_health().await;
    let stats_map = chain.instance_stats().await;
    let traces = chain.recent_traces(20).await;

    // Build per-instance status
    let instances: Vec<ProviderInstanceStatus> = instance_list
        .iter()
        .map(|inst| {
            let health = all_health
                .get(&inst.id)
                .map(|h| format!("{:?}", h))
                .unwrap_or_else(|| "Unknown".to_string());
            let stats = stats_map.get(&inst.id);
            ProviderInstanceStatus {
                id: inst.id.clone(),
                provider: inst.provider.clone(),
                model: inst.model.clone(),
                priority: inst.priority,
                enabled: inst.enabled,
                health,
                latency_p50_ms: stats.and_then(|s| s.latency_p50_ms),
                latency_p99_ms: stats.and_then(|s| s.latency_p99_ms),
                request_count: stats.map_or(0, |s| s.request_count),
                error_count: stats.map_or(0, |s| s.error_count),
                failover_count: stats.map_or(0, |s| s.failover_count),
            }
        })
        .collect();

    // Compute total failovers (traces with >1 attempt)
    let total_failovers = traces.iter().filter(|t| t.attempts.len() > 1).count() as u64;

    // Flatten recent traces into per-attempt entries for debugging
    let recent_traces: Vec<TraceEntry> = traces
        .iter()
        .flat_map(|t| {
            t.attempts.iter().filter(|a| a.instance_id != "none").map(|a| TraceEntry {
                timestamp: t.started_at,
                instance_id: a.instance_id.clone(),
                success: matches!(a.result, AttemptResult::Success),
                duration_ms: a.duration_ms,
            })
        })
        .take(20)
        .collect();

    Json(ProviderStatusResponse {
        policy,
        current_instance_id,
        instances,
        total_failovers,
        recent_traces,
    })
}

/// Register routes
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/providers", get(list_providers))
        .route("/providers", post(add_provider))
        .route("/providers/status", get(get_provider_status))
        .route("/providers/{id}", delete(delete_provider))
        .route("/providers/{id}/select", post(select_provider))
        .route("/providers/{id}/reset", post(reset_provider))
        .route("/providers/selection", delete(clear_selection))
}
