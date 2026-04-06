//! eaasp-certifier CLI — verify EAASP Runtime Contract compliance.

use clap::{Parser, Subcommand};
use tracing::info;

#[derive(Parser)]
#[command(name = "eaasp-certifier")]
#[command(about = "EAASP Runtime Contract verifier")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Verify a runtime endpoint for contract compliance.
    Verify {
        /// gRPC endpoint (e.g., "http://localhost:50051")
        #[arg(short, long)]
        endpoint: String,

        /// Output format: "text" | "json" | "markdown"
        #[arg(short, long, default_value = "text")]
        format: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("eaasp_certifier=info")
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Verify { endpoint, format } => {
            info!(endpoint = %endpoint, "Starting contract verification");
            let report = eaasp_certifier::verifier::verify_endpoint(&endpoint).await?;
            match format.as_str() {
                "json" => println!("{}", serde_json::to_string_pretty(&report)?),
                "markdown" => println!("{}", eaasp_certifier::report::to_markdown(&report)),
                _ => println!("{report}"),
            }
            if !report.passed {
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
