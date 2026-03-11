use std::sync::Arc;

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

// --------------- Response types ---------------

#[derive(Serialize)]
pub struct CollaborationStatusResponse {
    pub id: String,
    pub agent_count: usize,
    pub active_agent: Option<String>,
    pub pending_proposals: usize,
    pub event_count: usize,
    pub state_keys: Vec<String>,
}

#[derive(Serialize)]
pub struct CollaborationAgentResponse {
    pub id: String,
    pub name: String,
    pub capabilities: Vec<String>,
    pub session_id: String,
}

#[derive(Serialize)]
pub struct CollaborationEventResponse {
    #[serde(flatten)]
    pub event: serde_json::Value,
}

#[derive(Serialize)]
pub struct ProposalResponse {
    pub id: String,
    pub from_agent: String,
    pub action: String,
    pub description: String,
    pub status: String,
    pub votes: Vec<VoteResponse>,
}

#[derive(Serialize)]
pub struct VoteResponse {
    pub agent_id: String,
    pub approve: bool,
    pub reason: Option<String>,
}

#[derive(Serialize)]
pub struct SharedStateResponse {
    pub entries: Vec<SharedStateEntry>,
}

#[derive(Serialize)]
pub struct SharedStateEntry {
    pub key: String,
    pub value: serde_json::Value,
}

// --------------- Request types ---------------

#[derive(Deserialize)]
pub struct CreateProposalRequest {
    pub from_agent: String,
    pub action: String,
    pub description: String,
}

#[derive(Deserialize)]
pub struct VoteRequest {
    pub agent_id: String,
    pub approve: bool,
    pub reason: Option<String>,
}

// --------------- Handlers ---------------

/// GET /api/collaboration/status
pub async fn get_status(State(state): State<Arc<AppState>>) -> Json<CollaborationStatusResponse> {
    let collab = state.agent_supervisor.collaboration_context();
    match collab {
        Some(ctx) => {
            let status = ctx.status(0, None);
            Json(CollaborationStatusResponse {
                id: status.id,
                agent_count: status.agent_count,
                active_agent: status.active_agent,
                pending_proposals: status.pending_proposals,
                event_count: status.event_count,
                state_keys: status.state_keys,
            })
        }
        None => Json(CollaborationStatusResponse {
            id: String::new(),
            agent_count: 0,
            active_agent: None,
            pending_proposals: 0,
            event_count: 0,
            state_keys: vec![],
        }),
    }
}

/// GET /api/collaboration/agents
pub async fn list_agents(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<CollaborationAgentResponse>> {
    let agents = state.agent_supervisor.collaboration_agents();
    Json(
        agents
            .into_iter()
            .map(|a| CollaborationAgentResponse {
                id: a.id.clone(),
                name: a.name.clone(),
                capabilities: a
                    .capabilities
                    .iter()
                    .map(|c| format!("{:?}", c))
                    .collect(),
                session_id: a.handle.session_id.as_str().to_string(),
            })
            .collect(),
    )
}

/// GET /api/collaboration/events
pub async fn list_events(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<CollaborationEventResponse>> {
    let collab = state.agent_supervisor.collaboration_context();
    match collab {
        Some(ctx) => {
            let events = ctx.events();
            Json(
                events
                    .into_iter()
                    .filter_map(|e| {
                        serde_json::to_value(&e)
                            .ok()
                            .map(|v| CollaborationEventResponse { event: v })
                    })
                    .collect(),
            )
        }
        None => Json(vec![]),
    }
}

/// GET /api/collaboration/proposals
pub async fn list_proposals(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<ProposalResponse>> {
    let collab = state.agent_supervisor.collaboration_context();
    match collab {
        Some(ctx) => {
            let proposals = ctx.proposals();
            Json(
                proposals
                    .into_iter()
                    .map(|p| ProposalResponse {
                        id: p.id,
                        from_agent: p.from_agent,
                        action: p.action,
                        description: p.description,
                        status: format!("{:?}", p.status),
                        votes: p
                            .votes
                            .into_iter()
                            .map(|v| VoteResponse {
                                agent_id: v.agent_id,
                                approve: v.approve,
                                reason: v.reason,
                            })
                            .collect(),
                    })
                    .collect(),
            )
        }
        None => Json(vec![]),
    }
}

/// POST /api/collaboration/proposals
pub async fn create_proposal(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateProposalRequest>,
) -> Json<serde_json::Value> {
    let collab = state.agent_supervisor.collaboration_context();
    match collab {
        Some(ctx) => {
            let id = octo_engine::agent::CollaborationProtocol::propose_action(
                &ctx,
                &req.from_agent,
                req.action,
                req.description,
            );
            Json(serde_json::json!({ "id": id }))
        }
        None => Json(serde_json::json!({ "error": "No active collaboration" })),
    }
}

/// POST /api/collaboration/proposals/:id/vote
pub async fn vote_on_proposal(
    State(state): State<Arc<AppState>>,
    Path(proposal_id): Path<String>,
    Json(req): Json<VoteRequest>,
) -> Json<serde_json::Value> {
    let collab = state.agent_supervisor.collaboration_context();
    match collab {
        Some(ctx) => {
            let found = octo_engine::agent::CollaborationProtocol::vote(
                &ctx,
                &proposal_id,
                &req.agent_id,
                req.approve,
                req.reason,
            );
            Json(serde_json::json!({ "found": found }))
        }
        None => Json(serde_json::json!({ "error": "No active collaboration" })),
    }
}

/// GET /api/collaboration/shared-state
pub async fn get_shared_state(
    State(state): State<Arc<AppState>>,
) -> Json<SharedStateResponse> {
    let collab = state.agent_supervisor.collaboration_context();
    match collab {
        Some(ctx) => {
            let keys = ctx.state_keys();
            let entries: Vec<SharedStateEntry> = keys
                .into_iter()
                .filter_map(|k| {
                    ctx.get_state(&k).map(|v| SharedStateEntry {
                        key: k,
                        value: v,
                    })
                })
                .collect();
            Json(SharedStateResponse { entries })
        }
        None => Json(SharedStateResponse { entries: vec![] }),
    }
}

// --------------- Router ---------------

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/collaboration/status", get(get_status))
        .route("/collaboration/agents", get(list_agents))
        .route("/collaboration/events", get(list_events))
        .route(
            "/collaboration/proposals",
            get(list_proposals).post(create_proposal),
        )
        .route(
            "/collaboration/proposals/{id}/vote",
            post(vote_on_proposal),
        )
        .route("/collaboration/shared-state", get(get_shared_state))
}
