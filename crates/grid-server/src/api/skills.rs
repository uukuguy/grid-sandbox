//! Skill catalog REST API
//!
//! GET    /api/v1/skills              list all loaded skills
//! GET    /api/v1/skills/:name        get skill details
//! POST   /api/v1/skills/:name/execute  execute a skill via SkillTool
//! DELETE /api/v1/skills/:name        unload a skill

use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use grid_types::{SandboxId, ToolContext};

use crate::state::AppState;

#[derive(Serialize)]
struct SkillListResponse {
    skills: Vec<SkillInfo>,
    total: usize,
}

#[derive(Serialize)]
struct SkillInfo {
    name: String,
    description: String,
    version: Option<String>,
    user_invocable: bool,
    allowed_tools: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct ExecuteRequest {
    /// Action to perform: "activate" (default), "run_script", "list_scripts"
    #[serde(default = "default_action")]
    action: String,
    /// Optional arguments (e.g. script name for run_script)
    #[serde(default)]
    args: Option<serde_json::Value>,
}

fn default_action() -> String {
    "activate".to_string()
}

#[derive(Serialize)]
struct ExecuteResponse {
    status: String,
    /// Skill output content or error message
    result: serde_json::Value,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/skills", get(list_skills))
        .route("/skills/{name}", get(get_skill).delete(delete_skill))
        .route("/skills/{name}/execute", post(execute_skill))
}

async fn list_skills(State(state): State<Arc<AppState>>) -> Json<SkillListResponse> {
    let skills: Vec<SkillInfo> = state
        .agent_supervisor
        .skill_registry()
        .map(|reg| {
            reg.list_all()
                .into_iter()
                .map(|s| SkillInfo {
                    name: s.name,
                    description: s.description,
                    version: s.version,
                    user_invocable: s.user_invocable,
                    allowed_tools: s.allowed_tools,
                })
                .collect()
        })
        .unwrap_or_default();

    let total = skills.len();
    Json(SkillListResponse { skills, total })
}

async fn get_skill(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<SkillInfo>, StatusCode> {
    let registry = state
        .agent_supervisor
        .skill_registry()
        .ok_or(StatusCode::NOT_FOUND)?;

    let skill = registry.get(&name).ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(SkillInfo {
        name: skill.name,
        description: skill.description,
        version: skill.version,
        user_invocable: skill.user_invocable,
        allowed_tools: skill.allowed_tools,
    }))
}

async fn execute_skill(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(body): Json<ExecuteRequest>,
) -> Result<Json<ExecuteResponse>, StatusCode> {
    use grid_engine::skills::SkillTool;
    use grid_engine::tools::Tool;

    // Look up skill in registry
    let registry = state
        .agent_supervisor
        .skill_registry()
        .ok_or(StatusCode::NOT_FOUND)?;

    let skill = registry.get(&name).ok_or(StatusCode::NOT_FOUND)?;

    // Build the SkillTool wrapper
    let skill_tool = SkillTool::new(skill);

    // Construct parameters for execute()
    let params = serde_json::json!({
        "action": body.action,
        "args": body.args,
    });

    // Create a minimal ToolContext for the execution
    let tool_ctx = ToolContext {
        sandbox_id: SandboxId::from_string("api-skill-exec"),
        user_id: grid_types::UserId::from_string(grid_types::id::DEFAULT_USER_ID),
        working_dir: PathBuf::from("."),
        path_validator: None,
    };

    match skill_tool.execute(params, &tool_ctx).await {
        Ok(output) => Ok(Json(ExecuteResponse {
            status: if output.is_error { "error" } else { "ok" }.to_string(),
            result: serde_json::json!({
                "content": output.content,
                "metadata": output.metadata,
            }),
        })),
        Err(e) => {
            tracing::error!(skill = %name, error = %e, "Skill execution failed");
            Ok(Json(ExecuteResponse {
                status: "error".to_string(),
                result: serde_json::json!({ "message": e.to_string() }),
            }))
        }
    }
}

async fn delete_skill(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let registry = state
        .agent_supervisor
        .skill_registry()
        .ok_or(StatusCode::NOT_FOUND)?;

    if registry.remove(&name).is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    tracing::info!(skill = %name, "Skill unloaded via REST API");
    Ok(StatusCode::NO_CONTENT)
}
