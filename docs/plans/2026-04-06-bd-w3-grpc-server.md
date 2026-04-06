# grid-runtime gRPC Server 实现计划 (BD W3)

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 让 grid-runtime 作为 gRPC server 启动，暴露 EAASP 13+ 方法契约，可被 `grpcurl` 或 eaasp-certifier 调用。

**Architecture:** `service.rs` 实现 tonic 生成的 `RuntimeService` trait，将 gRPC 请求转换为 `RuntimeContract` trait 调用。`config.rs` 管理 gRPC 端口和 engine 配置。`main.rs` 初始化 GridHarness 并启动 tonic server。先升级 proto v1.2（+3 方法），再实现 gRPC 层。

**Tech Stack:** tonic 0.12, prost 0.13, tokio, grid-engine, grid-runtime contract/harness

**Baseline:** 14 tests (7 contract + 7 harness) @ f8b8e3d

---

## Task 1: Proto v1.2 — 新增 3 个 RPC + 扩展字段

**Files:**
- Modify: `proto/eaasp/runtime/v1/runtime.proto`
- Modify: `crates/grid-runtime/src/contract.rs`
- Modify: `crates/grid-runtime/src/harness.rs`

**Step 1: 更新 runtime.proto — 新增 RPC 和 message**

在 `runtime.proto` 的 `service RuntimeService` 块末尾（`Health` 之后）新增：

```protobuf
  // Disconnect a specific MCP server.
  rpc DisconnectMcp(DisconnectMcpRequest) returns (DisconnectMcpResponse);

  // Pause a session (persist state, release resources).
  rpc PauseSession(PauseRequest) returns (PauseResponse);

  // Resume a paused session.
  rpc ResumeSession(ResumeRequest) returns (ResumeResponse);
```

在文件末尾新增 message 定义：

```protobuf
message DisconnectMcpRequest {
  string session_id = 1;
  string server_name = 2;
}

message DisconnectMcpResponse {
  bool success = 1;
}

message PauseRequest {
  string session_id = 1;
}

message PauseResponse {
  bool success = 1;
}

message ResumeRequest {
  string session_id = 1;
}

message ResumeResponse {
  bool success = 1;
  string session_id = 2;
}
```

同时扩展 `SessionPayload` 新增两个字段：

```protobuf
  string hook_bridge_url = 7;      // Optional: L3-provided HookBridge address
  string telemetry_endpoint = 8;   // Optional: L3 telemetry receiver
```

扩展 `CapabilityManifest` 新增字段：

```protobuf
  bool requires_hook_bridge = 12;  // true for Tier 2/3, false for Tier 1
```

扩展 `SessionState` 新增字段：

```protobuf
  string state_format = 5;         // "rust-serde-v1" | "python-json" | "ts-json"
```

**Step 2: 更新 contract.rs — trait + types**

在 `RuntimeContract` trait 中新增 3 个方法（`health` 之后）：

```rust
    /// Disconnect a specific MCP server by name.
    async fn disconnect_mcp(
        &self,
        handle: &SessionHandle,
        server_name: &str,
    ) -> anyhow::Result<()>;

    /// Pause session: serialize state and release resources.
    async fn pause_session(&self, handle: &SessionHandle) -> anyhow::Result<()>;

    /// Resume a previously paused session.
    async fn resume_session(&self, session_id: &str) -> anyhow::Result<SessionHandle>;
```

在 `SessionPayload` struct 新增：

```rust
    /// HookBridge URL (optional, from L3).
    pub hook_bridge_url: Option<String>,
    /// Telemetry endpoint (optional, from L3).
    pub telemetry_endpoint: Option<String>,
```

在 `CapabilityManifest` struct 新增：

```rust
    /// Whether this runtime requires HookBridge (false for Tier 1).
    pub requires_hook_bridge: bool,
```

在 `SessionState` struct 新增：

```rust
    /// Serialization format identifier (e.g., "rust-serde-v1").
    pub state_format: String,
```

**Step 3: 更新 harness.rs — GridHarness 实现 3 个新方法**

```rust
    async fn disconnect_mcp(
        &self,
        handle: &SessionHandle,
        server_name: &str,
    ) -> anyhow::Result<()> {
        let mcp_manager = self.runtime.mcp_manager();
        let mut mcp_guard = mcp_manager.lock().await;
        mcp_guard.remove_server(server_name).await;
        info!(
            session_id = %handle.session_id,
            server = %server_name,
            "GridHarness: MCP server disconnected"
        );
        Ok(())
    }

    async fn pause_session(&self, handle: &SessionHandle) -> anyhow::Result<()> {
        let session_id = SessionId::from_string(&handle.session_id);
        // Pause = get state + stop session (state can be restored later)
        self.runtime.stop_session(&session_id).await;
        info!(session_id = %handle.session_id, "GridHarness: session paused");
        Ok(())
    }

    async fn resume_session(&self, session_id: &str) -> anyhow::Result<SessionHandle> {
        // Resume requires state to be passed externally via restore_state
        // This is a simplified stub — full implementation needs L4 session store
        warn!(session_id = %session_id, "GridHarness: resume_session stub — use restore_state with persisted state");
        Err(anyhow::anyhow!("resume_session requires state from L4 session store; use restore_state instead"))
    }
```

同时修正 `get_state` 中 `SessionState` 构造，加入 `state_format: "rust-serde-v1".into()`。

修正 `get_capabilities` 中 `CapabilityManifest` 构造，加入 `requires_hook_bridge: false`。

修正 `session_payload_roundtrip` 测试和 `capability_manifest_serialization` 测试以包含新字段。

**Step 4: 验证编译通过**

Run: `cargo check -p grid-runtime`
Expected: 编译成功，无错误

**Step 5: 运行现有测试**

Run: `cargo test -p grid-runtime -- --test-threads=1`
Expected: 14 tests pass（可能需要调整测试中的 struct 构造）

**Step 6: Commit**

```bash
git add proto/eaasp/runtime/v1/runtime.proto crates/grid-runtime/src/contract.rs crates/grid-runtime/src/harness.rs
git commit -m "feat(grid-runtime): proto v1.2 — +DisconnectMcp/Pause/Resume, extend SessionPayload/CapabilityManifest/SessionState"
```

---

## Task 2: config.rs — 运行时配置

**Files:**
- Create: `crates/grid-runtime/src/config.rs`
- Modify: `crates/grid-runtime/src/lib.rs`

**Step 1: 创建 config.rs**

```rust
//! grid-runtime configuration.
//!
//! Layered: environment variables > defaults.

use std::net::SocketAddr;

/// grid-runtime server configuration.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// gRPC listen address (default: 0.0.0.0:50051).
    pub grpc_addr: SocketAddr,
    /// Runtime instance identifier.
    pub runtime_id: String,
    /// LLM provider API key.
    pub api_key: Option<String>,
    /// LLM provider (default: "anthropic").
    pub provider: String,
    /// LLM model (default: "claude-sonnet-4-20250514").
    pub model: String,
}

impl RuntimeConfig {
    /// Load configuration from environment variables.
    pub fn from_env() -> Self {
        let _ = dotenvy::dotenv();

        let grpc_addr: SocketAddr = std::env::var("GRID_RUNTIME_ADDR")
            .unwrap_or_else(|_| "0.0.0.0:50051".into())
            .parse()
            .expect("Invalid GRID_RUNTIME_ADDR");

        let runtime_id = std::env::var("GRID_RUNTIME_ID")
            .unwrap_or_else(|_| "grid-harness".into());

        let api_key = std::env::var("ANTHROPIC_API_KEY").ok();

        let provider = std::env::var("LLM_PROVIDER")
            .unwrap_or_else(|_| "anthropic".into());

        let model = std::env::var("LLM_MODEL")
            .unwrap_or_else(|_| "claude-sonnet-4-20250514".into());

        Self {
            grpc_addr,
            runtime_id,
            api_key,
            provider,
            model,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        // Clear env to test defaults
        std::env::remove_var("GRID_RUNTIME_ADDR");
        std::env::remove_var("GRID_RUNTIME_ID");
        let config = RuntimeConfig::from_env();
        assert_eq!(config.grpc_addr.port(), 50051);
        assert_eq!(config.runtime_id, "grid-harness");
        assert_eq!(config.provider, "anthropic");
    }
}
```

**Step 2: 注册模块到 lib.rs**

在 `lib.rs` 中 `pub mod harness;` 之后添加：

```rust
pub mod config;
```

**Step 3: 验证编译 + 测试**

Run: `cargo test -p grid-runtime -- --test-threads=1`
Expected: 15+ tests pass（+1 新测试）

**Step 4: Commit**

```bash
git add crates/grid-runtime/src/config.rs crates/grid-runtime/src/lib.rs
git commit -m "feat(grid-runtime): RuntimeConfig — env-based gRPC server config"
```

---

## Task 3: service.rs — gRPC service 实现

**Files:**
- Create: `crates/grid-runtime/src/service.rs`
- Modify: `crates/grid-runtime/src/lib.rs`

**Step 1: 创建 service.rs**

这是核心文件。实现 tonic 生成的 `RuntimeService` trait，将 proto 类型转换为 `contract.rs` 类型，调用 `RuntimeContract` 方法。

```rust
//! gRPC service implementation for EAASP RuntimeService.
//!
//! Maps tonic-generated types ↔ contract types, delegates to RuntimeContract.

use std::pin::Pin;
use std::sync::Arc;

use tokio_stream::Stream;
use tonic::{Request, Response, Status};

use crate::contract::{self, RuntimeContract};
use crate::proto::runtime_service_server::RuntimeService;
use crate::proto;

/// gRPC service wrapping a RuntimeContract implementation.
pub struct RuntimeGrpcService<C: RuntimeContract> {
    contract: Arc<C>,
}

impl<C: RuntimeContract + 'static> RuntimeGrpcService<C> {
    pub fn new(contract: Arc<C>) -> Self {
        Self { contract }
    }
}

// ── Type conversion helpers ──

fn to_session_payload(p: proto::SessionPayload) -> contract::SessionPayload {
    contract::SessionPayload {
        user_id: p.user_id,
        user_role: p.user_role,
        org_unit: p.org_unit,
        managed_hooks_json: if p.managed_hooks_json.is_empty() {
            None
        } else {
            Some(p.managed_hooks_json)
        },
        quotas: p.quotas,
        context: p.context,
        hook_bridge_url: if p.hook_bridge_url.is_empty() {
            None
        } else {
            Some(p.hook_bridge_url)
        },
        telemetry_endpoint: if p.telemetry_endpoint.is_empty() {
            None
        } else {
            Some(p.telemetry_endpoint)
        },
    }
}

fn to_user_message(m: proto::UserMessage) -> contract::UserMessage {
    contract::UserMessage {
        content: m.content,
        message_type: m.message_type,
        metadata: m.metadata,
    }
}

fn to_skill_content(s: proto::SkillContent) -> contract::SkillContent {
    contract::SkillContent {
        skill_id: s.skill_id,
        name: s.name,
        frontmatter_yaml: s.frontmatter_yaml,
        prose: s.prose,
    }
}

fn to_mcp_configs(servers: Vec<proto::McpServerConfig>) -> Vec<contract::McpServerConfig> {
    servers
        .into_iter()
        .map(|s| contract::McpServerConfig {
            name: s.name,
            transport: s.transport,
            command: if s.command.is_empty() {
                None
            } else {
                Some(s.command)
            },
            args: s.args,
            url: if s.url.is_empty() { None } else { Some(s.url) },
            env: s.env,
        })
        .collect()
}

fn hook_decision_to_proto(d: contract::HookDecision) -> proto::HookDecision {
    match d {
        contract::HookDecision::Allow => proto::HookDecision {
            decision: "allow".into(),
            reason: String::new(),
            modified_input: String::new(),
        },
        contract::HookDecision::Deny { reason } => proto::HookDecision {
            decision: "deny".into(),
            reason,
            modified_input: String::new(),
        },
        contract::HookDecision::Modify { transformed_input } => proto::HookDecision {
            decision: "modify".into(),
            reason: String::new(),
            modified_input: serde_json::to_string(&transformed_input).unwrap_or_default(),
        },
    }
}

fn stop_decision_to_proto(d: contract::StopDecision) -> proto::StopDecision {
    match d {
        contract::StopDecision::Complete => proto::StopDecision {
            decision: "complete".into(),
            feedback: String::new(),
        },
        contract::StopDecision::Continue { feedback } => proto::StopDecision {
            decision: "continue".into(),
            feedback,
        },
    }
}

fn session_state_to_proto(s: contract::SessionState) -> proto::SessionState {
    proto::SessionState {
        session_id: s.session_id,
        state_data: s.state_data,
        runtime_id: s.runtime_id,
        created_at: s.created_at.to_rfc3339(),
        state_format: s.state_format,
    }
}

fn capability_to_proto(c: contract::CapabilityManifest) -> proto::CapabilityManifest {
    proto::CapabilityManifest {
        runtime_id: c.runtime_id,
        runtime_name: c.runtime_name,
        tier: match c.tier {
            contract::RuntimeTier::Harness => "harness".into(),
            contract::RuntimeTier::Aligned => "aligned".into(),
            contract::RuntimeTier::Framework => "framework".into(),
        },
        model: c.model,
        context_window: c.context_window,
        supported_tools: c.supported_tools,
        native_hooks: c.native_hooks,
        native_mcp: c.native_mcp,
        native_skills: c.native_skills,
        cost: c.cost.map(|c| proto::CostEstimate {
            input_cost_per_1k: c.input_cost_per_1k,
            output_cost_per_1k: c.output_cost_per_1k,
        }),
        metadata: c.metadata,
        requires_hook_bridge: c.requires_hook_bridge,
    }
}

fn telemetry_to_proto(events: Vec<contract::TelemetryEvent>) -> proto::TelemetryBatch {
    proto::TelemetryBatch {
        events: events
            .into_iter()
            .map(|e| proto::TelemetryEvent {
                session_id: e.session_id,
                runtime_id: e.runtime_id,
                user_id: e.user_id.unwrap_or_default(),
                event_type: e.event_type,
                timestamp: e.timestamp.to_rfc3339(),
                payload_json: serde_json::to_string(&e.payload).unwrap_or_default(),
                resource_usage: Some(proto::ResourceUsage {
                    input_tokens: e.resource_usage.input_tokens,
                    output_tokens: e.resource_usage.output_tokens,
                    compute_ms: e.resource_usage.compute_ms,
                }),
            })
            .collect(),
    }
}

fn health_to_proto(h: contract::HealthStatus) -> proto::HealthStatus {
    proto::HealthStatus {
        healthy: h.healthy,
        runtime_id: h.runtime_id,
        checks: h.checks,
    }
}

// ── gRPC Service Implementation ──

type SendStream =
    Pin<Box<dyn Stream<Item = Result<proto::ResponseChunk, Status>> + Send>>;

#[tonic::async_trait]
impl<C: RuntimeContract + 'static> RuntimeService for RuntimeGrpcService<C> {
    type SendStream = SendStream;

    async fn initialize(
        &self,
        request: Request<proto::InitializeRequest>,
    ) -> Result<Response<proto::InitializeResponse>, Status> {
        let payload = request
            .into_inner()
            .payload
            .ok_or_else(|| Status::invalid_argument("missing payload"))?;

        let handle = self
            .contract
            .initialize(to_session_payload(payload))
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(proto::InitializeResponse {
            session_id: handle.session_id,
        }))
    }

    async fn send(
        &self,
        request: Request<proto::SendRequest>,
    ) -> Result<Response<Self::SendStream>, Status> {
        let req = request.into_inner();
        let handle = contract::SessionHandle {
            session_id: req.session_id,
        };
        let message = req
            .message
            .ok_or_else(|| Status::invalid_argument("missing message"))?;

        let stream = self
            .contract
            .send(&handle, to_user_message(message))
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        // Map contract::ResponseChunk → proto::ResponseChunk
        let proto_stream = tokio_stream::StreamExt::map(stream, |chunk| {
            Ok(proto::ResponseChunk {
                chunk_type: chunk.chunk_type,
                content: chunk.content,
                tool_name: chunk.tool_name.unwrap_or_default(),
                tool_id: chunk.tool_id.unwrap_or_default(),
                is_error: chunk.is_error,
            })
        });

        Ok(Response::new(Box::pin(proto_stream)))
    }

    async fn load_skill(
        &self,
        request: Request<proto::LoadSkillRequest>,
    ) -> Result<Response<proto::LoadSkillResponse>, Status> {
        let req = request.into_inner();
        let handle = contract::SessionHandle {
            session_id: req.session_id,
        };
        let skill = req
            .skill
            .ok_or_else(|| Status::invalid_argument("missing skill"))?;

        match self
            .contract
            .load_skill(&handle, to_skill_content(skill))
            .await
        {
            Ok(()) => Ok(Response::new(proto::LoadSkillResponse {
                success: true,
                error: String::new(),
            })),
            Err(e) => Ok(Response::new(proto::LoadSkillResponse {
                success: false,
                error: e.to_string(),
            })),
        }
    }

    async fn on_tool_call(
        &self,
        request: Request<proto::ToolCallEvent>,
    ) -> Result<Response<proto::HookDecision>, Status> {
        let req = request.into_inner();
        let handle = contract::SessionHandle {
            session_id: req.session_id,
        };
        let call = contract::ToolCall {
            tool_name: req.tool_name,
            tool_id: req.tool_id,
            input: serde_json::from_str(&req.input_json).unwrap_or_default(),
        };

        let decision = self
            .contract
            .on_tool_call(&handle, call)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(hook_decision_to_proto(decision)))
    }

    async fn on_tool_result(
        &self,
        request: Request<proto::ToolResultEvent>,
    ) -> Result<Response<proto::HookDecision>, Status> {
        let req = request.into_inner();
        let handle = contract::SessionHandle {
            session_id: req.session_id,
        };
        let result = contract::ToolResult {
            tool_name: req.tool_name,
            tool_id: req.tool_id,
            output: req.output,
            is_error: req.is_error,
        };

        let decision = self
            .contract
            .on_tool_result(&handle, result)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(hook_decision_to_proto(decision)))
    }

    async fn on_stop(
        &self,
        request: Request<proto::StopRequest>,
    ) -> Result<Response<proto::StopDecision>, Status> {
        let handle = contract::SessionHandle {
            session_id: request.into_inner().session_id,
        };

        let decision = self
            .contract
            .on_stop(&handle)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(stop_decision_to_proto(decision)))
    }

    async fn get_state(
        &self,
        request: Request<proto::GetStateRequest>,
    ) -> Result<Response<proto::SessionState>, Status> {
        let handle = contract::SessionHandle {
            session_id: request.into_inner().session_id,
        };

        let state = self
            .contract
            .get_state(&handle)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(session_state_to_proto(state)))
    }

    async fn restore_state(
        &self,
        request: Request<proto::SessionState>,
    ) -> Result<Response<proto::InitializeResponse>, Status> {
        let ps = request.into_inner();
        let state = contract::SessionState {
            session_id: ps.session_id,
            runtime_id: ps.runtime_id,
            state_data: ps.state_data,
            created_at: chrono::Utc::now(), // L4 should pass real timestamp
            state_format: ps.state_format,
        };

        let handle = self
            .contract
            .restore_state(state)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(proto::InitializeResponse {
            session_id: handle.session_id,
        }))
    }

    async fn connect_mcp(
        &self,
        request: Request<proto::ConnectMcpRequest>,
    ) -> Result<Response<proto::ConnectMcpResponse>, Status> {
        let req = request.into_inner();
        let handle = contract::SessionHandle {
            session_id: req.session_id,
        };
        let server_names: Vec<String> = req.servers.iter().map(|s| s.name.clone()).collect();

        match self
            .contract
            .connect_mcp(&handle, to_mcp_configs(req.servers))
            .await
        {
            Ok(()) => Ok(Response::new(proto::ConnectMcpResponse {
                success: true,
                connected: server_names,
                failed: vec![],
            })),
            Err(e) => Ok(Response::new(proto::ConnectMcpResponse {
                success: false,
                connected: vec![],
                failed: vec![e.to_string()],
            })),
        }
    }

    async fn emit_telemetry(
        &self,
        request: Request<proto::EmitTelemetryRequest>,
    ) -> Result<Response<proto::TelemetryBatch>, Status> {
        let handle = contract::SessionHandle {
            session_id: request.into_inner().session_id,
        };

        let events = self
            .contract
            .emit_telemetry(&handle)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(telemetry_to_proto(events)))
    }

    async fn get_capabilities(
        &self,
        _request: Request<proto::Empty>,
    ) -> Result<Response<proto::CapabilityManifest>, Status> {
        let manifest = self.contract.get_capabilities();
        Ok(Response::new(capability_to_proto(manifest)))
    }

    async fn terminate(
        &self,
        request: Request<proto::TerminateRequest>,
    ) -> Result<Response<proto::TerminateResponse>, Status> {
        let handle = contract::SessionHandle {
            session_id: request.into_inner().session_id.clone(),
        };

        // Emit final telemetry before termination
        let final_telemetry = self
            .contract
            .emit_telemetry(&handle)
            .await
            .unwrap_or_default();

        self.contract
            .terminate(&handle)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(proto::TerminateResponse {
            success: true,
            final_telemetry: Some(telemetry_to_proto(final_telemetry)),
        }))
    }

    async fn health(
        &self,
        _request: Request<proto::Empty>,
    ) -> Result<Response<proto::HealthStatus>, Status> {
        let status = self
            .contract
            .health()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(health_to_proto(status)))
    }

    async fn disconnect_mcp(
        &self,
        request: Request<proto::DisconnectMcpRequest>,
    ) -> Result<Response<proto::DisconnectMcpResponse>, Status> {
        let req = request.into_inner();
        let handle = contract::SessionHandle {
            session_id: req.session_id,
        };

        self.contract
            .disconnect_mcp(&handle, &req.server_name)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(proto::DisconnectMcpResponse {
            success: true,
        }))
    }

    async fn pause_session(
        &self,
        request: Request<proto::PauseRequest>,
    ) -> Result<Response<proto::PauseResponse>, Status> {
        let handle = contract::SessionHandle {
            session_id: request.into_inner().session_id,
        };

        self.contract
            .pause_session(&handle)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(proto::PauseResponse { success: true }))
    }

    async fn resume_session(
        &self,
        request: Request<proto::ResumeRequest>,
    ) -> Result<Response<proto::ResumeResponse>, Status> {
        let session_id = request.into_inner().session_id;

        let handle = self
            .contract
            .resume_session(&session_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(proto::ResumeResponse {
            success: true,
            session_id: handle.session_id,
        }))
    }
}
```

**Step 2: 注册模块到 lib.rs**

在 `lib.rs` 中 `pub mod config;` 之后添加：

```rust
pub mod service;
```

**Step 3: 验证编译**

Run: `cargo check -p grid-runtime`
Expected: 编译成功

**Step 4: Commit**

```bash
git add crates/grid-runtime/src/service.rs crates/grid-runtime/src/lib.rs
git commit -m "feat(grid-runtime): RuntimeGrpcService — tonic service impl for 16-method contract"
```

---

## Task 4: main.rs — gRPC server 启动

**Files:**
- Modify: `crates/grid-runtime/src/main.rs`

**Step 1: 实现 main.rs**

```rust
//! grid-runtime — EAASP L1 gRPC server entry point.
//!
//! Starts a gRPC server exposing the 16-method RuntimeContract
//! for the EAASP platform to manage.

use std::sync::Arc;

use tonic::transport::Server;
use tracing::info;

use grid_runtime::config::RuntimeConfig;
use grid_runtime::harness::GridHarness;
use grid_runtime::proto::runtime_service_server::RuntimeServiceServer;
use grid_runtime::service::RuntimeGrpcService;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "grid_runtime=info".into()),
        )
        .init();

    let config = RuntimeConfig::from_env();
    info!(
        addr = %config.grpc_addr,
        runtime_id = %config.runtime_id,
        "grid-runtime starting (EAASP L1 Tier 1 Harness)"
    );

    // Build AgentRuntime
    let engine_runtime = grid_engine::AgentRuntime::builder()
        .build()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to build AgentRuntime: {}", e))?;

    let harness = Arc::new(
        GridHarness::new(Arc::new(engine_runtime))
            .with_runtime_id(&config.runtime_id),
    );

    let grpc_service = RuntimeGrpcService::new(harness);
    let server = RuntimeServiceServer::new(grpc_service);

    info!(addr = %config.grpc_addr, "gRPC server listening");

    Server::builder()
        .add_service(server)
        .serve(config.grpc_addr)
        .await?;

    Ok(())
}
```

**Step 2: 验证编译**

Run: `cargo build -p grid-runtime`
Expected: 编译成功，生成 `grid-runtime` 二进制

**Step 3: Commit**

```bash
git add crates/grid-runtime/src/main.rs
git commit -m "feat(grid-runtime): gRPC server entry point — tonic + GridHarness"
```

---

## Task 5: gRPC 集成测试

**Files:**
- Create: `crates/grid-runtime/tests/grpc_integration.rs`

**Step 1: 创建集成测试**

```rust
//! gRPC integration tests for grid-runtime.
//!
//! Tests the full gRPC stack: client → service → GridHarness → grid-engine.
//! Uses an in-process tonic server (no network).

use std::sync::Arc;

use tonic::transport::{Channel, Server};
use tokio::net::TcpListener;

use grid_runtime::proto::runtime_service_client::RuntimeServiceClient;
use grid_runtime::proto::runtime_service_server::RuntimeServiceServer;
use grid_runtime::proto;
use grid_runtime::harness::GridHarness;
use grid_runtime::service::RuntimeGrpcService;

/// Start an in-process gRPC server and return a connected client.
async fn setup_grpc() -> RuntimeServiceClient<Channel> {
    let runtime = grid_engine::AgentRuntime::builder()
        .build()
        .await
        .expect("Failed to build AgentRuntime");

    let harness = Arc::new(GridHarness::new(Arc::new(runtime)));
    let service = RuntimeGrpcService::new(harness);
    let server = RuntimeServiceServer::new(service);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        Server::builder()
            .add_service(server)
            .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))
            .await
            .unwrap();
    });

    // Give server a moment to start
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    RuntimeServiceClient::connect(format!("http://{}", addr))
        .await
        .expect("Failed to connect to gRPC server")
}

#[tokio::test]
async fn test_health() {
    let mut client = setup_grpc().await;

    let response = client
        .health(proto::Empty {})
        .await
        .expect("health failed");

    let status = response.into_inner();
    assert!(status.healthy);
    assert_eq!(status.runtime_id, "grid-harness");
}

#[tokio::test]
async fn test_get_capabilities() {
    let mut client = setup_grpc().await;

    let response = client
        .get_capabilities(proto::Empty {})
        .await
        .expect("get_capabilities failed");

    let manifest = response.into_inner();
    assert_eq!(manifest.runtime_name, "Grid");
    assert_eq!(manifest.tier, "harness");
    assert!(manifest.native_hooks);
    assert!(manifest.native_mcp);
    assert!(manifest.native_skills);
    assert!(!manifest.requires_hook_bridge);
}

#[tokio::test]
async fn test_initialize_and_terminate() {
    let mut client = setup_grpc().await;

    // Initialize
    let response = client
        .initialize(proto::InitializeRequest {
            payload: Some(proto::SessionPayload {
                user_id: "test-user".into(),
                user_role: "developer".into(),
                org_unit: "engineering".into(),
                managed_hooks_json: String::new(),
                quotas: Default::default(),
                context: Default::default(),
                hook_bridge_url: String::new(),
                telemetry_endpoint: String::new(),
            }),
        })
        .await
        .expect("initialize failed");

    let session_id = response.into_inner().session_id;
    assert!(!session_id.is_empty());

    // Terminate
    let response = client
        .terminate(proto::TerminateRequest {
            session_id: session_id.clone(),
        })
        .await
        .expect("terminate failed");

    assert!(response.into_inner().success);
}

#[tokio::test]
async fn test_on_tool_call_allows() {
    let mut client = setup_grpc().await;

    let response = client
        .on_tool_call(proto::ToolCallEvent {
            session_id: "test".into(),
            tool_name: "bash".into(),
            tool_id: "t1".into(),
            input_json: "{}".into(),
        })
        .await
        .expect("on_tool_call failed");

    assert_eq!(response.into_inner().decision, "allow");
}

#[tokio::test]
async fn test_on_stop_completes() {
    let mut client = setup_grpc().await;

    let response = client
        .on_stop(proto::StopRequest {
            session_id: "test".into(),
        })
        .await
        .expect("on_stop failed");

    assert_eq!(response.into_inner().decision, "complete");
}
```

**Step 2: 运行测试**

Run: `cargo test -p grid-runtime -- --test-threads=1`
Expected: 所有测试通过（14 原有 + 1 config + 5 集成 = 20+）

**Step 3: Commit**

```bash
git add -f crates/grid-runtime/tests/grpc_integration.rs
git commit -m "test(grid-runtime): gRPC integration tests — health, capabilities, init/terminate, hooks"
```

---

## Task 6: Dockerfile + Makefile

**Files:**
- Create: `crates/grid-runtime/Dockerfile`
- Modify: `Makefile`

**Step 1: 创建 Dockerfile**

```dockerfile
# grid-runtime — EAASP L1 Tier 1 Harness container image.
#
# Multi-stage build: compile in Rust builder, run in minimal distroless.
# L1 containers are EPHEMERAL: created per-session, destroyed on terminate.

# ── Build stage ──
FROM rust:1.86-bookworm AS builder

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY proto/ proto/

# Install protobuf compiler
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

# Build release binary
RUN cargo build -p grid-runtime --release

# ── Runtime stage ──
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/grid-runtime /usr/local/bin/grid-runtime

# gRPC port
EXPOSE 50051

# Environment defaults
ENV GRID_RUNTIME_ADDR=0.0.0.0:50051
ENV GRID_RUNTIME_ID=grid-harness
ENV RUST_LOG=grid_runtime=info

# Health check via gRPC health (basic TCP check)
HEALTHCHECK --interval=10s --timeout=3s --start-period=5s \
    CMD timeout 2 bash -c '</dev/tcp/localhost/50051' || exit 1

ENTRYPOINT ["grid-runtime"]
```

**Step 2: 更新 Makefile — 新增 grid-runtime 构建目标**

在 Makefile 的容器相关部分后添加：

```makefile
# ── grid-runtime container ──
runtime-build:
	@echo "Building grid-runtime container image..."
	docker build -f crates/grid-runtime/Dockerfile -t grid-runtime:latest .

runtime-run:
	@echo "Starting grid-runtime container..."
	docker run --rm -p 50051:50051 \
		-e ANTHROPIC_API_KEY=$${ANTHROPIC_API_KEY} \
		grid-runtime:latest

runtime-build-binary:
	@echo "Building grid-runtime binary..."
	cargo build -p grid-runtime --release
```

**Step 3: 验证 Makefile 目标**

Run: `make runtime-build-binary`
Expected: Release 编译成功

**Step 4: Commit**

```bash
git add crates/grid-runtime/Dockerfile Makefile
git commit -m "feat(grid-runtime): Dockerfile + Makefile targets for containerized deployment"
```

---

## 总结

| Task | 产出 | 新增测试 |
|------|------|---------|
| 1 | proto v1.2 + contract/harness 更新 | 修复现有 14 tests |
| 2 | config.rs | +1 test |
| 3 | service.rs (gRPC service impl) | 编译验证 |
| 4 | main.rs (server entry) | 编译验证 |
| 5 | grpc_integration.rs | +5 tests |
| 6 | Dockerfile + Makefile | 二进制构建验证 |

**预期最终测试数**: 20+ grid-runtime tests

**验收标准**:
- `cargo test -p grid-runtime -- --test-threads=1` 全部通过
- `cargo build -p grid-runtime --release` 成功
- `make runtime-build-binary` 成功
- Docker 镜像可构建（如有 Docker 环境）
