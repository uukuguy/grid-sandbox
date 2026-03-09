//! Scheduled task execution methods for AgentRuntime.
//! Migrated from AgentLoop to harness::run_agent_loop() (D5 Stage 2).

use std::sync::Arc;

use futures_util::StreamExt;
use octo_types::{ChatMessage, ContentBlock, MessageRole, ToolContext, UserId};

use crate::scheduler::ScheduledTask;

use super::events::AgentEvent;
use super::harness::run_agent_loop;
use super::loop_config::AgentLoopConfig;
use super::runtime::AgentRuntime;
use super::AgentError;

impl AgentRuntime {
    /// Execute a scheduled task: create session, run agent, return result.
    /// Reuses provider/tools/memory from this AgentRuntime.
    pub async fn execute_scheduled_task(&self, task: &ScheduledTask) -> Result<String, AgentError> {
        let config = &task.agent_config;

        // Create session for the task using the session store
        let user_id = task
            .user_id
            .as_ref()
            .map(|u| UserId::from_string(u.clone()))
            .unwrap_or_else(|| UserId::from_string("scheduler".to_string()));

        let session = self.session_store.create_session_with_user(&user_id).await;
        let session_id = session.session_id.clone();
        let sandbox_id = session.sandbox_id.clone();

        // Prepare initial message with the task input
        let user_message = ChatMessage::user(config.input.clone());
        let messages = vec![user_message];

        // Create tool context with security policy for path validation
        let tool_ctx = ToolContext {
            sandbox_id: sandbox_id.clone(),
            working_dir: self.working_dir.clone(),
            path_validator: Some(
                self.security_policy.clone() as std::sync::Arc<dyn octo_types::PathValidator>,
            ),
        };

        // Create tool snapshot
        let tools = {
            let tools_guard = self.tools.lock().unwrap_or_else(|e| e.into_inner());
            Arc::new(tools_guard.snapshot())
        };

        // Build AgentLoopConfig for the harness
        let loop_config = AgentLoopConfig {
            provider: Some(self.provider.clone()),
            tools: Some(tools),
            memory: Some(self.memory.clone()),
            model: config.model.clone(),
            session_id,
            user_id,
            sandbox_id,
            tool_ctx: Some(tool_ctx),
            ..AgentLoopConfig::default()
        };

        // Run agent via harness with timeout
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(config.timeout_secs),
            Self::collect_harness_response(loop_config, messages),
        )
        .await;

        match result {
            Ok(Ok(response)) => {
                tracing::info!(
                    task_id = %task.id,
                    "Scheduled task completed successfully"
                );
                Ok(response)
            }
            Ok(Err(e)) => {
                tracing::error!(task_id = %task.id, error = %e, "Agent execution error");
                Err(AgentError::ScheduledTask(e.to_string()))
            }
            Err(_) => {
                tracing::error!(task_id = %task.id, "Agent execution timed out");
                Err(AgentError::ScheduledTask(format!(
                    "Timeout after {} seconds",
                    config.timeout_secs
                )))
            }
        }
    }

    /// Run the harness and extract the assistant response from final_messages.
    async fn collect_harness_response(
        config: AgentLoopConfig,
        messages: Vec<ChatMessage>,
    ) -> Result<String, String> {
        let mut stream = run_agent_loop(config, messages);
        let mut final_messages: Option<Vec<ChatMessage>> = None;
        let mut last_error: Option<String> = None;

        while let Some(event) = stream.next().await {
            match event {
                AgentEvent::Completed(result) => {
                    final_messages = Some(result.final_messages);
                }
                AgentEvent::Error { message } => {
                    last_error = Some(message);
                }
                _ => {}
            }
        }

        if let Some(err) = last_error {
            if final_messages.is_none() {
                return Err(err);
            }
        }

        let msgs = final_messages.unwrap_or_default();
        let response = msgs
            .iter()
            .rev()
            .find(|m| m.role == MessageRole::Assistant)
            .and_then(|m| {
                m.content.iter().find_map(|c| {
                    if let ContentBlock::Text { text } = c {
                        Some(text.clone())
                    } else {
                        None
                    }
                })
            })
            .unwrap_or_else(|| "Task completed".to_string());

        Ok(response)
    }
}
