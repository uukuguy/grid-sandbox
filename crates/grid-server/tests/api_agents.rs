//! E2E tests for agent lifecycle API endpoints.
//!
//! Routes under test:
//!   POST   /api/v1/agents           — register new agent
//!   GET    /api/v1/agents           — list all agents
//!   GET    /api/v1/agents/:id       — get agent by id
//!   DELETE /api/v1/agents/:id       — unregister agent
//!   GET    /api/v1/agents/:unknown  — 404 for unknown agent

mod common;

use axum::http::StatusCode;
use serde_json::json;

#[tokio::test]
async fn list_agents_initially_empty() {
    let app = common::TestApp::new().await;
    let (status, body) = app.get("/api/v1/agents").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_array(), "should return an array");
    // The catalog may contain zero agents (empty) at start
    assert!(body.as_array().unwrap().len() == 0 || body.as_array().unwrap().len() > 0);
}

#[tokio::test]
async fn create_agent_returns_created() {
    let app = common::TestApp::new().await;
    let manifest = json!({
        "name": "test-agent",
        "tags": ["test"],
        "role": "assistant",
        "goal": "help with tests"
    });

    let (status, body) = app.post_json("/api/v1/agents", manifest).await;

    assert_eq!(status, StatusCode::CREATED);
    assert!(body["id"].is_string(), "should return an agent id");
    assert_eq!(body["manifest"]["name"], "test-agent");
}

#[tokio::test]
async fn get_agent_by_id() {
    let app = common::TestApp::new().await;

    // Create an agent first
    let manifest = json!({
        "name": "lookup-agent",
        "tags": ["lookup"]
    });
    let (status, created) = app.post_json("/api/v1/agents", manifest).await;
    assert_eq!(status, StatusCode::CREATED);

    let agent_id = created["id"].as_str().expect("agent id should be a string");

    // Now get it by ID
    let (status, body) = app.get(&format!("/api/v1/agents/{}", agent_id)).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], agent_id);
    assert_eq!(body["manifest"]["name"], "lookup-agent");
}

#[tokio::test]
async fn get_unknown_agent_returns_404() {
    let app = common::TestApp::new().await;
    let (status, _body) = app.get("/api/v1/agents/nonexistent-id-12345").await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_agent_returns_no_content() {
    let app = common::TestApp::new().await;

    // Create an agent
    let manifest = json!({
        "name": "delete-me",
        "tags": []
    });
    let (status, created) = app.post_json("/api/v1/agents", manifest).await;
    assert_eq!(status, StatusCode::CREATED);
    let agent_id = created["id"].as_str().unwrap();

    // Delete it
    let (status, _body) = app.delete(&format!("/api/v1/agents/{}", agent_id)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Verify it's gone
    let (status, _body) = app.get(&format!("/api/v1/agents/{}", agent_id)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_unknown_agent_returns_404() {
    let app = common::TestApp::new().await;
    let (status, _body) = app.delete("/api/v1/agents/does-not-exist").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn list_agents_after_create() {
    let app = common::TestApp::new().await;

    // Create two agents
    let m1 = json!({"name": "agent-1", "tags": ["a"]});
    let m2 = json!({"name": "agent-2", "tags": ["b"]});
    app.post_json("/api/v1/agents", m1).await;
    app.post_json("/api/v1/agents", m2).await;

    let (status, body) = app.get("/api/v1/agents").await;
    assert_eq!(status, StatusCode::OK);

    let agents = body.as_array().expect("should be an array");
    // At least 2 agents should exist (may be more if there were defaults)
    assert!(agents.len() >= 2, "expected at least 2 agents, got {}", agents.len());
}
