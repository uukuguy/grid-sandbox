//! Extension context.

use std::path::PathBuf;
use std::sync::Arc;

use crate::extension::traits::ExtensionHostActions;

/// Context passed to extension hooks.
pub struct ExtensionContext {
    /// Session ID.
    pub session_id: String,
    /// User ID.
    pub user_id: String,
    /// Sandbox ID.
    pub sandbox_id: String,
    /// Working directory.
    pub working_dir: PathBuf,
    /// Host actions.
    pub host_actions: Arc<dyn ExtensionHostActions>,
    /// Current messages as JSON string.
    pub messages_json: String,
    /// Current round.
    pub round: u32,
    /// Tool calls this round.
    pub tool_calls_this_round: u32,
    /// Total tool calls.
    pub total_tool_calls: u32,
}

impl ExtensionContext {
    /// Create a new extension context.
    pub fn new(
        session_id: String,
        user_id: String,
        sandbox_id: String,
        working_dir: PathBuf,
        host_actions: Arc<dyn ExtensionHostActions>,
    ) -> Self {
        Self {
            session_id,
            user_id,
            sandbox_id,
            working_dir,
            host_actions,
            messages_json: String::new(),
            round: 0,
            tool_calls_this_round: 0,
            total_tool_calls: 0,
        }
    }

    /// Get the working directory.
    pub fn working_directory(&self) -> &PathBuf {
        &self.working_dir
    }

    /// Get the session ID.
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Update messages as JSON.
    pub fn set_messages_json(&mut self, messages_json: String) {
        self.messages_json = messages_json;
    }

    /// Get current messages as JSON.
    pub fn messages_json(&self) -> &str {
        &self.messages_json
    }

    /// Increment round counter.
    pub fn next_round(&mut self) {
        self.round += 1;
        self.tool_calls_this_round = 0;
    }

    /// Record a tool call.
    pub fn record_tool_call(&mut self) {
        self.tool_calls_this_round += 1;
        self.total_tool_calls += 1;
    }

    /// Get current round.
    pub fn round(&self) -> u32 {
        self.round
    }

    /// Get tool calls this round.
    pub fn tool_calls_this_round(&self) -> u32 {
        self.tool_calls_this_round
    }

    /// Get total tool calls.
    pub fn total_tool_calls(&self) -> u32 {
        self.total_tool_calls
    }
}

/// Builder for ExtensionContext.
pub struct ExtensionContextBuilder {
    session_id: Option<String>,
    user_id: Option<String>,
    sandbox_id: Option<String>,
    working_dir: Option<PathBuf>,
    host_actions: Option<Arc<dyn ExtensionHostActions>>,
}

impl ExtensionContextBuilder {
    pub fn new() -> Self {
        Self {
            session_id: None,
            user_id: None,
            sandbox_id: None,
            working_dir: None,
            host_actions: None,
        }
    }

    pub fn session_id(mut self, id: String) -> Self {
        self.session_id = Some(id);
        self
    }

    pub fn user_id(mut self, id: String) -> Self {
        self.user_id = Some(id);
        self
    }

    pub fn sandbox_id(mut self, id: String) -> Self {
        self.sandbox_id = Some(id);
        self
    }

    pub fn working_dir(mut self, dir: PathBuf) -> Self {
        self.working_dir = Some(dir);
        self
    }

    pub fn host_actions(mut self, actions: Arc<dyn ExtensionHostActions>) -> Self {
        self.host_actions = Some(actions);
        self
    }

    pub fn build(self) -> ExtensionContext {
        ExtensionContext::new(
            self.session_id.unwrap_or_default(),
            self.user_id.unwrap_or_default(),
            self.sandbox_id.unwrap_or_default(),
            self.working_dir.unwrap_or_else(|| PathBuf::from(".")),
            self.host_actions.unwrap_or_else(|| {
                Arc::new(crate::extension::traits::InMemoryExtensionHostActions::new(
                    PathBuf::from("."),
                    "default".to_string(),
                ))
            }),
        )
    }
}

impl Default for ExtensionContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_builder() {
        let ctx = ExtensionContextBuilder::new()
            .session_id("test-session".to_string())
            .user_id("test-user".to_string())
            .sandbox_id("test-sandbox".to_string())
            .working_dir(PathBuf::from("/tmp"))
            .build();

        assert_eq!(ctx.session_id, "test-session");
        assert_eq!(ctx.user_id, "test-user");
        assert_eq!(ctx.sandbox_id, "test-sandbox");
        assert_eq!(ctx.working_dir, PathBuf::from("/tmp"));
    }

    #[test]
    fn test_round_increment() {
        let mut ctx = ExtensionContextBuilder::new().build();

        ctx.next_round();
        assert_eq!(ctx.round, 1);

        ctx.record_tool_call();
        ctx.record_tool_call();
        assert_eq!(ctx.tool_calls_this_round, 2);
        assert_eq!(ctx.total_tool_calls, 2);

        ctx.next_round();
        assert_eq!(ctx.round, 2);
        assert_eq!(ctx.tool_calls_this_round, 0);
    }
}
