//! `execute_skill` tool — allows the Agent to execute a skill by name.
//!
//! For KNOWLEDGE skills, returns the skill body as guidance text.
//! For PLAYBOOK skills, spawns a SubAgent via SubAgentRuntime (Phase AY convergence).

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use tokio::sync::broadcast;

use octo_types::skill::ExecutionMode;
use octo_types::{ToolContext, ToolOutput, ToolSource};

use crate::agent::entry::AgentManifest;
use crate::agent::events::AgentEvent;
use crate::agent::loop_config::AgentLoopConfig;
use crate::agent::subagent::SubAgentManager;
use crate::agent::subagent_runtime::SubAgentRuntime;
use crate::providers::Provider;
use crate::skills::constraint::ToolConstraintEnforcer;
use crate::skills::registry::SkillRegistry;
use crate::tools::traits::Tool;
use crate::tools::ToolRegistry;

/// Context needed to spawn SubAgent for playbook skill execution.
/// Avoids circular dependency with AgentLoopConfig (which contains tools).
pub struct SubAgentContext {
    pub manager: Arc<SubAgentManager>,
    pub provider: Arc<dyn Provider>,
    pub tools: Arc<ToolRegistry>,
    pub model: String,
    pub user_id: octo_types::UserId,
    pub sandbox_id: octo_types::SandboxId,
    pub tool_ctx: Option<octo_types::ToolContext>,
    /// Optional broadcast sender to forward sub-agent streaming events to the
    /// parent agent's event stream, enabling real-time TUI rendering.
    pub event_sender: Option<broadcast::Sender<AgentEvent>>,
}

/// Tool that executes a skill by name.
///
/// - KNOWLEDGE: returns the skill body as instructions for the agent to follow.
/// - PLAYBOOK: spawns an isolated SubAgent via SubAgentRuntime that follows the
///   skill's instructions with a constrained tool set.
pub struct ExecuteSkillTool {
    skill_registry: Arc<SkillRegistry>,
    subagent_ctx: Option<SubAgentContext>,
}

impl ExecuteSkillTool {
    pub fn new(skill_registry: Arc<SkillRegistry>) -> Self {
        Self {
            skill_registry,
            subagent_ctx: None,
        }
    }

    /// Configure SubAgent execution context for playbook skills.
    pub fn with_subagent_ctx(mut self, ctx: SubAgentContext) -> Self {
        self.subagent_ctx = Some(ctx);
        self
    }
}

#[async_trait]
impl Tool for ExecuteSkillTool {
    fn name(&self) -> &str {
        "execute_skill"
    }

    fn description(&self) -> &str {
        "Execute a skill by name. For knowledge skills, returns guidance instructions. \
         For playbook skills, spawns an isolated sub-agent to execute the skill's operations."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "required": ["skill_name", "request"],
            "properties": {
                "skill_name": {
                    "type": "string",
                    "description": "Name of the skill to execute"
                },
                "request": {
                    "type": "string",
                    "description": "Natural language description of what you want the skill to do"
                }
            }
        })
    }

    fn risk_level(&self) -> octo_types::RiskLevel {
        octo_types::RiskLevel::HighRisk
    }

    fn source(&self) -> ToolSource {
        ToolSource::BuiltIn
    }

    async fn execute(&self, params: serde_json::Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        let skill_name = params["skill_name"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let request = params["request"]
            .as_str()
            .unwrap_or("")
            .to_string();

        if skill_name.is_empty() {
            return Ok(ToolOutput::error("Missing required parameter: skill_name"));
        }
        if request.is_empty() {
            return Ok(ToolOutput::error("Missing required parameter: request"));
        }

        // Look up the skill
        let skill = match self.skill_registry.get(&skill_name) {
            Some(s) => s,
            None => {
                // List available skills for helpful error
                let available: Vec<String> = self
                    .skill_registry
                    .list_all()
                    .iter()
                    .map(|s| s.name.clone())
                    .collect();
                return Ok(ToolOutput::error(format!(
                    "Skill '{}' not found. Available skills: {}",
                    skill_name,
                    available.join(", ")
                )));
            }
        };

        match skill.execution_mode {
            ExecutionMode::Knowledge => {
                // Return the skill body as guidance text
                let output = if skill.body.is_empty() {
                    format!(
                        "Skill '{}' activated: {}\n\n(No additional instructions provided)",
                        skill.name, skill.description
                    )
                } else {
                    format!(
                        "## Skill: {}\n\n{}\n\n---\nNow apply these instructions to: {}",
                        skill.name, skill.body, request
                    )
                };
                Ok(ToolOutput::success(output))
            }
            ExecutionMode::Playbook => self.execute_playbook(&skill, &request).await,
        }
    }
}

impl ExecuteSkillTool {
    /// Execute a playbook skill via SubAgentRuntime (Phase AY convergence).
    ///
    /// Builds an AgentManifest from the skill definition, then delegates to
    /// SubAgentRuntime for the complete lifecycle (build → run → cleanup).
    async fn execute_playbook(
        &self,
        skill: &octo_types::SkillDefinition,
        request: &str,
    ) -> Result<ToolOutput> {
        let ctx = match &self.subagent_ctx {
            Some(c) => c,
            None => {
                return Ok(ToolOutput::error(
                    "Cannot execute playbook skill: no SubAgent manager configured",
                ));
            }
        };

        // Build system prompt from skill body
        let system_prompt = format!(
            "You are executing the '{}' skill.\n\n{}\n\n## Your Task\n{}",
            skill.name,
            if skill.body.is_empty() {
                &skill.description
            } else {
                &skill.body
            },
            request
        );

        // Build filtered tool registry based on skill's allowed_tools
        let tools = if skill.allowed_tools.is_some() {
            let enforcer = ToolConstraintEnforcer::from_active_skills(&[skill.clone()]);
            let all_names: Vec<String> = ctx.tools.names();
            let filtered = enforcer.filter_tools(&all_names);
            Some(Arc::new(ctx.tools.snapshot_filtered(&filtered)))
        } else {
            Some(ctx.tools.clone())
        };

        // Build a manifest from skill definition
        let manifest = AgentManifest {
            name: format!("skill-{}", skill.name),
            system_prompt: Some(system_prompt),
            model: skill.model.clone(),
            background: skill.background,
            ..Default::default()
        };

        // Build parent config for SubAgentRuntime
        let parent_config = AgentLoopConfig {
            max_iterations: if skill.max_rounds > 0 {
                skill.max_rounds
            } else {
                30
            },
            provider: Some(ctx.provider.clone()),
            tools,
            memory: None, // Isolated — no shared memory
            model: ctx.model.clone(),
            user_id: ctx.user_id.clone(),
            sandbox_id: ctx.sandbox_id.clone(),
            tool_ctx: ctx.tool_ctx.clone(),
            ..AgentLoopConfig::default()
        };

        // Build SubAgentRuntime
        let runtime = match SubAgentRuntime::build(
            request.to_string(),
            Some(manifest),
            &parent_config,
            ctx.manager.clone(),
            ctx.event_sender.clone(),
            None,
        )
        .await
        {
            Ok(rt) => rt,
            Err(e) => {
                return Ok(ToolOutput::error(format!(
                    "Cannot spawn skill sub-agent: {e}"
                )));
            }
        };

        if skill.background {
            // Background: fire-and-forget
            let session_id = runtime.run_async();
            Ok(ToolOutput::success(format!(
                "Skill '{}' launched in background. Use query_agent with session_id '{}' to check status.",
                skill.name, session_id
            )))
        } else {
            // Foreground: wait for result
            let result = runtime.run_sync().await?;
            if result.output.is_empty() {
                Ok(ToolOutput::error(format!(
                    "Skill '{}' produced no output",
                    skill.name
                )))
            } else {
                Ok(ToolOutput::success(format!(
                    "## Skill '{}' Result (iterations: {})\n\n{}",
                    skill.name, result.rounds, result.output
                )))
            }
        }
    }
}
