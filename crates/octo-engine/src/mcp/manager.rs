use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::tools::ToolRegistry;

use super::bridge::McpToolBridge;
use super::stdio::StdioMcpClient;
use super::traits::{McpClient, McpServerConfig, McpToolInfo};

/// MCP config file format (.octo/mcp.json).
#[derive(Debug, serde::Deserialize)]
struct McpConfigFile {
    servers: HashMap<String, McpServerEntry>,
}

#[derive(Debug, serde::Deserialize)]
struct McpServerEntry {
    command: String,
    args: Vec<String>,
    #[serde(default)]
    env: HashMap<String, String>,
}

/// Manages multiple MCP server connections.
pub struct McpManager {
    clients: HashMap<String, Arc<RwLock<Box<dyn McpClient>>>>,
    tool_infos: HashMap<String, Vec<McpToolInfo>>,
}

impl McpManager {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
            tool_infos: HashMap::new(),
        }
    }

    /// Load MCP server configs from a JSON file.
    pub fn load_config(config_path: &Path) -> Result<Vec<McpServerConfig>> {
        let content = std::fs::read_to_string(config_path)
            .with_context(|| format!("reading {}", config_path.display()))?;
        let config: McpConfigFile = serde_json::from_str(&content)
            .with_context(|| format!("parsing {}", config_path.display()))?;

        Ok(config
            .servers
            .into_iter()
            .map(|(name, entry)| McpServerConfig {
                name,
                command: entry.command,
                args: entry.args,
                env: entry.env,
            })
            .collect())
    }

    /// Add and connect a new MCP server.
    pub async fn add_server(&mut self, config: McpServerConfig) -> Result<Vec<McpToolInfo>> {
        let name = config.name.clone();
        let mut client = StdioMcpClient::new(config);
        client.connect().await?;

        let tools = client.list_tools().await?;
        info!(
            server = %name,
            tool_count = tools.len(),
            "MCP server connected with tools"
        );

        let client: Arc<RwLock<Box<dyn McpClient>>> =
            Arc::new(RwLock::new(Box::new(client)));
        self.clients.insert(name.clone(), client);
        self.tool_infos.insert(name, tools.clone());
        Ok(tools)
    }

    /// Remove and shutdown an MCP server.
    pub async fn remove_server(&mut self, name: &str) -> Result<()> {
        if let Some(client) = self.clients.remove(name) {
            let mut client = client.write().await;
            client.shutdown().await?;
        }
        self.tool_infos.remove(name);
        info!(server = %name, "MCP server removed");
        Ok(())
    }

    /// Bridge all MCP tools into a ToolRegistry.
    pub fn bridge_tools(&self, registry: &mut ToolRegistry) {
        for (server_name, tools) in &self.tool_infos {
            let client = self.clients.get(server_name).unwrap().clone();
            for tool_info in tools {
                let bridge = McpToolBridge::new(
                    client.clone(),
                    server_name.clone(),
                    tool_info.clone(),
                );
                registry.register(bridge);
                debug!(
                    server = %server_name,
                    tool = %tool_info.name,
                    "Bridged MCP tool"
                );
            }
        }
    }

    /// Shutdown all MCP servers.
    pub async fn shutdown_all(&mut self) -> Result<()> {
        let names: Vec<String> = self.clients.keys().cloned().collect();
        for name in names {
            if let Some(client) = self.clients.remove(&name) {
                let mut c = client.write().await;
                if let Err(e) = c.shutdown().await {
                    warn!(server = %name, error = %e, "Error shutting down MCP server");
                }
            }
        }
        self.tool_infos.clear();
        Ok(())
    }

    /// Get number of connected servers.
    pub fn server_count(&self) -> usize {
        self.clients.len()
    }
}

impl Default for McpManager {
    fn default() -> Self {
        Self::new()
    }
}
