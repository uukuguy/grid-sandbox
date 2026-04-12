//! grid-runtime — EAASP v2 L1 gRPC server entry point.
//!
//! Starts a gRPC server exposing the v2.0 16-method RuntimeContract
//! for the EAASP platform to manage.

use std::sync::Arc;

use tonic::transport::Server;
use tracing::info;

use grid_engine::{AgentCatalog, AgentRuntime, AgentRuntimeConfig, ProviderConfig, TenantContext};
use grid_runtime::config::RuntimeConfig;
use grid_runtime::harness::GridHarness;
use grid_runtime::proto::runtime_service_server::RuntimeServiceServer;
use grid_runtime::service::RuntimeGrpcService;
use grid_types::id::{TenantId, UserId};

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
        provider = %config.provider,
        model = %config.model,
        "grid-runtime starting (EAASP L1 Tier 1 Harness)"
    );

    // Build AgentRuntime with minimal config
    let db_path = std::env::var("GRID_DB_PATH")
        .unwrap_or_else(|_| "./data/grid-runtime.db".into());

    let provider_config = ProviderConfig {
        name: config.provider.clone(),
        api_key: config.api_key.clone(),
        base_url: None,
        model: Some(config.model.clone()),
    };

    let runtime_config = AgentRuntimeConfig::from_parts(
        db_path,
        provider_config,
        vec![], // no skill dirs for runtime
        None,   // no provider chain
        None,   // no working dir
        true,   // enable event bus for telemetry
    );

    let catalog = Arc::new(AgentCatalog::new());
    let tenant_context = TenantContext::for_single_user(
        TenantId::from_string("runtime"),
        UserId::from_string("runtime-user"),
    );

    let engine_runtime = AgentRuntime::new(catalog, runtime_config, Some(tenant_context))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to build AgentRuntime: {}", e))?;

    let harness = Arc::new(
        GridHarness::new(Arc::new(engine_runtime))
            .with_runtime_id(&config.runtime_id)
            .with_provider(&config.provider, &config.model),
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
