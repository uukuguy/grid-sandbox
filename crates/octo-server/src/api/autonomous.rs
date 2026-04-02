//! AR-T5: Webhook trigger endpoint for autonomous agent sessions.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct TriggerRequest {
    /// Optional existing session to trigger autonomous mode on.
    #[serde(default)]
    pub session_id: Option<String>,
    /// Override for max autonomous rounds.
    #[serde(default)]
    pub max_rounds: Option<u32>,
    /// Arbitrary payload passed to the agent.
    #[serde(default)]
    pub payload: serde_json::Value,
}

/// POST /api/v1/autonomous/trigger — Webhook endpoint that triggers autonomous mode.
pub async fn trigger_autonomous(
    State(_state): State<Arc<AppState>>,
    Json(body): Json<TriggerRequest>,
) -> impl IntoResponse {
    // Validate request
    let session_id = body.session_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let max_rounds = body.max_rounds.unwrap_or(5);

    // TODO: Wire into AgentRuntime.start_autonomous() once the runtime
    // method is implemented. For now, return a placeholder response.
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "session_id": session_id,
            "status": "accepted",
            "max_rounds": max_rounds,
            "payload_size": body.payload.to_string().len(),
        })),
    )
}
