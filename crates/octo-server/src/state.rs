use std::sync::Arc;

use octo_engine::{Provider, SessionStore, ToolRegistry, WorkingMemory};

pub struct AppState {
    pub provider: Arc<dyn Provider>,
    pub tools: Arc<ToolRegistry>,
    pub memory: Arc<dyn WorkingMemory>,
    pub sessions: Arc<dyn SessionStore>,
    pub model: Option<String>,
}

impl AppState {
    pub fn new(
        provider: Arc<dyn Provider>,
        tools: Arc<ToolRegistry>,
        memory: Arc<dyn WorkingMemory>,
        sessions: Arc<dyn SessionStore>,
        model: Option<String>,
    ) -> Self {
        Self {
            provider,
            tools,
            memory,
            sessions,
            model,
        }
    }
}
