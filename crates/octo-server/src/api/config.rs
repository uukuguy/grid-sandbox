use axum::{extract::State, Json};
use serde::Serialize;
use std::sync::Arc;

use crate::state::AppState;

/// Frontend configuration - subset of server config needed by frontend
#[derive(Serialize)]
pub struct FrontendConfig {
    /// Server host
    pub host: String,
    /// Server port
    pub port: u16,
    /// Backend API base URL
    pub api_url: String,
    /// WebSocket URL for real-time communication
    pub ws_url: String,
    /// MCP servers directory (if configured)
    pub mcp_servers_dir: Option<String>,
    /// Provider name (e.g., "anthropic", "openai")
    pub provider: String,
    /// Model being used (if set)
    pub model: Option<String>,
}

/// Get frontend configuration
/// This endpoint provides the frontend with runtime configuration from config.yaml
pub async fn get_config(State(state): State<Arc<AppState>>) -> Json<FrontendConfig> {
    let host = state.config.server.host.clone();
    let port = state.config.server.port;

    // Construct URLs based on server config
    let api_url = format!("http://{}:{}", host, port);
    let ws_url = format!("ws://{}:{}", host, port);

    Json(FrontendConfig {
        host,
        port,
        api_url,
        ws_url,
        mcp_servers_dir: state.config.mcp.servers_dir.clone(),
        provider: state.config.provider.name.clone(),
        model: state.config.provider.model.clone(),
    })
}
