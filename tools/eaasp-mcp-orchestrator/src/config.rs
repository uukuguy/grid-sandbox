use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// MCP server run mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunMode {
    /// Single shared process for all sessions.
    Shared,
    /// One process per session (future: Docker).
    PerSession,
    /// Start on demand, stop after idle timeout.
    OnDemand,
}

/// MCP server definition from YAML config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerDef {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub transport: String,
    pub port: u16,
    pub mode: RunMode,
    pub tags: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub health_endpoint: String,
}

/// Top-level YAML config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    pub servers: Vec<McpServerDef>,
}
