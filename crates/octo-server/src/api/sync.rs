//! REST API endpoints for the D6 offline sync protocol.
//!
//! Routes:
//! - `GET  /api/sync/status?device_id=xxx` — device sync status
//! - `POST /api/sync/pull`                 — pull remote changes
//! - `POST /api/sync/push`                 — push local changes

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;

use crate::state::AppState;

// ── Query / request types ───────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct DeviceQuery {
    pub device_id: String,
}

// ── Handlers ────────────────────────────────────────────────────────────

/// GET /api/sync/status?device_id=xxx
pub async fn sync_status(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DeviceQuery>,
) -> Result<Json<octo_engine::sync::SyncStatus>, StatusCode> {
    let conn = tokio_rusqlite::Connection::open(&state.db_path)
        .await
        .map_err(|e| {
            tracing::error!("Failed to open DB for sync status: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Ensure tables exist before querying.
    octo_engine::sync::server::standalone::ensure_tables(&conn)
        .await
        .map_err(|e| {
            tracing::error!("Failed to ensure sync tables: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let status = octo_engine::sync::server::standalone::status(&conn, &query.device_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get sync status: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(status))
}

/// POST /api/sync/pull
pub async fn sync_pull(
    State(state): State<Arc<AppState>>,
    Json(req): Json<octo_engine::sync::SyncPullRequest>,
) -> Result<Json<octo_engine::sync::SyncPullResponse>, StatusCode> {
    let conn = tokio_rusqlite::Connection::open(&state.db_path)
        .await
        .map_err(|e| {
            tracing::error!("Failed to open DB for sync pull: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    octo_engine::sync::server::standalone::ensure_tables(&conn)
        .await
        .map_err(|e| {
            tracing::error!("Failed to ensure sync tables: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let resp = octo_engine::sync::server::standalone::pull(&conn, &req)
        .await
        .map_err(|e| {
            tracing::error!("Sync pull failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(resp))
}

/// POST /api/sync/push
pub async fn sync_push(
    State(state): State<Arc<AppState>>,
    Json(req): Json<octo_engine::sync::SyncPushRequest>,
) -> Result<Json<octo_engine::sync::SyncPushResponse>, StatusCode> {
    let conn = tokio_rusqlite::Connection::open(&state.db_path)
        .await
        .map_err(|e| {
            tracing::error!("Failed to open DB for sync push: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    octo_engine::sync::server::standalone::ensure_tables(&conn)
        .await
        .map_err(|e| {
            tracing::error!("Failed to ensure sync tables: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let resp = octo_engine::sync::server::standalone::push(&conn, &req)
        .await
        .map_err(|e| {
            tracing::error!("Sync push failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(resp))
}

// ── Router ──────────────────────────────────────────────────────────────

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/sync/status", get(sync_status))
        .route("/sync/pull", post(sync_pull))
        .route("/sync/push", post(sync_push))
}
