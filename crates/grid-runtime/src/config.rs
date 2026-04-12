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

        let runtime_id =
            std::env::var("GRID_RUNTIME_ID").unwrap_or_else(|_| "grid-harness".into());

        // Read API key based on configured provider; fall back across providers.
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .ok()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok());

        let provider = std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "anthropic".into());

        let model =
            std::env::var("LLM_MODEL").unwrap_or_else(|_| "claude-sonnet-4-20250514".into());

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
        std::env::remove_var("GRID_RUNTIME_ADDR");
        std::env::remove_var("GRID_RUNTIME_ID");
        let config = RuntimeConfig::from_env();
        assert_eq!(config.grpc_addr.port(), 50051);
        assert_eq!(config.runtime_id, "grid-harness");
        assert_eq!(config.provider, "anthropic");
    }
}
