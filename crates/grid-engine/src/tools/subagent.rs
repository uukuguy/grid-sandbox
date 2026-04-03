//! Agent tools — spawn and query sub-agents via SubAgentRuntime (Phase AY).
//!
//! Renamed from SpawnSubAgentTool/QuerySubAgentTool to AgentTool/QueryAgentTool
//! for CC-OSS alignment.

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use tokio::sync::broadcast;

use grid_types::{RiskLevel, ToolContext, ToolOutput, ToolProgress, ToolSource};

use crate::agent::catalog::AgentCatalog;
use crate::agent::entry::AgentManifest;
use crate::agent::events::AgentEvent;
use crate::agent::loop_config::AgentLoopConfig;
use crate::agent::subagent::{SubAgentManager, SubAgentStatus};
use crate::agent::subagent_runtime::SubAgentRuntime;
use crate::skills::SkillRegistry;

use super::traits::Tool;

/// Tool that spawns a sub-agent using SubAgentRuntime.
///
/// Renamed from SpawnSubAgentTool → AgentTool for CC-OSS alignment (Phase AY).
/// Tool name: "agent" (was "spawn_subagent").
pub struct AgentTool {
    subagent_manager: Arc<SubAgentManager>,
    /// Template config cloned for each child agent.
    parent_config: Arc<AgentLoopConfig>,
    /// Agent catalog for looking up built-in/YAML agent definitions (Phase AX).
    catalog: Option<Arc<AgentCatalog>>,
    /// Skill registry for preloading skill content into agent system prompts.
    skill_registry: Option<Arc<SkillRegistry>>,
    /// Dynamic description including agent listing from catalog.
    dynamic_description: String,
    /// Event sender for forwarding sub-agent streaming events to parent.
    event_sender: Option<broadcast::Sender<AgentEvent>>,
}

impl AgentTool {
    pub fn new(manager: Arc<SubAgentManager>, config: Arc<AgentLoopConfig>) -> Self {
        Self {
            subagent_manager: manager,
            parent_config: config,
            catalog: None,
            skill_registry: None,
            dynamic_description: super::prompts::SUBAGENT_DESCRIPTION.to_string(),
            event_sender: None,
        }
    }

    /// Attach an agent catalog for agent_type lookup.
    /// Also rebuilds the dynamic description with agent listings.
    pub fn with_catalog(mut self, catalog: Arc<AgentCatalog>) -> Self {
        self.catalog = Some(catalog.clone());
        self.rebuild_description(&catalog);
        self
    }

    /// Rebuild description with dynamic agent listing from catalog.
    fn rebuild_description(&mut self, catalog: &AgentCatalog) {
        use crate::agent::builtin_agents::builtin_agent_manifests;

        let mut lines = Vec::new();
        for manifest in builtin_agent_manifests() {
            if let Some(ref when) = manifest.when_to_use {
                let tools_desc = if !manifest.disallowed_tools.is_empty() {
                    format!(
                        "All tools except {}",
                        manifest.disallowed_tools.join(", ")
                    )
                } else if !manifest.tool_filter.is_empty() {
                    manifest.tool_filter.join(", ")
                } else {
                    "All tools".to_string()
                };
                lines.push(format!(
                    "- {}: {} (Tools: {})",
                    manifest.name, when, tools_desc
                ));
            }
        }

        // Also include YAML-loaded agents from catalog (non-builtin)
        for entry in catalog.list_all() {
            if entry.manifest.source != crate::agent::entry::AgentSource::BuiltIn {
                if let Some(ref when) = entry.manifest.when_to_use {
                    lines.push(format!("- {}: {}", entry.manifest.name, when));
                }
            }
        }

        if lines.is_empty() {
            return; // Keep static description
        }

        self.dynamic_description = format!(
            r#"Launch a new agent to handle complex, multi-step tasks autonomously.

The Agent tool launches specialized agents that autonomously handle complex tasks. Each agent type has specific capabilities and tools available to it.

Available agent types and the tools they have access to:
{}

When using the Agent tool, specify a subagent_type parameter to select which agent type to use. If omitted, the general-purpose agent is used.

When NOT to use the Agent tool:
- If you want to read a specific file path, use file_read directly
- If you are searching for a specific class definition, use grep/glob directly
- If you are searching for code within 2-3 specific files, use file_read directly
- Tasks that need <3 tool calls — do it yourself

Usage notes:
- Always include a short description (3-5 words) summarizing what the agent will do
- The agent starts with zero context. Write prompts like briefing a colleague who just walked in.
- Explain WHAT you want and WHY, describe what you've already learned or ruled out.
- The agent's outputs should generally be trusted
- Clearly tell the agent whether you expect it to write code or just to do research"#,
            lines.join("\n")
        );
    }

    /// Attach a skill registry for preloading skills into agent system prompts.
    pub fn with_skill_registry(mut self, registry: Arc<SkillRegistry>) -> Self {
        self.skill_registry = Some(registry);
        self
    }

    /// Attach an event sender for forwarding sub-agent streaming events.
    pub fn with_event_sender(mut self, sender: broadcast::Sender<AgentEvent>) -> Self {
        self.event_sender = Some(sender);
        self
    }
}

#[async_trait]
impl Tool for AgentTool {
    fn name(&self) -> &str {
        "agent"
    }

    fn description(&self) -> &str {
        &self.dynamic_description
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "required": ["task"],
            "properties": {
                "task": {
                    "type": "string",
                    "description": "Description of the task for the sub-agent"
                },
                "agent_type": {
                    "type": "string",
                    "description": "Optional agent type name. When provided, uses the agent's \
                        configured tools, model, and system prompt from the agent catalog. \
                        Built-in types: general-purpose, explore, plan, coder, reviewer, verification"
                },
                "max_iterations": {
                    "type": "integer",
                    "description": "Max LLM iterations for the sub-agent (default: 10)"
                },
                "tools_whitelist": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Optional list of tool names the sub-agent can use (overridden when agent_type is provided)"
                }
            }
        })
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::HighRisk
    }

    fn source(&self) -> ToolSource {
        ToolSource::BuiltIn
    }

    async fn execute(&self, params: serde_json::Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        let task = params["task"]
            .as_str()
            .unwrap_or("No task specified")
            .to_string();

        let agent_type = params["agent_type"].as_str().map(String::from);

        // Resolve manifest from agent_type (if provided)
        let manifest: Option<AgentManifest> = if let Some(ref at) = agent_type {
            match &self.catalog {
                Some(catalog) => catalog.get_by_name(at).map(|e| e.manifest),
                None => {
                    tracing::warn!(agent_type = %at, "agent_type specified but no catalog available");
                    None
                }
            }
        } else {
            None
        };

        // Build SubAgentRuntime
        let is_background = manifest.as_ref().map_or(false, |m| m.background);
        let runtime = match SubAgentRuntime::build(
            task,
            manifest,
            &self.parent_config,
            self.subagent_manager.clone(),
            self.event_sender.clone(),
            self.skill_registry.as_ref(),
        )
        .await
        {
            Ok(rt) => rt,
            Err(e) => return Ok(ToolOutput::error(format!("Cannot spawn agent: {e}"))),
        };

        let session_id = runtime.id.clone();
        let depth = self.subagent_manager.depth() + 1;

        if is_background {
            // Async: fire-and-forget
            runtime.run_async();
            Ok(ToolOutput::success(
                json!({
                    "session_id": session_id,
                    "status": "spawned",
                    "agent_type": agent_type,
                    "depth": depth,
                })
                .to_string(),
            ))
        } else {
            // Sync: wait for result (default)
            let result = runtime.run_sync().await?;
            if result.output.is_empty() {
                Ok(ToolOutput::error("Agent produced no output"))
            } else {
                Ok(ToolOutput::success(result.output))
            }
        }
    }

    async fn execute_with_progress(
        &self,
        params: serde_json::Value,
        ctx: &ToolContext,
        on_progress: Option<super::traits::ProgressCallback>,
    ) -> Result<ToolOutput> {
        if let Some(ref cb) = on_progress {
            cb(ToolProgress::indeterminate("spawning agent..."));
        }
        let result = self.execute(params, ctx).await;
        if let Some(ref cb) = on_progress {
            cb(ToolProgress::percent(1.0, "agent completed"));
        }
        result
    }
}

/// Tool that queries the status/result of a previously spawned sub-agent.
///
/// Renamed from QuerySubAgentTool → QueryAgentTool (Phase AY).
/// Tool name: "query_agent" (was "query_subagent").
pub struct QueryAgentTool {
    subagent_manager: Arc<SubAgentManager>,
}

impl QueryAgentTool {
    pub fn new(manager: Arc<SubAgentManager>) -> Self {
        Self {
            subagent_manager: manager,
        }
    }
}

#[async_trait]
impl Tool for QueryAgentTool {
    fn name(&self) -> &str {
        "query_agent"
    }

    fn description(&self) -> &str {
        "Query the status and result of a previously spawned agent."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "required": ["session_id"],
            "properties": {
                "session_id": {
                    "type": "string",
                    "description": "The session_id returned by the agent tool"
                }
            }
        })
    }

    fn source(&self) -> ToolSource {
        ToolSource::BuiltIn
    }

    async fn execute(&self, params: serde_json::Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        let session_id = params["session_id"]
            .as_str()
            .unwrap_or("")
            .to_string();

        if session_id.is_empty() {
            return Ok(ToolOutput::error("Missing session_id parameter"));
        }

        let agents = self.subagent_manager.list().await;
        if let Some(handle) = agents.iter().find(|a| a.id == session_id) {
            let status_str = match &handle.status {
                SubAgentStatus::Running => "running",
                SubAgentStatus::Completed => "completed",
                SubAgentStatus::Failed(_) => "failed",
                SubAgentStatus::Cancelled => "cancelled",
            };

            let error_msg = if let SubAgentStatus::Failed(e) = &handle.status {
                Some(e.clone())
            } else {
                None
            };

            Ok(ToolOutput::success(
                json!({
                    "session_id": session_id,
                    "status": status_str,
                    "description": handle.description,
                    "error": error_msg,
                })
                .to_string(),
            ))
        } else {
            Ok(ToolOutput::error(
                json!({
                    "session_id": session_id,
                    "status": "not_found",
                })
                .to_string(),
            ))
        }
    }

    async fn execute_with_progress(
        &self,
        params: serde_json::Value,
        ctx: &ToolContext,
        on_progress: Option<super::traits::ProgressCallback>,
    ) -> Result<ToolOutput> {
        if let Some(ref cb) = on_progress {
            cb(ToolProgress::indeterminate("querying agent..."));
        }
        self.execute(params, ctx).await
    }
}

// Backward compatibility aliases
pub type SpawnSubAgentTool = AgentTool;
pub type QuerySubAgentTool = QueryAgentTool;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::catalog::AgentCatalog;
    use crate::agent::entry::{AgentManifest, AgentSource};
    use crate::agent::subagent_runtime::SubAgentRuntime;
    use crate::tools::ToolRegistry;

    fn make_registry() -> ToolRegistry {
        let mut reg = ToolRegistry::new();
        reg.register(crate::tools::sleep::SleepTool);
        reg.register(crate::tools::doctor::DoctorTool);
        reg.register(crate::tools::notifier::NotifierTool);
        reg
    }

    #[test]
    fn test_tool_name_is_agent() {
        let mgr = Arc::new(SubAgentManager::new(4, 3));
        let config = Arc::new(AgentLoopConfig::default());
        let tool = AgentTool::new(mgr, config);
        assert_eq!(tool.name(), "agent");
    }

    #[test]
    fn test_query_tool_name_is_query_agent() {
        let mgr = Arc::new(SubAgentManager::new(4, 3));
        let tool = QueryAgentTool::new(mgr);
        assert_eq!(tool.name(), "query_agent");
    }

    #[test]
    fn test_with_catalog_lookup() {
        let catalog = AgentCatalog::new();
        catalog.register(
            AgentManifest {
                name: "my-agent".into(),
                system_prompt: Some("Hello".into()),
                source: AgentSource::BuiltIn,
                ..Default::default()
            },
            None,
        );

        let config = AgentLoopConfig::default();
        let mgr = Arc::new(SubAgentManager::new(4, 3));
        let tool =
            AgentTool::new(mgr, Arc::new(config)).with_catalog(Arc::new(catalog));

        assert!(tool.catalog.is_some());
    }

    #[test]
    fn test_backward_compat_aliases() {
        // Verify type aliases compile
        let mgr = Arc::new(SubAgentManager::new(4, 3));
        let config = Arc::new(AgentLoopConfig::default());
        let _spawn: SpawnSubAgentTool = AgentTool::new(mgr.clone(), config);
        let _query: QuerySubAgentTool = QueryAgentTool::new(mgr);
    }
}
