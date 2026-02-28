use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// MCP 服务器传输方式
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum McpTransport {
    /// 本地进程 stdin/stdout（默认）
    #[default]
    Stdio,
    /// Streamable HTTP / SSE（远程服务器）
    Sse,
}

impl std::str::FromStr for McpTransport {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "sse" => Ok(McpTransport::Sse),
            "stdio" | _ => Ok(McpTransport::Stdio),
        }
    }
}

/// Info about a tool provided by an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolInfo {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}

/// Configuration for an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// Configuration for an MCP server (persisted version with ID).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfigV2 {
    pub id: String,
    pub name: String,
    pub source: String,
    pub command: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    pub enabled: bool,
    #[serde(default)]
    pub transport: McpTransport,
    /// SSE transport 专用：服务器 URL（如 "http://localhost:8080/mcp"）
    #[serde(default)]
    pub url: Option<String>,
}

impl From<McpServerConfigV2> for McpServerConfig {
    fn from(v2: McpServerConfigV2) -> Self {
        Self {
            name: v2.name,
            command: v2.command,
            args: v2.args,
            env: v2.env,
        }
    }
}

/// Abstraction over MCP protocol client.
#[async_trait]
pub trait McpClient: Send + Sync {
    /// Server name.
    fn name(&self) -> &str;

    /// Connect to the MCP server (spawn process + handshake).
    async fn connect(&mut self) -> Result<()>;

    /// List tools provided by the server.
    async fn list_tools(&self) -> Result<Vec<McpToolInfo>>;

    /// Call a tool on the server.
    async fn call_tool(&self, name: &str, args: serde_json::Value) -> Result<serde_json::Value>;

    /// Check if connected.
    fn is_connected(&self) -> bool;

    /// Graceful shutdown.
    async fn shutdown(&mut self) -> Result<()>;
}
