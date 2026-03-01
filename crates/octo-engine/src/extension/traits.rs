//! Extension traits for octo-engine.

use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;

use crate::extension::context::ExtensionContext;

/// Events emitted by extensions.
#[derive(Debug, Clone)]
pub enum ExtensionEvent {
    /// Agent started.
    AgentStart { session_id: String },
    /// Agent ended.
    AgentEnd { session_id: String, success: bool },
    /// Tool call intercepted.
    ToolCall {
        tool_name: String,
        arguments: serde_json::Value,
    },
    /// Tool result intercepted.
    ToolResult {
        tool_name: String,
        result: String,
    },
    /// Context compaction happened.
    Compaction {
        before_tokens: u32,
        after_tokens: u32,
    },
    /// Custom event from extension.
    Custom { name: String, data: serde_json::Value },
}

/// Host actions available to extensions.
#[async_trait]
pub trait ExtensionHostActions: Send + Sync {
    /// Get the current working directory.
    fn get_working_directory(&self) -> PathBuf;

    /// Get the sandbox ID.
    fn get_sandbox_id(&self) -> String;

    /// Read a file.
    async fn read_file(&self, path: &std::path::Path) -> Result<String, String>;

    /// Write a file.
    async fn write_file(&self, path: &std::path::Path, content: &str) -> Result<(), String>;

    /// Emit an event to the event bus.
    fn emit_event(&self, event: ExtensionEvent);

    /// Get a configuration value.
    fn get_config(&self, key: &str) -> Option<String>;
}

/// Extension trait - implement to add custom behavior.
#[async_trait]
pub trait Extension: Send + Sync {
    /// Get the extension name.
    fn name(&self) -> &str;

    /// Get the extension version.
    fn version(&self) -> &str;

    /// Called when the agent starts.
    async fn on_agent_start(&self, _ctx: &ExtensionContext) -> Result<(), String> {
        Ok(())
    }

    /// Called when the agent ends.
    async fn on_agent_end(
        &self,
        _ctx: &ExtensionContext,
        _result: &AgentResult,
    ) -> Result<(), String> {
        Ok(())
    }

    /// Called before a tool is executed.
    /// Return modified tool output to intercept, or None to proceed normally.
    async fn on_tool_call(
        &self,
        _ctx: &ExtensionContext,
        _tool_name: &str,
        _arguments: &serde_json::Value,
    ) -> Result<Option<String>, String> {
        Ok(None)
    }

    /// Called after a tool executes.
    /// Return modified result, or None to use original.
    async fn on_tool_result(
        &self,
        _ctx: &ExtensionContext,
        _tool_name: &str,
        _result: &str,
    ) -> Result<Option<String>, String> {
        Ok(None)
    }

    /// Called before context compaction.
    async fn on_before_compaction(
        &self,
        _ctx: &ExtensionContext,
        _messages_json: &str,
    ) -> Result<(), String> {
        Ok(())
    }

    /// Called after context compaction.
    async fn on_after_compaction(
        &self,
        _ctx: &ExtensionContext,
        _messages_json: &str,
    ) -> Result<(), String> {
        Ok(())
    }
}

/// Hostcall interceptor - intercept and modify tool calls.
#[async_trait]
pub trait HostcallInterceptor: Send + Sync {
    /// Intercept a file read operation.
    /// Return Some(content) to override, None to proceed normally.
    fn intercept_file_read(&self, path: &std::path::Path) -> Option<String> {
        None
    }

    /// Intercept a file write operation.
    /// Return Some(()) to override, None to proceed normally.
    fn intercept_file_write(
        &self,
        path: &std::path::Path,
        _content: &str,
    ) -> Option<Result<(), String>> {
        None
    }

    /// Intercept a shell command.
    /// Return Some(output) to override, None to proceed normally.
    fn intercept_shell(&self, _command: &str) -> Option<Result<String, String>> {
        None
    }

    /// Check if a path is allowed.
    fn is_path_allowed(&self, path: &std::path::Path) -> bool {
        true
    }

    /// Check if a command is allowed.
    fn is_command_allowed(&self, command: &str) -> bool {
        true
    }
}

/// Agent execution result.
#[derive(Debug, Clone)]
pub struct AgentResult {
    /// Whether the agent completed successfully.
    pub success: bool,
    /// Final message from the agent.
    pub final_message: Option<String>,
    /// Number of tool calls made.
    pub tool_calls: u32,
    /// Number of rounds executed.
    pub rounds: u32,
    /// Error message if failed.
    pub error: Option<String>,
}

impl AgentResult {
    /// Create a successful result.
    pub fn success(final_message: String, tool_calls: u32, rounds: u32) -> Self {
        Self {
            success: true,
            final_message: Some(final_message),
            tool_calls,
            rounds,
            error: None,
        }
    }

    /// Create a failed result.
    pub fn failure(error: String) -> Self {
        Self {
            success: false,
            final_message: None,
            tool_calls: 0,
            rounds: 0,
            error: Some(error),
        }
    }
}

/// Simple in-memory extension host actions implementation.
pub struct InMemoryExtensionHostActions {
    working_dir: PathBuf,
    sandbox_id: String,
    config: std::collections::HashMap<String, String>,
}

impl InMemoryExtensionHostActions {
    pub fn new(working_dir: PathBuf, sandbox_id: String) -> Self {
        Self {
            working_dir,
            sandbox_id,
            config: std::collections::HashMap::new(),
        }
    }

    pub fn with_config(mut self, key: &str, value: &str) -> Self {
        self.config.insert(key.to_string(), value.to_string());
        self
    }
}

#[async_trait]
impl ExtensionHostActions for InMemoryExtensionHostActions {
    fn get_working_directory(&self) -> PathBuf {
        self.working_dir.clone()
    }

    fn get_sandbox_id(&self) -> String {
        self.sandbox_id.clone()
    }

    async fn read_file(&self, path: &std::path::Path) -> Result<String, String> {
        std::fs::read_to_string(path).map_err(|e| e.to_string())
    }

    async fn write_file(&self, path: &std::path::Path, content: &str) -> Result<(), String> {
        std::fs::write(path, content).map_err(|e| e.to_string())
    }

    fn emit_event(&self, _event: ExtensionEvent) {
        // No-op for in-memory implementation
    }

    fn get_config(&self, key: &str) -> Option<String> {
        self.config.get(key).cloned()
    }
}
