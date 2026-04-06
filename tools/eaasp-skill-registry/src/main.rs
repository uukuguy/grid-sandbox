use std::sync::Arc;

use clap::Parser;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

use eaasp_skill_registry::{routes, store::SkillStore};

#[derive(Parser, Debug)]
#[command(name = "eaasp-skill-registry", about = "EAASP L2 Skill Registry server")]
struct Cli {
    /// Directory for persistent data (SQLite + skill files)
    #[arg(long, default_value = "./data/skill-registry")]
    data_dir: String,

    /// Port to listen on
    #[arg(long, default_value_t = 8081)]
    port: u16,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    let data_dir = std::path::PathBuf::from(&cli.data_dir);
    let store = SkillStore::open(&data_dir).await?;
    let store = Arc::new(store);

    let app = routes::router(store);

    let addr = format!("0.0.0.0:{}", cli.port);
    tracing::info!("Skill Registry listening on {}", addr);

    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
