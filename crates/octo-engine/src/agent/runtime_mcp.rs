//! MCP server management methods for AgentRuntime.

use std::sync::Arc;

use octo_types::ToolSource;

use crate::mcp::traits::McpToolInfo;

use super::runtime::AgentRuntime;
use super::AgentError;

impl AgentRuntime {
    /// 添加 MCP Server → 自动注册 tools
    pub async fn add_mcp_server(
        &self,
        config: crate::mcp::traits::McpServerConfig,
    ) -> Result<Vec<McpToolInfo>, AgentError> {
        let mcp = &self.mcp_manager;

        let tools = {
            let mut guard = mcp.lock().await;
            guard
                .add_server(config.clone())
                .await
                .map_err(|e| AgentError::McpError(e.to_string()))?
        };

        // 注册到 ToolRegistry
        {
            let mcp_guard = mcp.lock().await;
            let mut tools_guard = self.tools.lock().unwrap_or_else(|e| e.into_inner());
            for tool_info in &tools {
                if let Some(client) = mcp_guard.clients().get(&config.name) {
                    let bridge = crate::mcp::bridge::McpToolBridge::new(
                        client.clone(),
                        config.name.clone(),
                        tool_info.clone(),
                    );
                    tools_guard.register(bridge);
                }
            }
        }

        Ok(tools)
    }

    /// 移除 MCP Server → 自动注销 tools
    pub async fn remove_mcp_server(&self, name: &str) -> Result<(), AgentError> {
        let mcp = &self.mcp_manager;

        // 先获取要移除的 tools 信息
        let _removed_tool_names: Vec<String> = {
            let guard = mcp.lock().await;
            guard
                .get_tool_infos(name)
                .map(|tools| tools.iter().map(|t| t.name.clone()).collect())
                .unwrap_or_default()
        };

        // 调用 remove_server
        {
            let mut guard = mcp.lock().await;
            guard
                .remove_server(name)
                .await
                .map_err(|e| AgentError::McpError(e.to_string()))?;
        }

        // 从 ToolRegistry 注销
        // 由于 ToolRegistry 没有 unregister 方法，我们重新构建工具列表
        // 过滤掉属于该 MCP server 的工具
        let all_tools: Vec<(String, Arc<dyn crate::tools::Tool>)> = {
            let tools_guard = self.tools.lock().unwrap_or_else(|e| e.into_inner());
            tools_guard
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        };
        let mut new_registry = crate::tools::ToolRegistry::new();
        for (tool_name, tool) in all_tools {
            // 检查工具来源是否为该 MCP server，使用模式匹配
            let should_remove = match tool.source() {
                ToolSource::Mcp(server_name) => server_name == name,
                _ => false,
            };
            if should_remove {
                continue; // 跳过要移除的工具
            }
            new_registry.register_arc(tool_name, tool);
        }
        // 替换旧的 registry
        let mut tools_guard = self.tools.lock().unwrap_or_else(|e| e.into_inner());
        *tools_guard = new_registry;

        Ok(())
    }

    /// 列出运行中的 MCP servers
    pub async fn list_mcp_servers(&self) -> Vec<crate::mcp::manager::ServerRuntimeState> {
        let guard = self.mcp_manager.lock().await;
        let states = guard.all_runtime_states();
        states.into_iter().map(|(_, state)| state).collect()
    }

    /// 获取所有 MCP servers 的运行时状态（包含名称）
    pub async fn get_all_mcp_server_states(
        &self,
    ) -> std::collections::HashMap<String, crate::mcp::manager::ServerRuntimeState> {
        let guard = self.mcp_manager.lock().await;
        guard.all_runtime_states()
    }

    /// 获取指定 MCP server 的 tools
    pub async fn get_mcp_tool_infos(
        &self,
        server_id: &str,
    ) -> Vec<crate::mcp::traits::McpToolInfo> {
        let guard = self.mcp_manager.lock().await;
        guard.get_tool_infos(server_id).unwrap_or_default()
    }

    /// 调用 MCP tool
    pub async fn call_mcp_tool(
        &self,
        server_id: &str,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let guard = self.mcp_manager.lock().await;
        guard
            .call_tool(server_id, tool_name, arguments)
            .await
            .map_err(|e| e.to_string())
    }

    /// 获取指定 MCP server 的运行时状态
    pub async fn get_mcp_runtime_state(
        &self,
        server_id: &str,
    ) -> crate::mcp::manager::ServerRuntimeState {
        let guard = self.mcp_manager.lock().await;
        guard.get_runtime_state(server_id)
    }

    /// 获取指定 MCP server 的 tool 数量
    pub async fn get_mcp_tool_count(&self, server_id: &str) -> usize {
        let guard = self.mcp_manager.lock().await;
        guard.get_tool_count(server_id)
    }
}
