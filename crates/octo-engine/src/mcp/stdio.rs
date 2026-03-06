use anyhow::{Context, Result};
use async_trait::async_trait;
use tracing::{debug, info};

use rmcp::model::{CallToolRequestParams, RawContent};
use rmcp::service::RunningService;
use rmcp::transport::{ConfigureCommandExt, TokioChildProcess};
use rmcp::{RoleClient, ServiceExt};

use super::traits::{McpClient, McpServerConfig, McpToolInfo};

pub struct StdioMcpClient {
    config: McpServerConfig,
    service: Option<RunningService<RoleClient, ()>>,
}

impl StdioMcpClient {
    pub fn new(config: McpServerConfig) -> Self {
        Self {
            config,
            service: None,
        }
    }
}

#[async_trait]
impl McpClient for StdioMcpClient {
    fn name(&self) -> &str {
        &self.config.name
    }

    async fn connect(&mut self) -> Result<()> {
        let config = &self.config;
        info!(
            name = %config.name,
            command = %config.command,
            "Connecting to MCP server"
        );

        let env = config.env.clone();
        let args = config.args.clone();

        let transport = TokioChildProcess::new(
            tokio::process::Command::new(&config.command).configure(move |c| {
                for arg in &args {
                    c.arg(arg);
                }
                c.env_clear();
                for (k, v) in &env {
                    c.env(k, v);
                }
            }),
        )
        .context("Failed to spawn MCP server process")?;

        let service = ()
            .serve(transport)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to initialize MCP connection: {e}"))?;

        let peer_info = service.peer_info();
        info!(
            name = %config.name,
            server = ?peer_info,
            "MCP server connected"
        );

        self.service = Some(service);
        Ok(())
    }

    async fn list_tools(&self) -> Result<Vec<McpToolInfo>> {
        let service = self
            .service
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("MCP client not connected"))?;

        let tools = service
            .list_all_tools()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list MCP tools: {e}"))?;

        let result: Vec<McpToolInfo> = tools
            .into_iter()
            .map(|t| McpToolInfo {
                name: t.name.to_string(),
                description: t.description.map(|d| d.to_string()),
                input_schema: serde_json::Value::Object(t.input_schema.as_ref().clone()),
            })
            .collect();

        debug!(count = result.len(), "Listed MCP tools");
        Ok(result)
    }

    async fn call_tool(&self, name: &str, args: serde_json::Value) -> Result<serde_json::Value> {
        let service = self
            .service
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("MCP client not connected"))?;

        let arguments = if args.is_object() {
            Some(args.as_object().unwrap().clone())
        } else {
            None
        };

        let result = service
            .call_tool(CallToolRequestParams {
                meta: None,
                name: name.to_string().into(),
                arguments,
                task: None,
            })
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call MCP tool '{name}': {e}"))?;

        // Convert result content to JSON
        let content_strs: Vec<String> = result
            .content
            .into_iter()
            .filter_map(|c| match &c.raw {
                RawContent::Text(text) => Some(text.text.clone()),
                _ => None,
            })
            .collect();

        Ok(serde_json::json!({
            "content": content_strs.join("\n"),
            "isError": result.is_error.unwrap_or(false),
        }))
    }

    fn is_connected(&self) -> bool {
        self.service.is_some()
    }

    async fn shutdown(&mut self) -> Result<()> {
        if let Some(service) = self.service.take() {
            info!(name = %self.config.name, "Shutting down MCP server");
            service
                .cancel()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to cancel MCP service: {e}"))?;
        }
        Ok(())
    }
}
