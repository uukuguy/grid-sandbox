pub mod memory;
pub mod sqlite;

use async_trait::async_trait;
use octo_types::{ChatMessage, SandboxId, SessionId, UserId};

#[derive(Debug, Clone)]
pub struct SessionData {
    pub session_id: SessionId,
    pub user_id: UserId,
    pub sandbox_id: SandboxId,
}

#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn create_session(&self) -> SessionData;
    async fn get_session(&self, session_id: &SessionId) -> Option<SessionData>;
    async fn get_messages(&self, session_id: &SessionId) -> Option<Vec<ChatMessage>>;
    async fn push_message(&self, session_id: &SessionId, message: ChatMessage);
    async fn set_messages(&self, session_id: &SessionId, messages: Vec<ChatMessage>);
}

pub use memory::InMemorySessionStore;
pub use sqlite::SqliteSessionStore;
