//! WebSocket handler for real-time agent communication

use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::{AppState, AuthExtractor};

/// WebSocket message from client
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "chat")]
    Chat { content: String },
    #[serde(rename = "ping")]
    Ping,
}

/// WebSocket message to client
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "response")]
    Response { content: String, done: bool },
    #[serde(rename = "error")]
    Error { message: String },
    #[serde(rename = "pong")]
    Pong,
}

/// WebSocket handler
pub async fn ws_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(session_id): Path<String>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(state, auth.user_id, session_id, socket))
}

async fn handle_socket(_state: Arc<AppState>, _user_id: String, session_id: String, socket: WebSocket) {
    let (mut sender, mut receiver) = socket.split();

    // Create a channel for sending messages back to client
    let (tx, mut rx) = mpsc::channel::<ServerMessage>(100);

    // Spawn task to forward messages to WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let text = serde_json::to_string(&msg).unwrap_or_default();
            if sender.send(Message::Text(text.into())).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                    match client_msg {
                        ClientMessage::Chat { content } => {
                            // TODO: Integrate with AgentRuntime (P1-4)
                            // For now, just echo the message back
                            let response = ServerMessage::Response {
                                content: format!("[Stub] Received: {}", content),
                                done: true,
                            };
                            let _ = tx.send(response).await;
                        }
                        ClientMessage::Ping => {
                            let _ = tx.send(ServerMessage::Pong).await;
                        }
                    }
                }
            }
            Ok(Message::Close(_)) => break,
            Err(_) => break,
            _ => {}
        }
    }

    send_task.abort();
    tracing::info!("WebSocket closed for session: {}", session_id);
}
