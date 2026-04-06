use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};

use crate::models::{PromoteRequest, SearchQuery, SubmitDraftRequest};
use crate::store::SkillStore;

/// Build the Axum router with all skill registry routes.
pub fn router(store: Arc<SkillStore>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/skills/search", get(search_skills))
        .route("/skills/draft", post(submit_draft))
        .route("/skills/{id}/content", get(get_skill_content))
        .route("/skills/{id}/versions", get(list_versions))
        .route("/skills/{id}/promote/{version}", post(promote_skill))
        .with_state(store)
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn submit_draft(
    State(store): State<Arc<SkillStore>>,
    Json(req): Json<SubmitDraftRequest>,
) -> impl IntoResponse {
    match store.submit_draft(req).await {
        Ok(meta) => (StatusCode::CREATED, Json(serde_json::to_value(meta).unwrap())).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn get_skill_content(
    State(store): State<Arc<SkillStore>>,
    Path(id): Path<String>,
    Query(params): Query<SearchQuery>,
) -> impl IntoResponse {
    // Allow optional ?version= query param via the q field (reuse SearchQuery loosely)
    // Actually, let's use a dedicated extraction — but for simplicity, version comes from query
    let version = params.q.clone(); // reuse q as version hint if needed
    match store.read_skill(id, version).await {
        Ok(Some(content)) => Json(serde_json::to_value(content).unwrap()).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "skill not found" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn search_skills(
    State(store): State<Arc<SkillStore>>,
    Query(params): Query<SearchQuery>,
) -> impl IntoResponse {
    let tag = params.tags.clone();
    match store.search(tag, params.q.clone(), params.status.clone(), params.limit).await {
        Ok(results) => Json(serde_json::to_value(results).unwrap()).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn list_versions(
    State(store): State<Arc<SkillStore>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match store.list_versions(id).await {
        Ok(versions) => Json(serde_json::to_value(versions).unwrap()).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn promote_skill(
    State(store): State<Arc<SkillStore>>,
    Path((id, version)): Path<(String, String)>,
    Json(req): Json<PromoteRequest>,
) -> impl IntoResponse {
    match store.promote(id, version, req.target_status).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({ "promoted": true }))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
