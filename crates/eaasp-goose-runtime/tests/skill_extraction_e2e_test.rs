// W1.T5 skill-extraction E2E smoke — verifies gRPC stack end-to-end;
// real ACP+middleware wiring deferred to Phase 3.
//
// Skips gracefully when goose binary is absent (same pattern as adapter_test.rs).
// Spins up GooseRuntimeService in-process on a free port, then exercises the
// Initialize → Send → Terminate lifecycle with a mock skill-extraction message.

use std::sync::Arc;

use eaasp_goose_runtime::goose_adapter::GooseAdapter;
use eaasp_goose_runtime::proto;
use eaasp_goose_runtime::proto::runtime_service_client::RuntimeServiceClient;
use eaasp_goose_runtime::proto::runtime_service_server::RuntimeServiceServer;
use eaasp_goose_runtime::service::GooseRuntimeService;
use tokio::net::TcpListener;
use tonic::transport::{Channel, Server};

async fn start_in_process_server() -> RuntimeServiceClient<Channel> {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let adapter = Arc::new(GooseAdapter::with_mode("shared"));
    let service = GooseRuntimeService::new(adapter, "shared");

    tokio::spawn(async move {
        Server::builder()
            .add_service(RuntimeServiceServer::new(service))
            .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))
            .await
            .unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    RuntimeServiceClient::connect(format!("http://{addr}"))
        .await
        .expect("failed to connect to in-process gRPC server")
}

#[tokio::test]
async fn skill_extraction_e2e_smoke() {
    // Skip if goose binary is not available — same gate as adapter_test.rs.
    if std::env::var("GOOSE_BIN").is_err() && which::which("goose").is_err() {
        eprintln!("skip: goose binary not available (set GOOSE_BIN or install goose)");
        return;
    }

    let mut client = start_in_process_server().await;

    // Initialize — creates a goose subprocess session.
    let init_resp = client
        .initialize(proto::InitializeRequest {
            payload: Some(proto::SessionPayload {
                user_id: "test-user".to_string(),
                runtime_id: "eaasp-goose-runtime".to_string(),
                skill_instructions: Some(proto::SkillInstructions {
                    skill_id: "skill-extraction".to_string(),
                    name: "skill-extraction".to_string(),
                    content: "Extract reusable skills from conversation history.".to_string(),
                    ..Default::default()
                }),
                allow_trim_p5: true,
                ..Default::default()
            }),
        })
        .await
        .expect("Initialize should succeed");

    let session_id = init_resp.into_inner().session_id;
    assert!(!session_id.is_empty(), "session_id must be non-empty");

    // Send — simulate a skill-extraction user turn; stub returns ≥1 chunk with chunk_type "done".
    let mut send_stream = client
        .send(proto::SendRequest {
            session_id: session_id.clone(),
            message: Some(proto::UserMessage {
                content: "Extract skill from conversation history".to_string(),
                message_type: "text".to_string(),
                metadata: Default::default(),
            }),
        })
        .await
        .expect("Send should succeed")
        .into_inner();

    let mut chunks = Vec::new();
    while let Some(chunk) = send_stream.message().await.unwrap() {
        chunks.push(chunk);
    }
    assert!(!chunks.is_empty(), "Send must return at least one chunk");
    // ADR-V2-021: chunk_type is the proto ChunkType enum (i32 on wire).
    assert_eq!(
        chunks.last().unwrap().chunk_type,
        proto::ChunkType::Done as i32,
        "last chunk_type must be ChunkType::Done"
    );

    // Terminate — close the session gracefully.
    client
        .terminate(proto::Empty {})
        .await
        .expect("Terminate should succeed");
}
