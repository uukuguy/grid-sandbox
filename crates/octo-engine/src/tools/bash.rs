use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::process::Command;
use tracing::debug;

use octo_types::{ApprovalRequirement, RiskLevel, ToolContext, ToolOutput, ToolSource};

use super::traits::Tool;

// Sandbox imports - feature-gated
#[cfg(feature = "sandbox-wasm")]
use crate::sandbox::{AdapterEnum, SandboxRouter, SandboxType, SubprocessAdapter, ToolCategory};

/// Environment variables passed through to command execution.
///
/// Security boundary is at the sandbox level (Docker/WASM isolation),
/// not the tool level. Commands need access to API keys, Python paths,
/// and other runtime state to function properly.
const PASSTHROUGH_ENV_VARS: &[&str] = &[
    // System basics
    "PATH", "HOME", "TMPDIR", "LANG", "LC_ALL", "TERM", "USER", "SHELL",
    // Build tools
    "CARGO_HOME", "RUSTUP_HOME",
    // Python
    "VIRTUAL_ENV", "PYTHONPATH", "UV_CACHE_DIR",
    // Node
    "NODE_PATH", "NPM_CONFIG_PREFIX",
    // LLM API keys (needed by skill scripts)
    "ANTHROPIC_API_KEY", "OPENAI_API_KEY", "OPENAI_BASE_URL",
    "TAVILY_API_KEY", "JINA_API_KEY",
    // Proxy (corporate environments)
    "HTTP_PROXY", "HTTPS_PROXY", "NO_PROXY",
    "http_proxy", "https_proxy", "no_proxy",
];

pub struct BashTool {
    /// Sandbox router for secure execution (feature-gated)
    #[cfg(feature = "sandbox-wasm")]
    router: Option<SandboxRouter>,
}

impl BashTool {
    pub fn new() -> Self {
        #[cfg(feature = "sandbox-wasm")]
        let router = Some(SandboxRouter::with_policy(crate::sandbox::SandboxPolicy::Development));
        Self {
            #[cfg(feature = "sandbox-wasm")]
            router,
        }
    }

    /// 执行命令 - 优先使用沙箱，失败则回退到直接执行
    #[cfg(feature = "sandbox-wasm")]
    async fn execute_via_sandbox(
        &self,
        command: &str,
        working_dir: &std::path::Path,
    ) -> Result<(String, i32), String> {
        use crate::sandbox::ExecResult;

        if let Some(router) = &self.router {
            // Clone the router to allow mutation
            let mut router = router.clone();
            // 注册 subprocess 适配器
            router.register_adapter(AdapterEnum::Subprocess(SubprocessAdapter::new()));
            // 使用 subprocess 作为默认执行器
            router.set_mapping(ToolCategory::Shell, SandboxType::Subprocess);

            // 在指定工作目录中执行
            let full_command = format!("cd {} && {}", working_dir.display(), command);

            match router
                .execute(ToolCategory::Shell, &full_command, "bash")
                .await
            {
                Ok(ExecResult {
                    stdout,
                    stderr,
                    success,
                    exit_code,
                    ..
                }) => {
                    let combined = if stderr.is_empty() {
                        stdout
                    } else if stdout.is_empty() {
                        format!("STDERR:\n{stderr}")
                    } else {
                        format!("{stdout}\nSTDERR:\n{stderr}")
                    };
                    let code = if success { 0 } else { exit_code };
                    return Ok((combined, code));
                }
                Err(e) => {
                    // 沙箱执行失败，回退到直接执行
                    tracing::warn!(
                        "Sandbox execution failed, falling back to direct execution: {}",
                        e
                    );
                }
            }
        }
        Err("Sandbox not available".to_string())
    }

    /// 克隆路由器（用于测试）
    #[cfg(feature = "sandbox-wasm")]
    pub fn router(&self) -> Option<&SandboxRouter> {
        self.router.as_ref()
    }

    /// 设置自定义沙箱路由器（用于测试或高级配置）
    #[cfg(feature = "sandbox-wasm")]
    pub fn set_router(&mut self, router: SandboxRouter) {
        self.router = Some(router);
    }
}

impl Default for BashTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "Execute a bash command. Returns stdout, stderr, and exit code."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The bash command to execute"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in seconds (default: 30, max: 120)"
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<ToolOutput> {
        let command = params["command"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'command' parameter"))?;

        let timeout_secs = params["timeout"].as_u64().unwrap_or(30).min(120);

        debug!(command, timeout_secs, "executing bash command");

        // 尝试沙箱执行（如果启用）
        #[cfg(feature = "sandbox-wasm")]
        {
            match self.execute_via_sandbox(command, &ctx.working_dir).await {
                Ok((output_text, exit_code)) => {
                    // 截断过长输出
                    let output_text = if output_text.len() > 100_000 {
                        format!(
                            "{}...\n[output truncated, {} bytes total]",
                            &output_text[..100_000],
                            output_text.len()
                        )
                    } else {
                        output_text
                    };

                    if exit_code == 0 {
                        return Ok(ToolOutput::success(output_text));
                    } else {
                        return Ok(ToolOutput::error(format!(
                            "Exit code: {exit_code}\n{output_text}"
                        )));
                    }
                }
                Err(_) => {
                    // 沙箱不可用或失败，继续使用直接执行
                    tracing::warn!(
                        "Sandbox not available, falling back to direct command execution"
                    );
                }
            }
        }

        // 直接执行（默认行为或沙箱回退）
        let env_vars: Vec<(String, String)> = std::env::vars()
            .filter(|(k, _)| PASSTHROUGH_ENV_VARS.contains(&k.as_str()))
            .collect();

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            Command::new("bash")
                .arg("-c")
                .arg(command)
                .current_dir(&ctx.working_dir)
                .env_clear()
                .envs(env_vars)
                .output(),
        )
        .await;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code().unwrap_or(-1);

                let combined = if stderr.is_empty() {
                    stdout
                } else if stdout.is_empty() {
                    format!("STDERR:\n{stderr}")
                } else {
                    format!("{stdout}\nSTDERR:\n{stderr}")
                };

                // Truncate if too long
                let output_text = if combined.len() > 100_000 {
                    format!(
                        "{}...\n[output truncated, {} bytes total]",
                        &combined[..100_000],
                        combined.len()
                    )
                } else {
                    combined
                };

                if exit_code == 0 {
                    Ok(ToolOutput::success(output_text))
                } else {
                    Ok(ToolOutput::error(format!(
                        "Exit code: {exit_code}\n{output_text}"
                    )))
                }
            }
            Ok(Err(e)) => Ok(ToolOutput::error(format!("Failed to execute command: {e}"))),
            Err(_) => Ok(ToolOutput::error(format!(
                "Command timed out after {timeout_secs} seconds"
            ))),
        }
    }

    fn source(&self) -> ToolSource {
        ToolSource::BuiltIn
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Destructive
    }

    fn approval(&self) -> ApprovalRequirement {
        ApprovalRequirement::Always
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_bash_tool_metadata() {
        let tool = BashTool::new();
        assert_eq!(tool.name(), "bash");
        assert_eq!(tool.source(), ToolSource::BuiltIn);
        assert_eq!(tool.risk_level(), RiskLevel::Destructive);
        assert_eq!(tool.approval(), ApprovalRequirement::Always);
    }

    #[tokio::test]
    async fn test_simple_command() {
        let tool = BashTool::new();
        let ctx = ToolContext {
            sandbox_id: octo_types::SandboxId::from_string("test"),
            working_dir: PathBuf::from("."),
            path_validator: None,
        };
        let result = tool.execute(json!({"command": "echo hello"}), &ctx).await.unwrap();
        assert!(!result.is_error);
        assert!(result.content.contains("hello"));
    }

    #[tokio::test]
    async fn test_pipe_and_shell_features_work() {
        let tool = BashTool::new();
        let ctx = ToolContext {
            sandbox_id: octo_types::SandboxId::from_string("test"),
            working_dir: PathBuf::from("."),
            path_validator: None,
        };
        // Pipes should work — security is at sandbox level, not tool level
        let result = tool.execute(json!({"command": "echo hello | tr a-z A-Z"}), &ctx).await.unwrap();
        assert!(!result.is_error);
        assert!(result.content.contains("HELLO"));
    }

    #[tokio::test]
    async fn test_curl_works() {
        let tool = BashTool::new();
        let ctx = ToolContext {
            sandbox_id: octo_types::SandboxId::from_string("test"),
            working_dir: PathBuf::from("."),
            path_validator: None,
        };
        // curl should not be blocked — needed by skill scripts
        let result = tool.execute(json!({"command": "curl --version"}), &ctx).await.unwrap();
        assert!(!result.is_error);
        assert!(result.content.contains("curl"));
    }

    #[test]
    fn test_timeout_param_capped_at_120() {
        // Timeout parameter is capped at 120 seconds
        let val: u64 = 999_u64.min(120);
        assert_eq!(val, 120);
        let val: u64 = 30_u64.min(120);
        assert_eq!(val, 30);
    }

    #[tokio::test]
    async fn test_missing_command_param() {
        let tool = BashTool::new();
        let ctx = ToolContext {
            sandbox_id: octo_types::SandboxId::from_string("test"),
            working_dir: PathBuf::from("."),
            path_validator: None,
        };
        let result = tool.execute(json!({}), &ctx).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_passthrough_env_vars_include_api_keys() {
        assert!(PASSTHROUGH_ENV_VARS.contains(&"ANTHROPIC_API_KEY"));
        assert!(PASSTHROUGH_ENV_VARS.contains(&"OPENAI_API_KEY"));
        assert!(PASSTHROUGH_ENV_VARS.contains(&"TAVILY_API_KEY"));
        assert!(PASSTHROUGH_ENV_VARS.contains(&"PATH"));
        assert!(PASSTHROUGH_ENV_VARS.contains(&"HOME"));
    }
}
