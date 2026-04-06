use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

use eaasp_mcp_orchestrator::config::OrchestratorConfig;
use eaasp_mcp_orchestrator::manager::McpManager;
use eaasp_mcp_orchestrator::routes;

#[derive(Parser)]
#[command(name = "eaasp-mcp-orchestrator")]
#[command(about = "EAASP L2 MCP Orchestrator — YAML-driven MCP server lifecycle management")]
struct Cli {
    /// Path to MCP servers YAML config file.
    #[arg(long, default_value = "./config/mcp-servers.yaml")]
    config: String,

    /// HTTP listen port.
    #[arg(long, default_value_t = 8082)]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    let yaml_content = std::fs::read_to_string(&cli.config)?;
    let config: OrchestratorConfig = serde_yaml::from_str(&yaml_content)?;

    tracing::info!(
        servers = config.servers.len(),
        config_path = %cli.config,
        "loaded MCP server config"
    );

    let mgr = Arc::new(McpManager::new(config.servers));

    // Start all Shared-mode servers on boot.
    mgr.start_all().await?;

    let app = routes::router(mgr);
    let addr = format!("0.0.0.0:{}", cli.port);
    tracing::info!(addr = %addr, "starting MCP orchestrator HTTP server");

    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
