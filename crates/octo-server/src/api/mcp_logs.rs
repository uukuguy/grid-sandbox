use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get},
    Json, Router,
};
use serde::Deserialize;

use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct LogQueryParams {
    pub level: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, serde::Serialize)]
pub struct McpLogEntry {
    pub id: String,
    pub server_id: String,
    pub level: String,
    pub direction: String,
    pub method: Option<String>,
    pub params: Option<serde_json::Value>,
    pub result: Option<serde_json::Value>,
    pub raw_data: Option<String>,
    pub duration_ms: Option<i64>,
    pub logged_at: String,
}

// List logs
pub async fn list_logs(
    State(_state): State<Arc<AppState>>,
    Path(server_id): Path<String>,
    Query(params): Query<LogQueryParams>,
) -> Json<Vec<McpLogEntry>> {
    let limit = params.limit.unwrap_or(100);
    let offset = params.offset.unwrap_or(0);

    // TODO: Get from storage
    Json(vec![])
}

// Clear logs
pub async fn clear_logs(
    State(_state): State<Arc<AppState>>,
    Path(server_id): Path<String>,
) -> Json<serde_json::Value> {
    // TODO: Clear from storage
    Json(serde_json::json!({"cleared": server_id}))
}

// Export logs
pub async fn export_logs(
    State(_state): State<Arc<AppState>>,
    Path(server_id): Path<String>,
) -> Json<serde_json::Value> {
    // TODO: Export to JSON
    Json(serde_json::json!({"exported": server_id, "format": "json"}))
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/mcp/servers/{server_id}/logs", get(list_logs))
        .route("/mcp/servers/{server_id}/logs", delete(clear_logs))
        .route("/mcp/servers/{server_id}/logs/export", get(export_logs))
}
