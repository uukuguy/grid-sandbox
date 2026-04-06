use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde_json::{json, Value};

use crate::manager::McpManager;

/// Build the Axum router with all MCP orchestrator endpoints.
pub fn router(mgr: Arc<McpManager>) -> Router {
    Router::new()
        .route("/mcp-servers", get(list_servers))
        .route("/mcp-servers/{name}/start", post(start_server))
        .route("/mcp-servers/{name}/stop", post(stop_server))
        .route("/mcp-servers/{name}/info", get(server_info))
        .route("/health", get(health))
        .with_state(mgr)
}

async fn list_servers(State(mgr): State<Arc<McpManager>>) -> Json<Value> {
    let servers = mgr.list_servers().await;
    Json(json!({ "servers": servers }))
}

async fn start_server(
    State(mgr): State<Arc<McpManager>>,
    Path(name): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    mgr.start(&name).await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
    })?;
    Ok(Json(json!({ "status": "started", "name": name })))
}

async fn stop_server(
    State(mgr): State<Arc<McpManager>>,
    Path(name): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    mgr.stop(&name).await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
    })?;
    Ok(Json(json!({ "status": "stopped", "name": name })))
}

async fn server_info(
    State(mgr): State<Arc<McpManager>>,
    Path(name): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let info = mgr.get_info(&name).await.ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(serde_json::to_value(info).unwrap()))
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}
