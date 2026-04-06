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
    /// Run blindbox comparison between two runtimes.
    Blindbox {
        /// First runtime endpoint (e.g., "http://localhost:50051")
        #[arg(long)]
        runtime_a: String,
        /// Second runtime endpoint (e.g., "http://localhost:50052")
        #[arg(long)]
        runtime_b: String,
        /// Prompt to send to both runtimes
        #[arg(short, long)]
        prompt: String,
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
        Commands::Blindbox {
            runtime_a,
            runtime_b,
            prompt,
        } => {
            info!("Starting blindbox comparison");
            let runtimes = [
                eaasp_certifier::runtime_pool::RuntimeEntry {
                    id: "runtime-a".into(),
                    name: "Runtime A".into(),
                    endpoint: runtime_a,
                    tier: "unknown".into(),
                    healthy: true,
                },
                eaasp_certifier::runtime_pool::RuntimeEntry {
                    id: "runtime-b".into(),
                    name: "Runtime B".into(),
                    endpoint: runtime_b,
                    tier: "unknown".into(),
                    healthy: true,
                },
            ];
            let record = eaasp_certifier::blindbox::execute_blindbox(&runtimes, &prompt).await?;
            println!("{}", serde_json::to_string_pretty(&record)?);
        }
    }

    Ok(())
}
