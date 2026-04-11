//! HookBridge gRPC server — exposes HookBridge trait as gRPC service.

use std::sync::Arc;

use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, Streaming};
use tracing::{debug, warn};

use crate::proto;
use crate::proto::hook_bridge_service_server::HookBridgeService;
use crate::traits::*;

/// gRPC server wrapping a HookBridge implementation.
pub struct HookBridgeGrpcServer<B: HookBridge> {
    bridge: Arc<B>,
}

impl<B: HookBridge + 'static> HookBridgeGrpcServer<B> {
    pub fn new(bridge: Arc<B>) -> Self {
        Self { bridge }
    }
}

type StreamHooksResponseStream = ReceiverStream<Result<proto::HookResponse, Status>>;

#[tonic::async_trait]
impl<B: HookBridge + 'static> HookBridgeService for HookBridgeGrpcServer<B> {
    type StreamHooksStream = StreamHooksResponseStream;

    async fn stream_hooks(
        &self,
        request: Request<Streaming<proto::HookEvent>>,
    ) -> Result<Response<Self::StreamHooksStream>, Status> {
        let bridge = self.bridge.clone();
        let mut in_stream = request.into_inner();
        let (tx, rx) = mpsc::channel(32);

        tokio::spawn(async move {
            while let Ok(Some(event)) = in_stream.message().await {
                let request_id = event.request_id.clone();
                let session_id = event.session_id.clone();

                let response = match event.event {
                    Some(proto::hook_event::Event::PreToolCall(hook)) => {
                        let input = serde_json::from_str(&hook.input_json)
                            .unwrap_or(serde_json::Value::Null);
                        match bridge
                            .evaluate_pre_tool_call(
                                &session_id,
                                &hook.tool_name,
                                &hook.tool_id,
                                &input,
                            )
                            .await
                        {
                            Ok(d) => proto::HookResponse {
                                request_id,
                                response: Some(proto::hook_response::Response::Decision(
                                    decision_to_proto(d),
                                )),
                            },
                            Err(e) => error_response(&request_id, &e.to_string()),
                        }
                    }
                    Some(proto::hook_event::Event::PostToolResult(hook)) => {
                        match bridge
                            .evaluate_post_tool_result(
                                &session_id,
                                &hook.tool_name,
                                &hook.tool_id,
                                &hook.output,
                                hook.is_error,
                            )
                            .await
                        {
                            Ok(d) => proto::HookResponse {
                                request_id,
                                response: Some(proto::hook_response::Response::Decision(
                                    decision_to_proto(d),
                                )),
                            },
                            Err(e) => error_response(&request_id, &e.to_string()),
                        }
                    }
                    Some(proto::hook_event::Event::Stop(_)) => {
                        match bridge.evaluate_stop(&session_id).await {
                            // v2 collapses StopDecision into HookDecision:
                            // Complete → allow, Continue → deny with feedback as reason.
                            Ok(d) => proto::HookResponse {
                                request_id,
                                response: Some(proto::hook_response::Response::Decision(
                                    stop_decision_to_proto(d),
                                )),
                            },
                            Err(e) => error_response(&request_id, &e.to_string()),
                        }
                    }
                    Some(proto::hook_event::Event::SessionStart(_)) => {
                        debug!(session_id = %session_id, "Session start hook received");
                        proto::HookResponse {
                            request_id,
                            response: Some(proto::hook_response::Response::Decision(
                                decision_to_proto(HookDecision::Allow),
                            )),
                        }
                    }
                    Some(proto::hook_event::Event::SessionEnd(_)) => {
                        debug!(session_id = %session_id, "Session end hook received");
                        proto::HookResponse {
                            request_id,
                            response: Some(proto::hook_response::Response::Decision(
                                decision_to_proto(HookDecision::Allow),
                            )),
                        }
                    }
                    // v2 adds PrePolicyDeploy / PreApproval / EventReceived — stub to Allow.
                    Some(_) => {
                        debug!(session_id = %session_id, "Unhandled v2 hook event received, defaulting to allow");
                        proto::HookResponse {
                            request_id,
                            response: Some(proto::hook_response::Response::Decision(
                                decision_to_proto(HookDecision::Allow),
                            )),
                        }
                    }
                    None => {
                        warn!("Empty hook event received");
                        error_response(&request_id, "empty event")
                    }
                };

                if tx.send(Ok(response)).await.is_err() {
                    break;
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn evaluate_hook(
        &self,
        request: Request<proto::HookEvaluateRequest>,
    ) -> Result<Response<proto::HookDecision>, Status> {
        let req = request.into_inner();

        let event_type = proto::HookEventType::try_from(req.event_type)
            .unwrap_or(proto::HookEventType::PreToolUse);

        let decision = match event_type {
            proto::HookEventType::PreToolUse => {
                let input = serde_json::from_str(&req.input_json)
                    .unwrap_or(serde_json::Value::Null);
                self.bridge
                    .evaluate_pre_tool_call(
                        &req.session_id,
                        &req.tool_name,
                        &req.tool_id,
                        &input,
                    )
                    .await
            }
            proto::HookEventType::PostToolUse | proto::HookEventType::PostToolUseFailure => {
                self.bridge
                    .evaluate_post_tool_result(
                        &req.session_id,
                        &req.tool_name,
                        &req.tool_id,
                        &req.output,
                        req.is_error,
                    )
                    .await
            }
            proto::HookEventType::Stop => {
                return match self.bridge.evaluate_stop(&req.session_id).await {
                    Ok(StopDecision::Complete) => Ok(Response::new(proto::HookDecision {
                        decision: "allow".into(),
                        reason: String::new(),
                        mutated_input_json: String::new(),
                        precedence: 0,
                    })),
                    Ok(StopDecision::Continue { feedback }) => {
                        Ok(Response::new(proto::HookDecision {
                            decision: "deny".into(),
                            reason: feedback,
                            mutated_input_json: String::new(),
                            precedence: 0,
                        }))
                    }
                    Err(e) => Err(Status::internal(e.to_string())),
                };
            }
            other => {
                return Err(Status::invalid_argument(format!(
                    "unsupported hook event_type: {other:?}"
                )));
            }
        };

        match decision {
            Ok(d) => Ok(Response::new(decision_to_proto(d))),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn report_telemetry(
        &self,
        _request: Request<proto::HookTelemetryBatch>,
    ) -> Result<Response<proto::TelemetryAck>, Status> {
        Ok(Response::new(proto::TelemetryAck {
            accepted: 1,
            rejected: 0,
        }))
    }

    async fn get_policy_summary(
        &self,
        _request: Request<proto::PolicySummaryRequest>,
    ) -> Result<Response<proto::PolicySummary>, Status> {
        let count = self.bridge.policy_count().await;
        Ok(Response::new(proto::PolicySummary {
            total_policies: count as u32,
            policies: vec![],
        }))
    }
}

fn decision_to_proto(d: HookDecision) -> proto::HookDecision {
    match d {
        HookDecision::Allow => proto::HookDecision {
            decision: "allow".into(),
            reason: String::new(),
            mutated_input_json: String::new(),
            precedence: 0,
        },
        HookDecision::Deny { reason } => proto::HookDecision {
            decision: "deny".into(),
            reason,
            mutated_input_json: String::new(),
            precedence: 0,
        },
        HookDecision::Modify { transformed_input } => proto::HookDecision {
            decision: "mutate".into(),
            reason: String::new(),
            mutated_input_json: serde_json::to_string(&transformed_input).unwrap_or_default(),
            precedence: 0,
        },
    }
}

fn stop_decision_to_proto(d: StopDecision) -> proto::HookDecision {
    // v2 collapses StopDecision into HookDecision on the wire.
    match d {
        StopDecision::Complete => proto::HookDecision {
            decision: "allow".into(),
            reason: String::new(),
            mutated_input_json: String::new(),
            precedence: 0,
        },
        StopDecision::Continue { feedback } => proto::HookDecision {
            decision: "deny".into(),
            reason: feedback,
            mutated_input_json: String::new(),
            precedence: 0,
        },
    }
}

fn error_response(request_id: &str, message: &str) -> proto::HookResponse {
    proto::HookResponse {
        request_id: request_id.into(),
        response: Some(proto::hook_response::Response::Error(
            proto::ErrorResponse {
                code: "INTERNAL".into(),
                message: message.into(),
            },
        )),
    }
}
