use std::sync::Arc;

use axum::{
    extract::{Path, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use chrono::Utc;

use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct McpServerConfigRequest {
    pub name: String,
    pub source: Option<String>,
    pub command: String,
    pub args: Vec<String>,
    pub env: Option<std::collections::HashMap<String, String>>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpServerResponse {
    pub id: String,
    pub name: String,
    pub source: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: std::collections::HashMap<String, String>,
    pub enabled: bool,
    pub runtime_status: String,
    pub tool_count: usize,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpServerStatusResponse {
    pub id: String,
    pub name: String,
    pub status: String,
    pub pid: Option<u32>,
    pub error: Option<String>,
    pub tool_count: usize,
}

// List all MCP servers
pub async fn list_servers(
    State(_state): State<Arc<AppState>>,
) -> Json<Vec<McpServerResponse>> {
    // TODO: Implement with storage
    Json(vec![])
}

// Get single server
pub async fn get_server(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<Option<McpServerResponse>> {
    // TODO: Implement with storage
    Json(None)
}

// Create new server
pub async fn create_server(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<McpServerConfigRequest>,
) -> Json<McpServerResponse> {
    let now = Utc::now().to_rfc3339();
    let id = uuid::Uuid::new_v4().to_string();

    Json(McpServerResponse {
        id,
        name: req.name,
        source: req.source.unwrap_or_else(|| "manual".to_string()),
        command: req.command,
        args: req.args,
        env: req.env.unwrap_or_default(),
        enabled: req.enabled.unwrap_or(true),
        runtime_status: "stopped".to_string(),
        tool_count: 0,
        created_at: now.clone(),
        updated_at: now,
    })
}

// Update server
pub async fn update_server(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<McpServerConfigRequest>,
) -> Json<Option<McpServerResponse>> {
    // TODO: Implement with storage
    Json(None)
}

// Delete server
pub async fn delete_server(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    // TODO: Implement with storage
    Json(serde_json::json!({"deleted": id}))
}

// Start server
pub async fn start_server(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    // TODO: Implement with McpManager
    Json(serde_json::json!({"started": id}))
}

// Stop server
pub async fn stop_server(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    // TODO: Implement with McpManager
    Json(serde_json::json!({"stopped": id}))
}

// Get server status
pub async fn get_server_status(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<Option<McpServerStatusResponse>> {
    // TODO: Implement with McpManager
    Json(None)
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/mcp/servers", get(list_servers))
        .route("/mcp/servers", post(create_server))
        .route("/mcp/servers/{id}", get(get_server))
        .route("/mcp/servers/{id}", put(update_server))
        .route("/mcp/servers/{id}", delete(delete_server))
        .route("/mcp/servers/{id}/start", post(start_server))
        .route("/mcp/servers/{id}/stop", post(stop_server))
        .route("/mcp/servers/{id}/status", get(get_server_status))
}
