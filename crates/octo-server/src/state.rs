use std::path::PathBuf;
use std::sync::Arc;

use octo_engine::{
    mcp::{McpManager, McpStorage}, MemoryStore, Provider,
    SessionStore, SkillRegistry, ToolExecutionRecorder, ToolRegistry, WorkingMemory,
};

use crate::config::Config;

pub struct AppState {
    pub provider: Arc<dyn Provider>,
    pub tools: Arc<ToolRegistry>,
    pub memory: Arc<dyn WorkingMemory>,
    pub sessions: Arc<dyn SessionStore>,
    pub memory_store: Arc<dyn MemoryStore>,
    pub db_path: PathBuf,
    pub mcp_manager: Arc<tokio::sync::Mutex<McpManager>>,
    pub model: Option<String>,
    pub recorder: Option<Arc<ToolExecutionRecorder>>,
    pub skill_registry: Arc<SkillRegistry>,
    /// Server configuration for frontend
    pub config: Config,
}

impl AppState {
    pub fn new(
        provider: Arc<dyn Provider>,
        tools: Arc<ToolRegistry>,
        memory: Arc<dyn WorkingMemory>,
        sessions: Arc<dyn SessionStore>,
        memory_store: Arc<dyn MemoryStore>,
        db_path: PathBuf,
        mcp_manager: Arc<tokio::sync::Mutex<McpManager>>,
        model: Option<String>,
        recorder: Option<Arc<ToolExecutionRecorder>>,
        skill_registry: Arc<SkillRegistry>,
        config: Config,
    ) -> Self {
        Self {
            provider,
            tools,
            memory,
            sessions,
            memory_store,
            db_path,
            mcp_manager,
            model,
            recorder,
            skill_registry,
            config,
        }
    }

    /// Get MCP storage on-demand (creates new connection each time)
    pub fn mcp_storage(&self) -> Option<octo_engine::mcp::storage::McpStorage> {
        McpStorage::new(&self.db_path).ok()
    }
}
