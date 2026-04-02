//! Session management tools for LLM-initiated multi-session coordination (T-G1).
//!
//! Provides 4 tools: session_create, session_message, session_status, session_stop.
//! These wrap the AgentRuntime's SessionRegistry, exposing multi-session capabilities
//! to the LLM via the standard Tool trait.

use std::sync::Arc;
#[allow(unused_imports)]
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use dashmap::DashMap;
use serde_json::{json, Value};

use super::traits::Tool;
use crate::agent::executor::{AgentExecutorHandle, AgentMessage};
use crate::agent::runtime::SessionEntry;
use octo_types::{RiskLevel, SessionId, ToolContext, ToolOutput, ToolSource};

// ─── Tool descriptions ───

#[allow(dead_code)]
const SESSION_CREATE_DESCRIPTION: &str = r#"Create a new agent sub-session to handle an independent task.

## When to use
- Need to parallelize multiple independent sub-tasks
- Research tasks that require heavy searching/reading that would fill the main context
- Tasks that need different tool permissions or models

## When NOT to use
- Simple single-step operations (just use the tool directly)
- Tasks that need the main session's conversation history (sub-sessions start fresh)
- Searching within 2-3 files (use grep/glob directly)

## Writing good prompts
Sub-sessions start with zero context. Write prompts like you're briefing a smart colleague who just walked in:
- Explain what you want done and why
- Describe what you already know and what you've ruled out
- Give enough background for the sub-session to make judgments, not just follow orders mechanically
- If you need a brief response, say so explicitly

Do NOT push synthesis to sub-sessions. "Fix the bug based on your findings" is a bad prompt — specify what file and what change to make.

## Example
session_create(prompt: "Investigate all token-counting code in crates/octo-engine/src/context/. List each function's location, parameters, and return value. Under 200 words.")
"#;

const SESSION_MESSAGE_DESCRIPTION: &str = r#"Send a message to a running sub-session.

Your plain text output is NOT visible to other sessions — you must use this tool to communicate with sub-sessions.

## Usage
- Use the session name or ID as the `to` field
- Message content is plain text

## Notes
- The sub-session processes the message on its next turn
- Do not guess sub-session output before it returns results
"#;

const SESSION_STATUS_DESCRIPTION: &str = r#"Check the status of one or all sessions.

Returns session ID, active status, and creation time. Use this to check if a sub-session is still running before sending messages or to get an overview of all active sessions.
"#;

const SESSION_STOP_DESCRIPTION: &str = r#"Stop and clean up a running sub-session.

The session's resources (tools, memory, MCP connections) are released. This is irreversible — the session cannot be resumed after stopping.

Use this to clean up sub-sessions that are no longer needed, or to cancel a sub-session that is stuck.
"#;

// ─── SessionCreateTool ───
// NOTE: SessionCreateTool requires Arc<AgentRuntime> which is not available at construction time.
// Deferred until post-init registration pattern is implemented. spawn_subagent covers the core use case.

/// Trait for session creation, implemented by AgentRuntime post-construction.
#[allow(dead_code)]
#[async_trait]
pub trait SessionCreator: Send + Sync {
    async fn create_session(
        &self,
        prompt: &str,
        name: Option<&str>,
        model: Option<&str>,
    ) -> Result<(SessionId, AgentExecutorHandle)>;
}

#[allow(dead_code)]
pub struct SessionCreateTool {
    creator: Arc<dyn SessionCreator>,
}

#[allow(dead_code)]
impl SessionCreateTool {
    pub fn new(creator: Arc<dyn SessionCreator>) -> Self {
        Self { creator }
    }
}

#[async_trait]
impl Tool for SessionCreateTool {
    fn name(&self) -> &str {
        "session_create"
    }

    fn description(&self) -> &str {
        SESSION_CREATE_DESCRIPTION
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "required": ["prompt"],
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "Task instruction for the sub-session"
                },
                "name": {
                    "type": "string",
                    "description": "Optional human-readable name for the session"
                },
                "model": {
                    "type": "string",
                    "description": "Optional model override (e.g. 'claude-haiku-4-5-20251001')"
                },
                "run_in_background": {
                    "type": "boolean",
                    "description": "If true, return immediately without waiting for result"
                }
            }
        })
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        let prompt = params
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: prompt"))?;
        let name = params.get("name").and_then(|v| v.as_str());
        let model = params.get("model").and_then(|v| v.as_str());

        match self.creator.create_session(prompt, name, model).await {
            Ok((session_id, _handle)) => {
                let display_name = name.unwrap_or(session_id.as_str());
                Ok(ToolOutput::success(format!(
                    "Sub-session created: {display_name} (id: {}). \
                     The session is processing your prompt. Results will be delivered when complete.",
                    session_id.as_str()
                )))
            }
            Err(e) => Ok(ToolOutput::error(format!(
                "Failed to create sub-session: {e}"
            ))),
        }
    }

    fn source(&self) -> ToolSource {
        ToolSource::BuiltIn
    }

    fn category(&self) -> &str {
        "session"
    }

    fn execution_timeout(&self) -> Duration {
        Duration::from_secs(30)
    }
}

// ─── SessionMessageTool ───

pub struct SessionMessageTool {
    sessions: Arc<DashMap<SessionId, SessionEntry>>,
}

impl SessionMessageTool {
    pub fn new(sessions: Arc<DashMap<SessionId, SessionEntry>>) -> Self {
        Self { sessions }
    }
}

#[async_trait]
impl Tool for SessionMessageTool {
    fn name(&self) -> &str {
        "session_message"
    }

    fn description(&self) -> &str {
        SESSION_MESSAGE_DESCRIPTION
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "required": ["to", "message"],
            "properties": {
                "to": {
                    "type": "string",
                    "description": "Target session ID or name"
                },
                "message": {
                    "type": "string",
                    "description": "Message content to send"
                }
            }
        })
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        let to = params
            .get("to")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: to"))?;
        let message = params
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: message"))?;

        let session_id = SessionId::from_string(to);
        match self.sessions.get(&session_id) {
            Some(entry) => {
                match entry
                    .handle
                    .send(AgentMessage::UserMessage {
                        content: message.to_string(),
                        channel_id: "session_message".to_string(),
                    })
                    .await
                {
                    Ok(_) => Ok(ToolOutput::success(format!(
                        "Message delivered to session {to}."
                    ))),
                    Err(e) => Ok(ToolOutput::error(format!(
                        "Failed to deliver message to session {to}: {e}"
                    ))),
                }
            }
            None => Ok(ToolOutput::error(format!(
                "Session not found: {to}. Use session_status to check active sessions."
            ))),
        }
    }

    fn source(&self) -> ToolSource {
        ToolSource::BuiltIn
    }

    fn category(&self) -> &str {
        "session"
    }
}

// ─── SessionStatusTool ───

pub struct SessionStatusTool {
    sessions: Arc<DashMap<SessionId, SessionEntry>>,
}

impl SessionStatusTool {
    pub fn new(sessions: Arc<DashMap<SessionId, SessionEntry>>) -> Self {
        Self { sessions }
    }
}

#[async_trait]
impl Tool for SessionStatusTool {
    fn name(&self) -> &str {
        "session_status"
    }

    fn description(&self) -> &str {
        SESSION_STATUS_DESCRIPTION
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "session_id": {
                    "type": "string",
                    "description": "Optional session ID to query. If omitted, returns all sessions."
                }
            }
        })
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        let session_id = params.get("session_id").and_then(|v| v.as_str());

        if let Some(id) = session_id {
            // Query a specific session
            let sid = SessionId::from_string(id);
            match self.sessions.get(&sid) {
                Some(entry) => {
                    let elapsed = entry.created_at.elapsed();
                    let last_activity = entry.last_activity.lock().unwrap().elapsed();
                    Ok(ToolOutput::success(json!({
                        "session_id": id,
                        "active": true,
                        "user_id": entry.user_id.as_str(),
                        "uptime_secs": elapsed.as_secs(),
                        "idle_secs": last_activity.as_secs(),
                    }).to_string()))
                }
                None => Ok(ToolOutput::success(json!({
                    "session_id": id,
                    "active": false,
                }).to_string())),
            }
        } else {
            // List all sessions
            let sessions: Vec<Value> = self
                .sessions
                .iter()
                .map(|entry| {
                    let elapsed = entry.value().created_at.elapsed();
                    let last_activity = entry.value().last_activity.lock().unwrap().elapsed();
                    json!({
                        "session_id": entry.key().as_str(),
                        "user_id": entry.value().user_id.as_str(),
                        "uptime_secs": elapsed.as_secs(),
                        "idle_secs": last_activity.as_secs(),
                    })
                })
                .collect();
            Ok(ToolOutput::success(json!({
                "active_sessions": sessions.len(),
                "sessions": sessions,
            }).to_string()))
        }
    }

    fn source(&self) -> ToolSource {
        ToolSource::BuiltIn
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn category(&self) -> &str {
        "session"
    }
}

// ─── SessionStopTool ───

pub struct SessionStopTool {
    sessions: Arc<DashMap<SessionId, SessionEntry>>,
}

impl SessionStopTool {
    pub fn new(sessions: Arc<DashMap<SessionId, SessionEntry>>) -> Self {
        Self { sessions }
    }
}

#[async_trait]
impl Tool for SessionStopTool {
    fn name(&self) -> &str {
        "session_stop"
    }

    fn description(&self) -> &str {
        SESSION_STOP_DESCRIPTION
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "required": ["session_id"],
            "properties": {
                "session_id": {
                    "type": "string",
                    "description": "ID of the session to stop"
                }
            }
        })
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        let id = params
            .get("session_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: session_id"))?;

        let session_id = SessionId::from_string(id);
        match self.sessions.remove(&session_id) {
            Some(_) => Ok(ToolOutput::success(format!(
                "Session {id} stopped and cleaned up."
            ))),
            None => Ok(ToolOutput::error(format!(
                "Session not found: {id}. It may have already been stopped."
            ))),
        }
    }

    fn source(&self) -> ToolSource {
        ToolSource::BuiltIn
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::HighRisk
    }

    fn category(&self) -> &str {
        "session"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;
    use tokio::sync::broadcast;

    // Returns (sessions_map, _rx_keepalive) — keep rx alive so send() succeeds in tests.
    fn make_test_sessions() -> (
        Arc<DashMap<SessionId, SessionEntry>>,
        tokio::sync::mpsc::Receiver<AgentMessage>,
    ) {
        let map = Arc::new(DashMap::new());
        let (tx, rx) = tokio::sync::mpsc::channel(16);
        let (btx, _) = broadcast::channel(16);
        let sid = SessionId::from_string("test-session-1");
        let handle = AgentExecutorHandle {
            tx,
            broadcast_tx: btx,
            session_id: sid.clone(),
        };
        let tools = Arc::new(std::sync::Mutex::new(super::super::ToolRegistry::new()));
        map.insert(
            sid,
            SessionEntry {
                handle,
                user_id: octo_types::UserId::from_string("user1"),
                created_at: Instant::now(),
                tools,
                last_activity: Arc::new(std::sync::Mutex::new(Instant::now())),
            },
        );
        (map, rx)
    }

    #[test]
    fn test_session_status_tool_metadata() {
        let (sessions, _rx) = make_test_sessions();
        let tool = SessionStatusTool::new(sessions);
        assert_eq!(tool.name(), "session_status");
        assert!(tool.is_read_only());
        assert_eq!(tool.category(), "session");
    }

    #[test]
    fn test_session_stop_tool_metadata() {
        let (sessions, _rx) = make_test_sessions();
        let tool = SessionStopTool::new(sessions);
        assert_eq!(tool.name(), "session_stop");
        assert_eq!(tool.category(), "session");
    }

    #[test]
    fn test_session_message_tool_metadata() {
        let (sessions, _rx) = make_test_sessions();
        let tool = SessionMessageTool::new(sessions);
        assert_eq!(tool.name(), "session_message");
        assert_eq!(tool.category(), "session");
    }

    #[tokio::test]
    async fn test_session_status_list_all() {
        let (sessions, _rx) = make_test_sessions();
        let tool = SessionStatusTool::new(sessions);
        let ctx = ToolContext {
            sandbox_id: octo_types::SandboxId::new(),
            user_id: octo_types::UserId::from_string("user1"),
            working_dir: std::path::PathBuf::from("/tmp"),
            path_validator: None,
        };
        let result = tool.execute(json!({}), &ctx).await.unwrap();
        let text = result.content;
        assert!(text.contains("active_sessions"));
        assert!(text.contains("test-session-1"));
    }

    #[tokio::test]
    async fn test_session_status_specific() {
        let (sessions, _rx) = make_test_sessions();
        let tool = SessionStatusTool::new(sessions);
        let ctx = ToolContext {
            sandbox_id: octo_types::SandboxId::new(),
            user_id: octo_types::UserId::from_string("user1"),
            working_dir: std::path::PathBuf::from("/tmp"),
            path_validator: None,
        };
        let result = tool
            .execute(json!({"session_id": "test-session-1"}), &ctx)
            .await
            .unwrap();
        let text = result.content;
        assert!(text.contains("\"active\":true"));
    }

    #[tokio::test]
    async fn test_session_status_not_found() {
        let (sessions, _rx) = make_test_sessions();
        let tool = SessionStatusTool::new(sessions);
        let ctx = ToolContext {
            sandbox_id: octo_types::SandboxId::new(),
            user_id: octo_types::UserId::from_string("user1"),
            working_dir: std::path::PathBuf::from("/tmp"),
            path_validator: None,
        };
        let result = tool
            .execute(json!({"session_id": "nonexistent"}), &ctx)
            .await
            .unwrap();
        let text = result.content;
        assert!(text.contains("\"active\":false"));
    }

    #[tokio::test]
    async fn test_session_stop_success() {
        let (sessions, _rx) = make_test_sessions();
        let tool = SessionStopTool::new(sessions.clone());
        let ctx = ToolContext {
            sandbox_id: octo_types::SandboxId::new(),
            user_id: octo_types::UserId::from_string("user1"),
            working_dir: std::path::PathBuf::from("/tmp"),
            path_validator: None,
        };
        let result = tool
            .execute(json!({"session_id": "test-session-1"}), &ctx)
            .await
            .unwrap();
        assert!(result.content.contains("stopped"));
        assert!(sessions.is_empty());
    }

    #[tokio::test]
    async fn test_session_stop_not_found() {
        let (sessions, _rx) = make_test_sessions();
        let tool = SessionStopTool::new(sessions);
        let ctx = ToolContext {
            sandbox_id: octo_types::SandboxId::new(),
            user_id: octo_types::UserId::from_string("user1"),
            working_dir: std::path::PathBuf::from("/tmp"),
            path_validator: None,
        };
        let result = tool
            .execute(json!({"session_id": "nonexistent"}), &ctx)
            .await
            .unwrap();
        assert!(result.content.contains("not found"));
    }

    #[tokio::test]
    async fn test_session_message_not_found() {
        let (sessions, _rx) = make_test_sessions();
        let tool = SessionMessageTool::new(sessions);
        let ctx = ToolContext {
            sandbox_id: octo_types::SandboxId::new(),
            user_id: octo_types::UserId::from_string("user1"),
            working_dir: std::path::PathBuf::from("/tmp"),
            path_validator: None,
        };
        let result = tool
            .execute(json!({"to": "nonexistent", "message": "hello"}), &ctx)
            .await
            .unwrap();
        assert!(result.content.contains("not found"));
    }

    #[tokio::test]
    async fn test_session_message_success() {
        let (sessions, _rx) = make_test_sessions();
        let tool = SessionMessageTool::new(sessions);
        let ctx = ToolContext {
            sandbox_id: octo_types::SandboxId::new(),
            user_id: octo_types::UserId::from_string("user1"),
            working_dir: std::path::PathBuf::from("/tmp"),
            path_validator: None,
        };
        let result = tool
            .execute(
                json!({"to": "test-session-1", "message": "hello"}),
                &ctx,
            )
            .await
            .unwrap();
        assert!(result.content.contains("delivered"));
    }
}
