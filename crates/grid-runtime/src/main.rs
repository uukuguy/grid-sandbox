//! grid-runtime — EAASP v2 L1 gRPC server entry point.
//!
//! Starts a gRPC server exposing the v2.0 16-method RuntimeContract
//! for the EAASP platform to manage.

use std::sync::Arc;

use tonic::transport::Server;
use tracing::info;

use grid_engine::providers::{Capability, CapabilityKey, ProbeStrategy};
use grid_engine::{AgentCatalog, AgentRuntime, AgentRuntimeConfig, ProviderConfig, TenantContext};
use grid_runtime::config::RuntimeConfig;
use grid_runtime::harness::GridHarness;
use grid_runtime::proto::runtime_service_server::RuntimeServiceServer;
use grid_runtime::service::RuntimeGrpcService;
use grid_types::id::{TenantId, UserId};

/// Parse `GRID_PROBE_STRATEGY` env var. Defaults to `Eager` — grid-runtime
/// insists that the configured provider is reachable and capability is known
/// before serving any sessions.
fn probe_strategy_from_env() -> ProbeStrategy {
    match std::env::var("GRID_PROBE_STRATEGY").ok().as_deref() {
        Some("lazy") => ProbeStrategy::Lazy,
        Some("per_session") => ProbeStrategy::PerSession,
        // Default + anything else → Eager.
        _ => ProbeStrategy::Eager,
    }
}

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
        base_url = config.base_url.as_deref().unwrap_or("(default)"),
        model = %config.model,
        "grid-runtime starting (EAASP L1 Tier 1 Harness)"
    );

    // Build AgentRuntime with minimal config
    let db_path = std::env::var("GRID_DB_PATH")
        .unwrap_or_else(|_| "./data/grid-runtime.db".into());

    let provider_config = ProviderConfig {
        name: config.provider.clone(),
        api_key: config.api_key.clone(),
        base_url: config.base_url.clone(),
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

    // ── Provider capability probing (Step 4) ──────────────────────────────
    // Default strategy is Eager: probe the configured provider/model now,
    // refuse to serve if the provider is unreachable. See
    // `docs/design/EAASP/PROVIDER_CAPABILITY_MATRIX.md`.
    //
    // Escape hatches:
    //   GRID_PROBE_STRATEGY=lazy         → defer probe to first session
    //   GRID_PROBE_STRATEGY=per_session  → probe per session initialize
    let strategy = probe_strategy_from_env();
    info!(strategy = ?strategy, "capability probe strategy");
    if strategy == ProbeStrategy::Eager {
        // Use empty base_url for compatibility with the runtime-side
        // lookup in `runtime.rs::start_session_with_executor` which can't
        // see the configured base_url (Provider trait doesn't expose it).
        // The static_baseline() in capabilities.rs treats empty base_url
        // as "default direct endpoint" for openai/anthropic.
        // For OpenRouter / vLLM / etc, the probe outcome (Supported /
        // Unsupported) overrides the static "Unknown" default.
        let cap_key = CapabilityKey::new(
            &config.provider,
            &config.model,
            "",
        );
        let tool_choice_cap = engine_runtime
            .capability_store()
            .ensure_tool_choice(cap_key, engine_runtime.provider().as_ref())
            .await;
        info!(
            tool_choice = ?tool_choice_cap,
            provider = %config.provider,
            model = %config.model,
            "provider capability probed"
        );
        // Unknown here only happens if the probe couldn't decide (e.g.
        // transport error). For Eager strategy, that means the provider is
        // misconfigured or unreachable → fail startup.
        if tool_choice_cap == Capability::Unknown {
            anyhow::bail!(
                "Eager probe failed: cannot determine tool_choice capability \
                 for provider={} model={}. Provider may be unreachable or \
                 misconfigured. Set GRID_PROBE_STRATEGY=lazy to defer, or \
                 fix the provider configuration.",
                config.provider,
                config.model
            );
        }
    }

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
