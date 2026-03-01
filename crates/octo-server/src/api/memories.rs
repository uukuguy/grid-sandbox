use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;

use octo_types::{MemoryCategory, MemoryEntry, MemoryFilter, MemoryId, SandboxId, SearchOptions, UserId};

use crate::state::AppState;

const DEFAULT_USER_ID: &str = "default";

#[derive(Deserialize)]
pub struct MemorySearchParams {
    pub q: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    20
}

/// Request body for creating a memory
#[derive(Deserialize)]
pub struct CreateMemoryRequest {
    /// Memory content (required)
    pub content: String,
    /// Memory category (optional, defaults to "general")
    pub category: Option<String>,
    /// Sandbox ID (optional)
    pub sandbox_id: Option<String>,
    /// Metadata (optional, JSON string or object)
    pub metadata: Option<serde_json::Value>,
    /// Importance score (optional, 0-100)
    pub importance: Option<i32>,
}

pub async fn search_memories(
    State(state): State<Arc<AppState>>,
    Query(params): Query<MemorySearchParams>,
) -> Json<serde_json::Value> {
    let query = params.q.unwrap_or_default();

    // No query: list all memories for the default user
    if query.is_empty() {
        let filter = MemoryFilter {
            user_id: DEFAULT_USER_ID.to_string(),
            limit: params.limit.min(100),
            ..Default::default()
        };
        return match state.memory_store.list(filter).await {
            Ok(entries) => Json(serde_json::json!({ "results": entries })),
            Err(_) => Json(serde_json::json!({ "results": [] })),
        };
    }

    // With query: FTS search
    let opts = SearchOptions {
        user_id: DEFAULT_USER_ID.to_string(),
        limit: params.limit.min(100),
        ..Default::default()
    };

    match state.memory_store.search(&query, opts).await {
        Ok(entries) => Json(serde_json::json!({ "results": entries })),
        Err(_) => Json(serde_json::json!({ "results": [] })),
    }
}

#[derive(Deserialize)]
pub struct WorkingMemoryParams {
    pub sandbox_id: Option<String>,
}

pub async fn get_working_memory(
    State(state): State<Arc<AppState>>,
    Query(params): Query<WorkingMemoryParams>,
) -> Json<serde_json::Value> {
    let user_id = UserId::from_string(DEFAULT_USER_ID);
    let sandbox_id = SandboxId::from_string(
        params.sandbox_id.as_deref().unwrap_or(DEFAULT_USER_ID),
    );
    match state.memory.get_blocks(&user_id, &sandbox_id).await {
        Ok(blocks) => Json(serde_json::json!({ "blocks": blocks })),
        Err(_) => Json(serde_json::json!({ "blocks": [] })),
    }
}

pub async fn get_memory(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let mem_id = MemoryId::from_string(&id);
    match state.memory_store.get(&mem_id).await {
        Ok(Some(entry)) => Json(serde_json::to_value(entry).unwrap_or_default()),
        Ok(None) => Json(serde_json::json!({"error": "not found"})),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

pub async fn delete_memory(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let mem_id = MemoryId::from_string(&id);
    match state.memory_store.delete(&mem_id).await {
        Ok(()) => Json(serde_json::json!({"deleted": id})),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

#[derive(Deserialize)]
pub struct DeleteFilterParams {
    pub category: Option<String>,
    pub sandbox_id: Option<String>,
}

pub async fn delete_memories_by_filter(
    State(state): State<Arc<AppState>>,
    Query(params): Query<DeleteFilterParams>,
) -> Json<serde_json::Value> {
    let categories = params
        .category
        .as_deref()
        .and_then(MemoryCategory::from_str)
        .map(|c| vec![c]);

    let filter = MemoryFilter {
        user_id: DEFAULT_USER_ID.to_string(),
        sandbox_id: params.sandbox_id,
        categories,
        ..Default::default()
    };

    match state.memory_store.delete_by_filter(filter).await {
        Ok(count) => Json(serde_json::json!({"deleted": count})),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

/// Create a new memory entry
pub async fn create_memory(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateMemoryRequest>,
) -> Json<serde_json::Value> {
    // Parse category or default to "profile"
    let category = req
        .category
        .and_then(|c| MemoryCategory::from_str(&c))
        .unwrap_or(MemoryCategory::Profile);

    // Use provided sandbox_id or default
    let sandbox_id = req
        .sandbox_id
        .unwrap_or_else(|| DEFAULT_USER_ID.to_string());

    // Create memory entry with all fields
    let mut entry = MemoryEntry::new(
        DEFAULT_USER_ID,
        category,
        &req.content,
    );
    entry.sandbox_id = sandbox_id;
    entry.importance = req.importance.unwrap_or(50) as f32 / 100.0;
    if let Some(meta) = req.metadata {
        entry.metadata = meta;
    }

    // Save to store
    match state.memory_store.store(entry).await {
        Ok(id) => Json(serde_json::json!({
            "id": id,
            "created": true,
        })),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}
