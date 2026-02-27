use std::sync::Arc;

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct McpToolInfo {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpToolCallRequest {
    pub tool_name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpToolCallResponse {
    pub id: String,
    pub server_id: String,
    pub tool_name: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub duration_ms: i64,
    pub executed_at: String,
}

// List tools for a server
pub async fn list_tools(
    State(_state): State<Arc<AppState>>,
    Path(server_id): Path<String>,
) -> Json<Vec<McpToolInfo>> {
    // TODO: Get from McpManager
    Json(vec![])
}

// Call a tool
pub async fn call_tool(
    State(_state): State<Arc<AppState>>,
    Path(server_id): Path<String>,
    Json(req): Json<McpToolCallRequest>,
) -> Json<McpToolCallResponse> {
    let now = chrono::Utc::now();

    Json(McpToolCallResponse {
        id: uuid::Uuid::new_v4().to_string(),
        server_id,
        tool_name: req.tool_name,
        result: None,
        error: Some("Not implemented".to_string()),
        duration_ms: 0,
        executed_at: now.to_rfc3339(),
    })
}

// List execution history
pub async fn list_executions(
    State(_state): State<Arc<AppState>>,
    Path(server_id): Path<String>,
) -> Json<Vec<McpToolCallResponse>> {
    // TODO: Get from storage
    Json(vec![])
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/mcp/servers/{server_id}/tools", get(list_tools))
        .route("/mcp/servers/{server_id}/call", post(call_tool))
        .route("/mcp/servers/{server_id}/executions", get(list_executions))
}
