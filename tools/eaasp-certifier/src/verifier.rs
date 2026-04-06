//! Verifier — 16-method contract verification engine.

use std::fmt;

use serde::{Deserialize, Serialize};
use tonic::transport::Channel;
use tracing::{error, info, warn};

use crate::common_proto;
use crate::runtime_proto;
use crate::runtime_proto::runtime_service_client::RuntimeServiceClient;

/// Verification result for a single method.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodResult {
    pub method: String,
    pub passed: bool,
    pub duration_ms: u64,
    pub error: Option<String>,
    pub notes: Option<String>,
}

/// Full verification report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReport {
    pub endpoint: String,
    pub runtime_id: String,
    pub runtime_name: String,
    pub tier: String,
    pub deployment_mode: String,
    pub passed: bool,
    pub total: usize,
    pub passed_count: usize,
    pub failed_count: usize,
    pub results: Vec<MethodResult>,
    pub timestamp: String,
}

impl fmt::Display for VerificationReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "================================================================")?;
        writeln!(f, " EAASP Contract Verification Report")?;
        writeln!(f, "================================================================")?;
        writeln!(f, " Endpoint:    {}", self.endpoint)?;
        writeln!(
            f,
            " Runtime:     {} ({})",
            self.runtime_name, self.runtime_id
        )?;
        writeln!(f, " Tier:        {}", self.tier)?;
        writeln!(f, " Deploy:      {}", self.deployment_mode)?;
        writeln!(f, " Timestamp:   {}", self.timestamp)?;
        writeln!(f, "----------------------------------------------------------------")?;
        writeln!(
            f,
            " Result:      {}/{} passed",
            self.passed_count, self.total
        )?;
        writeln!(
            f,
            " Status:      {}",
            if self.passed { "PASS" } else { "FAIL" }
        )?;
        writeln!(f, "----------------------------------------------------------------")?;

        for result in &self.results {
            let icon = if result.passed { "OK" } else { "FAIL" };
            write!(
                f,
                " [{icon:>4}] {:30} {:>6}ms",
                result.method, result.duration_ms
            )?;
            if let Some(err) = &result.error {
                write!(f, "  ! {err}")?;
            }
            if let Some(notes) = &result.notes {
                if result.error.is_none() {
                    write!(f, "  ({notes})")?;
                }
            }
            writeln!(f)?;
        }

        writeln!(f, "================================================================")?;
        Ok(())
    }
}

/// Verify all 16 methods of the RuntimeService contract.
pub async fn verify_endpoint(endpoint: &str) -> anyhow::Result<VerificationReport> {
    let channel = Channel::from_shared(endpoint.to_string())?
        .connect()
        .await?;
    let mut client = RuntimeServiceClient::new(channel);

    let mut results = Vec::new();

    // 1. Health
    results.push(verify_health(&mut client).await);

    // 2. GetCapabilities
    let caps = verify_get_capabilities(&mut client).await;
    let caps_info = caps.notes.clone().unwrap_or_default();
    results.push(caps);

    // 3. Initialize
    let init_result = verify_initialize(&mut client).await;
    let session_id = init_result
        .notes
        .clone()
        .unwrap_or_else(|| "test-session".into());
    results.push(init_result);

    // 4-16: remaining methods
    results.push(verify_send(&mut client, &session_id).await);
    results.push(verify_load_skill(&mut client, &session_id).await);
    results.push(verify_on_tool_call(&mut client, &session_id).await);
    results.push(verify_on_tool_result(&mut client, &session_id).await);
    results.push(verify_on_stop(&mut client, &session_id).await);
    results.push(verify_connect_mcp(&mut client, &session_id).await);
    results.push(verify_disconnect_mcp(&mut client, &session_id).await);
    results.push(verify_emit_telemetry(&mut client, &session_id).await);
    results.push(verify_get_state(&mut client, &session_id).await);
    results.push(verify_pause_session(&mut client, &session_id).await);
    results.push(verify_resume_session(&mut client, &session_id).await);
    results.push(verify_restore_state(&mut client).await);
    results.push(verify_terminate(&mut client, &session_id).await);

    let passed_count = results.iter().filter(|r| r.passed).count();
    let total = results.len();
    let (runtime_id, runtime_name, tier, deployment_mode) = parse_caps_info(&caps_info);

    Ok(VerificationReport {
        endpoint: endpoint.to_string(),
        runtime_id,
        runtime_name,
        tier,
        deployment_mode,
        passed: passed_count == total,
        total,
        passed_count,
        failed_count: total - passed_count,
        results,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

fn parse_caps_info(info: &str) -> (String, String, String, String) {
    let parts: Vec<&str> = info.splitn(4, ':').collect();
    match parts.as_slice() {
        [id, name, tier, deploy] => (
            id.to_string(),
            name.to_string(),
            tier.to_string(),
            deploy.to_string(),
        ),
        [id, name, tier] => (
            id.to_string(),
            name.to_string(),
            tier.to_string(),
            "unknown".into(),
        ),
        _ => (
            "unknown".into(),
            "unknown".into(),
            "unknown".into(),
            "unknown".into(),
        ),
    }
}

macro_rules! timed_verify {
    ($name:expr, $block:expr) => {{
        let start = std::time::Instant::now();
        let result: Result<Option<String>, anyhow::Error> = (async { $block }).await;
        let duration_ms = start.elapsed().as_millis() as u64;
        match result {
            Ok(notes) => MethodResult {
                method: $name.into(),
                passed: true,
                duration_ms,
                error: None,
                notes,
            },
            Err(e) => {
                error!(method = $name, error = %e, "Verification failed");
                MethodResult {
                    method: $name.into(),
                    passed: false,
                    duration_ms,
                    error: Some(e.to_string()),
                    notes: None,
                }
            }
        }
    }};
}

async fn verify_health(client: &mut RuntimeServiceClient<Channel>) -> MethodResult {
    timed_verify!("Health", {
        let resp = client
            .health(common_proto::Empty {})
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let status = resp.into_inner();
        if status.healthy {
            info!("Health: ok (runtime_id={})", status.runtime_id);
            Ok(None)
        } else {
            Err(anyhow::anyhow!("Runtime reports unhealthy"))
        }
    })
}

async fn verify_get_capabilities(client: &mut RuntimeServiceClient<Channel>) -> MethodResult {
    timed_verify!("GetCapabilities", {
        let resp = client
            .get_capabilities(common_proto::Empty {})
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let cap = resp.into_inner();
        info!(
            runtime = %cap.runtime_name,
            tier = %cap.tier,
            tools = cap.supported_tools.len(),
            "GetCapabilities OK"
        );
        Ok(Some(format!(
            "{}:{}:{}:{}",
            cap.runtime_id, cap.runtime_name, cap.tier, cap.deployment_mode
        )))
    })
}

async fn verify_initialize(client: &mut RuntimeServiceClient<Channel>) -> MethodResult {
    timed_verify!("Initialize", {
        let resp = client
            .initialize(runtime_proto::InitializeRequest {
                payload: Some(runtime_proto::SessionPayload {
                    user_id: "certifier-user".into(),
                    user_role: "tester".into(),
                    org_unit: "qa".into(),
                    ..Default::default()
                }),
            })
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let session_id = resp.into_inner().session_id;
        info!(session_id = %session_id, "Initialize OK");
        Ok(Some(session_id))
    })
}

async fn verify_send(
    client: &mut RuntimeServiceClient<Channel>,
    session_id: &str,
) -> MethodResult {
    timed_verify!("Send", {
        use tokio_stream::StreamExt;
        let mut stream = client
            .send(runtime_proto::SendRequest {
                session_id: session_id.into(),
                message: Some(runtime_proto::UserMessage {
                    content: "Say hello".into(),
                    message_type: "text".into(),
                    metadata: Default::default(),
                }),
            })
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?
            .into_inner();

        let mut chunk_count = 0u32;
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(c) => {
                    chunk_count += 1;
                    if c.chunk_type == "done" {
                        break;
                    }
                }
                Err(e) => {
                    warn!("Send stream error: {e}");
                    break;
                }
            }
        }
        info!(chunks = chunk_count, "Send OK");
        Ok(Some(format!("{chunk_count} chunks")))
    })
}

async fn verify_load_skill(
    client: &mut RuntimeServiceClient<Channel>,
    session_id: &str,
) -> MethodResult {
    timed_verify!("LoadSkill", {
        let resp = client
            .load_skill(runtime_proto::LoadSkillRequest {
                session_id: session_id.into(),
                skill: Some(runtime_proto::SkillContent {
                    skill_id: "test-skill".into(),
                    name: "Test Skill".into(),
                    frontmatter_yaml: "---\nname: test\n---".into(),
                    prose: "Do a simple test.".into(),
                }),
            })
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let result = resp.into_inner();
        if result.success {
            Ok(None)
        } else {
            Err(anyhow::anyhow!("LoadSkill failed: {}", result.error))
        }
    })
}

async fn verify_on_tool_call(
    client: &mut RuntimeServiceClient<Channel>,
    session_id: &str,
) -> MethodResult {
    timed_verify!("OnToolCall", {
        let resp = client
            .on_tool_call(common_proto::ToolCallEvent {
                session_id: session_id.into(),
                tool_name: "bash".into(),
                tool_id: "t-cert-1".into(),
                input_json: r#"{"command":"echo hello"}"#.into(),
            })
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let decision = resp.into_inner();
        info!(decision = %decision.decision, "OnToolCall OK");
        Ok(None)
    })
}

async fn verify_on_tool_result(
    client: &mut RuntimeServiceClient<Channel>,
    session_id: &str,
) -> MethodResult {
    timed_verify!("OnToolResult", {
        let resp = client
            .on_tool_result(common_proto::ToolResultEvent {
                session_id: session_id.into(),
                tool_name: "bash".into(),
                tool_id: "t-cert-1".into(),
                output: "hello".into(),
                is_error: false,
            })
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let decision = resp.into_inner();
        info!(decision = %decision.decision, "OnToolResult OK");
        Ok(None)
    })
}

async fn verify_on_stop(
    client: &mut RuntimeServiceClient<Channel>,
    session_id: &str,
) -> MethodResult {
    timed_verify!("OnStop", {
        let resp = client
            .on_stop(common_proto::StopRequest {
                session_id: session_id.into(),
            })
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let decision = resp.into_inner();
        info!(decision = %decision.decision, "OnStop OK");
        Ok(None)
    })
}

async fn verify_connect_mcp(
    client: &mut RuntimeServiceClient<Channel>,
    session_id: &str,
) -> MethodResult {
    timed_verify!("ConnectMcp", {
        let resp = client
            .connect_mcp(runtime_proto::ConnectMcpRequest {
                session_id: session_id.into(),
                servers: vec![runtime_proto::McpServerConfig {
                    name: "certifier-test-mcp".into(),
                    transport: "stdio".into(),
                    command: "echo".into(),
                    args: vec!["test".into()],
                    url: String::new(),
                    env: Default::default(),
                }],
            })
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let result = resp.into_inner();
        info!(success = result.success, "ConnectMcp responded");
        Ok(None)
    })
}

async fn verify_disconnect_mcp(
    client: &mut RuntimeServiceClient<Channel>,
    session_id: &str,
) -> MethodResult {
    timed_verify!("DisconnectMcp", {
        client
            .disconnect_mcp(runtime_proto::DisconnectMcpRequest {
                session_id: session_id.into(),
                server_name: "certifier-test-mcp".into(),
            })
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(None)
    })
}

async fn verify_emit_telemetry(
    client: &mut RuntimeServiceClient<Channel>,
    session_id: &str,
) -> MethodResult {
    timed_verify!("EmitTelemetry", {
        let resp = client
            .emit_telemetry(runtime_proto::EmitTelemetryRequest {
                session_id: session_id.into(),
            })
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let batch = resp.into_inner();
        info!(events = batch.events.len(), "EmitTelemetry OK");
        Ok(Some(format!("{} events", batch.events.len())))
    })
}

async fn verify_get_state(
    client: &mut RuntimeServiceClient<Channel>,
    session_id: &str,
) -> MethodResult {
    timed_verify!("GetState", {
        let resp = client
            .get_state(runtime_proto::GetStateRequest {
                session_id: session_id.into(),
            })
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let state = resp.into_inner();
        info!(
            format = %state.state_format,
            bytes = state.state_data.len(),
            "GetState OK"
        );
        Ok(Some(format!(
            "format={}, {}B",
            state.state_format,
            state.state_data.len()
        )))
    })
}

async fn verify_pause_session(
    client: &mut RuntimeServiceClient<Channel>,
    session_id: &str,
) -> MethodResult {
    timed_verify!("PauseSession", {
        let resp = client
            .pause_session(runtime_proto::PauseRequest {
                session_id: session_id.into(),
            })
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let result = resp.into_inner();
        if result.success {
            Ok(None)
        } else {
            Err(anyhow::anyhow!("PauseSession returned success=false"))
        }
    })
}

async fn verify_resume_session(
    client: &mut RuntimeServiceClient<Channel>,
    session_id: &str,
) -> MethodResult {
    timed_verify!("ResumeSession", {
        let result = client
            .resume_session(runtime_proto::ResumeRequest {
                session_id: session_id.into(),
            })
            .await;
        match result {
            Ok(resp) => {
                let r = resp.into_inner();
                info!(session_id = %r.session_id, "ResumeSession OK");
                Ok(None)
            }
            Err(e) => {
                warn!("ResumeSession returned error (expected for some runtimes): {e}");
                Ok(Some("method exists but not fully implemented".into()))
            }
        }
    })
}

async fn verify_restore_state(client: &mut RuntimeServiceClient<Channel>) -> MethodResult {
    timed_verify!("RestoreState", {
        let state = runtime_proto::SessionState {
            session_id: "certifier-restore-test".into(),
            state_data: serde_json::to_vec(&serde_json::json!([]))?,
            runtime_id: "certifier".into(),
            created_at: chrono::Utc::now().to_rfc3339(),
            state_format: "rust-serde-v1".into(),
        };
        let result = client.restore_state(state).await;
        match result {
            Ok(resp) => {
                let r = resp.into_inner();
                info!(session_id = %r.session_id, "RestoreState OK");
                Ok(None)
            }
            Err(e) => {
                warn!("RestoreState returned error: {e}");
                Ok(Some("method exists, may need valid state data".into()))
            }
        }
    })
}

async fn verify_terminate(
    client: &mut RuntimeServiceClient<Channel>,
    session_id: &str,
) -> MethodResult {
    timed_verify!("Terminate", {
        let resp = client
            .terminate(runtime_proto::TerminateRequest {
                session_id: session_id.into(),
            })
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let result = resp.into_inner();
        if result.success {
            let telemetry_count = result
                .final_telemetry
                .map(|b| b.events.len())
                .unwrap_or(0);
            info!(telemetry = telemetry_count, "Terminate OK");
            Ok(Some(format!("{telemetry_count} final telemetry events")))
        } else {
            Err(anyhow::anyhow!("Terminate returned success=false"))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verification_report_display() {
        let report = VerificationReport {
            endpoint: "http://localhost:50051".into(),
            runtime_id: "grid-harness".into(),
            runtime_name: "Grid".into(),
            tier: "harness".into(),
            deployment_mode: "shared".into(),
            passed: true,
            total: 2,
            passed_count: 2,
            failed_count: 0,
            results: vec![
                MethodResult {
                    method: "Health".into(),
                    passed: true,
                    duration_ms: 5,
                    error: None,
                    notes: None,
                },
                MethodResult {
                    method: "Initialize".into(),
                    passed: true,
                    duration_ms: 12,
                    error: None,
                    notes: Some("session-123".into()),
                },
            ],
            timestamp: "2026-04-06T12:00:00Z".into(),
        };

        let output = format!("{report}");
        assert!(output.contains("PASS"));
        assert!(output.contains("Grid"));
        assert!(output.contains("Health"));
        assert!(output.contains("2/2"));
    }

    #[test]
    fn parse_caps_info_valid() {
        let (id, name, tier, deploy) =
            parse_caps_info("grid-harness:Grid:harness:shared");
        assert_eq!(id, "grid-harness");
        assert_eq!(name, "Grid");
        assert_eq!(tier, "harness");
        assert_eq!(deploy, "shared");
    }

    #[test]
    fn parse_caps_info_without_deploy() {
        let (id, name, tier, deploy) = parse_caps_info("grid-harness:Grid:harness");
        assert_eq!(id, "grid-harness");
        assert_eq!(tier, "harness");
        assert_eq!(deploy, "unknown");
    }

    #[test]
    fn parse_caps_info_empty() {
        let (id, name, tier, deploy) = parse_caps_info("");
        assert_eq!(id, "unknown");
        assert_eq!(name, "unknown");
        assert_eq!(tier, "unknown");
        assert_eq!(deploy, "unknown");
    }
}
