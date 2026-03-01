use anyhow::Result;
use async_trait::async_trait;
use tracing::info;

use rmcp::model::{CallToolRequestParams, RawContent};
use rmcp::service::RunningService;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::{RoleClient, ServiceExt};

use super::traits::{McpClient, McpToolInfo};

/// MCP client using Streamable HTTP (SSE) transport.
/// Connects to a remote MCP server via HTTP URL.
pub struct SseMcpClient {
    name: String,
    url: String,
    service: Option<RunningService<RoleClient, ()>>,
}

impl SseMcpClient {
    pub fn new(name: String, url: String) -> Self {
        Self {
            name,
            url,
            service: None,
        }
    }
}

#[async_trait]
impl McpClient for SseMcpClient {
    fn name(&self) -> &str {
        &self.name
    }

    async fn connect(&mut self) -> Result<()> {
        info!(
            name = %self.name,
            url = %self.url,
            "Connecting to remote MCP server via SSE"
        );

        let transport = StreamableHttpClientTransport::from_uri(self.url.clone());

        let service = ()
            .serve(transport)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to connect to MCP server at {}: {e}", self.url))?;

        let peer_info = service.peer_info();
        info!(
            name = %self.name,
            server = ?peer_info,
            "Remote MCP server connected"
        );

        self.service = Some(service);
        Ok(())
    }

    async fn list_tools(&self) -> Result<Vec<McpToolInfo>> {
        let service = self
            .service
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("SSE MCP client not connected"))?;

        let tools = service
            .list_all_tools()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list SSE MCP tools: {e}"))?;

        Ok(tools
            .into_iter()
            .map(|t| McpToolInfo {
                name: t.name.to_string(),
                description: t.description.map(|d| d.to_string()),
                input_schema: serde_json::Value::Object(t.input_schema.as_ref().clone()),
            })
            .collect())
    }

    async fn call_tool(&self, name: &str, args: serde_json::Value) -> Result<serde_json::Value> {
        let service = self
            .service
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("SSE MCP client not connected"))?;

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
            .map_err(|e| anyhow::anyhow!("Failed to call SSE MCP tool '{name}': {e}"))?;

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
            info!(name = %self.name, "Disconnecting remote MCP server");
            service
                .cancel()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to cancel SSE MCP service: {e}"))?;
        }
        Ok(())
    }
}
