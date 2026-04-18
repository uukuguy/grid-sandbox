/// ACP (Agent Communication Protocol) JSON-RPC event parser for goose subprocess output.
///
/// Goose emits newline-delimited JSON-RPC 2.0 messages on stdout.
/// This module defines the typed event enum and a `TryFrom<&str>` parser.
use serde::Deserialize;

/// Typed ACP events emitted by goose subprocess.
#[derive(Debug, Clone, PartialEq)]
pub enum AcpEvent {
    /// A text chunk from the agent (`session/update` + `kind=agent_message_chunk`).
    Chunk { text: String, session_id: Option<String> },

    /// A tool call request (`session/update` + `kind=tool_use`).
    ToolCall {
        tool_name: String,
        tool_id: String,
        input_json: String,
        session_id: Option<String>,
    },

    /// A turn completion (`session/update` + `kind=finish` or `session/stopped`).
    Stop { reason: String, session_id: Option<String> },

    /// An error from the agent (`session/error` or `kind=error`).
    Error { message: String, session_id: Option<String> },

    /// Any unrecognised message — caller can log and skip.
    Unknown { raw: String },
}

// ─── Internal serde types ────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
struct RpcMessage {
    method: Option<String>,
    params: Option<serde_json::Value>,
    error: Option<serde_json::Value>,
    #[serde(default)]
    #[allow(dead_code)]
    id: Option<serde_json::Value>,
}

// ─── Parser ──────────────────────────────────────────────────────────────────

impl TryFrom<&str> for AcpEvent {
    type Error = serde_json::Error;

    fn try_from(raw: &str) -> std::result::Result<Self, serde_json::Error> {
        let msg: RpcMessage = serde_json::from_str(raw)?;

        // Error response
        if msg.error.is_some() {
            let message = msg
                .error
                .as_ref()
                .and_then(|e| e.get("message"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error")
                .to_string();
            return Ok(AcpEvent::Error { message, session_id: None });
        }

        let method = msg.method.as_deref().unwrap_or("");
        let params = msg.params.as_ref();

        match method {
            "session/update" => {
                let kind = params
                    .and_then(|p| p.get("kind"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let session_id = params
                    .and_then(|p| p.get("session_id"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                match kind {
                    "agent_message_chunk" => {
                        let text = params
                            .and_then(|p| p.get("content"))
                            .and_then(|c| c.get("text"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        Ok(AcpEvent::Chunk { text, session_id })
                    }
                    "tool_use" => {
                        let content = params.and_then(|p| p.get("content"));
                        let tool_name = content
                            .and_then(|c| c.get("name"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let tool_id = content
                            .and_then(|c| c.get("id"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let input_json = content
                            .and_then(|c| c.get("input"))
                            .map(|v| v.to_string())
                            .unwrap_or_default();
                        Ok(AcpEvent::ToolCall { tool_name, tool_id, input_json, session_id })
                    }
                    "finish" => Ok(AcpEvent::Stop { reason: "finish".to_string(), session_id }),
                    "error" => {
                        let message = params
                            .and_then(|p| p.get("content"))
                            .and_then(|c| c.get("text"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("agent error")
                            .to_string();
                        Ok(AcpEvent::Error { message, session_id })
                    }
                    _ => Ok(AcpEvent::Unknown { raw: raw.to_string() }),
                }
            }
            "session/stopped" => {
                let reason = params
                    .and_then(|p| p.get("reason"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("stopped")
                    .to_string();
                let session_id = params
                    .and_then(|p| p.get("session_id"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                Ok(AcpEvent::Stop { reason, session_id })
            }
            "session/error" => {
                let message = params
                    .and_then(|p| p.get("message"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("session error")
                    .to_string();
                let session_id = params
                    .and_then(|p| p.get("session_id"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                Ok(AcpEvent::Error { message, session_id })
            }
            _ => Ok(AcpEvent::Unknown { raw: raw.to_string() }),
        }
    }
}
