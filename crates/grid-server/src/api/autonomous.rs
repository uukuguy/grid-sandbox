//! AR-T5 + AU-G5 + AU-D1: Webhook trigger endpoint for autonomous agent sessions.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use grid_engine::agent::AutonomousConfig;
use grid_types::{SandboxId, SessionId, UserId};
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
    /// Override for idle sleep duration in seconds.
    #[serde(default)]
    pub idle_sleep_secs: Option<u64>,
    /// Initial prompt to send to the agent.
    #[serde(default)]
    pub prompt: Option<String>,
    /// Arbitrary payload passed to the agent.
    #[serde(default)]
    pub payload: serde_json::Value,
}

/// POST /api/v1/autonomous/trigger — Webhook endpoint that triggers autonomous mode.
///
/// Creates a new session with autonomous config, registers with AutonomousScheduler,
/// and optionally sends an initial prompt. Returns session details for monitoring.
pub async fn trigger_autonomous(
    State(state): State<Arc<AppState>>,
    Json(body): Json<TriggerRequest>,
) -> impl IntoResponse {
    let session_id_str = body
        .session_id
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let session_id = SessionId::from_string(&session_id_str);
    let user_id = UserId::from_string("webhook");
    let sandbox_id = SandboxId::new();

    let config = AutonomousConfig {
        enabled: true,
        max_autonomous_rounds: body.max_rounds.unwrap_or(100),
        idle_sleep_secs: body.idle_sleep_secs.unwrap_or(30),
        ..Default::default()
    };

    // AU-D1: Start session with autonomous config via runtime
    let initial_history = if let Some(ref prompt) = body.prompt {
        vec![grid_types::ChatMessage::user(prompt)]
    } else {
        vec![]
    };

    match state
        .agent_supervisor
        .start_session_with_autonomous(
            session_id.clone(),
            user_id,
            sandbox_id,
            initial_history,
            None,
            Some(config.clone()),
        )
        .await
    {
        Ok(_handle) => (
            StatusCode::CREATED,
            Json(serde_json::json!({
                "session_id": session_id_str,
                "status": "started",
                "autonomous": {
                    "enabled": true,
                    "max_rounds": config.max_autonomous_rounds,
                    "idle_sleep_secs": config.idle_sleep_secs,
                },
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "session_id": session_id_str,
                "status": "failed",
                "error": e.to_string(),
            })),
        ),
    }
}
