use octo_engine::auth::AuthConfigYaml;
use octo_engine::providers::{ProviderChainConfig, ProviderConfig, SmartRoutingConfig};
use octo_engine::scheduler::SchedulerConfig;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Sync configuration (offline-first sync with HLC timestamps)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Enable sync (default: false)
    #[serde(default)]
    pub enabled: bool,
    /// Node identifier (auto-generated UUID if not set)
    #[serde(default)]
    pub node_id: Option<String>,
}

/// TLS configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Enable TLS (default: false)
    #[serde(default)]
    pub enabled: bool,
    /// Path to PEM certificate file
    #[serde(default)]
    pub cert_path: Option<PathBuf>,
    /// Path to PEM private key file
    #[serde(default)]
    pub key_path: Option<PathBuf>,
    /// Auto-generate self-signed certificate (default: false)
    #[serde(default)]
    pub self_signed: bool,
    /// Directory for self-signed cert output (default: ./data/tls)
    #[serde(default)]
    pub self_signed_dir: Option<PathBuf>,
}

/// Main configuration for octo-server
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Server configuration
    #[serde(default)]
    pub server: ServerConfig,
    /// LLM provider configuration
    #[serde(default)]
    pub provider: ProviderConfig,
    /// Database configuration
    #[serde(default)]
    pub database: DatabaseConfig,
    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,
    /// MCP configuration
    #[serde(default)]
    pub mcp: McpConfig,
    /// Skills configuration
    #[serde(default)]
    pub skills: SkillsConfig,
    /// Tools configuration
    #[serde(default)]
    pub tools: ToolsConfig,
    /// Auth configuration (optional)
    #[serde(default)]
    pub auth: AuthConfigYaml,
    /// Scheduler configuration
    #[serde(default)]
    pub scheduler: SchedulerConfig,
    /// Provider Chain configuration (optional)
    #[serde(default)]
    pub provider_chain: Option<ProviderChainConfig>,
    /// Working directory for sandbox (optional)
    #[serde(default)]
    pub working_dir: Option<String>,
    /// Enable event bus for observability (default: false)
    #[serde(default)]
    pub enable_event_bus: bool,
    /// Smart routing configuration (optional)
    #[serde(default)]
    pub smart_routing: Option<SmartRoutingConfig>,
    /// TLS configuration
    #[serde(default)]
    pub tls: TlsConfig,
    /// Sync configuration (offline-first sync)
    #[serde(default)]
    pub sync: SyncConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server host (default: 127.0.0.1)
    pub host: String,
    /// Server port (default: 3001)
    pub port: u16,
    /// Allowed CORS origins (empty = allow all)
    #[serde(default)]
    pub cors_origins: Vec<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3001,
            cors_origins: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// SQLite database path
    pub path: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: "./data/octo.db".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// RUST_LOG filter string
    pub level: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "octo_server=info,octo_engine=info,tower_http=info".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpConfig {
    /// MCP servers directory (optional)
    pub servers_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillsConfig {
    /// Skills directories to load from
    pub dirs: Vec<String>,
}

/// Tools configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolsConfig {
    /// Web search engine priority order (e.g., ["jina", "tavily", "ddg"])
    #[serde(default)]
    pub web_search_priority: Vec<String>,
}

impl Config {
    /// Load configuration with priority: config.yaml < CLI args < .env
    ///
    /// Priority (lowest to highest):
    /// 1. config.yaml - base configuration file
    /// 2. CLI arguments - e.g., --port 4000
    /// 3. Environment variables (.env) - highest priority for overrides
    pub fn load(
        config_path: Option<&PathBuf>,
        cli_port: Option<u16>,
        cli_host: Option<&str>,
    ) -> Self {
        // Step 1: Load from config.yaml (lowest priority)
        // If no explicit path given, look for config.yaml in current directory
        let yaml_path = config_path
            .map(|p| p.as_path())
            .unwrap_or_else(|| Path::new("config.yaml"));

        let mut config = if yaml_path.exists() {
            if let Ok(content) = std::fs::read_to_string(yaml_path) {
                match serde_yaml::from_str::<Config>(&content) {
                    Ok(cfg) => Some(cfg),
                    Err(e) => {
                        tracing::warn!("Failed to parse config.yaml: {}, using defaults", e);
                        None
                    }
                }
            } else {
                None
            }
        } else {
            tracing::debug!("Config file {:?} not found, using defaults", yaml_path);
            None
        }
        .unwrap_or_default();

        // Step 2: CLI arguments override config.yaml
        if let Some(port) = cli_port {
            config.server.port = port;
        }
        if let Some(host) = cli_host {
            config.server.host = host.to_string();
        }

        // Step 3: Environment variables have highest priority (override everything)
        // Server
        if let Ok(host) = std::env::var("OCTO_HOST") {
            config.server.host = host;
        }
        if let Ok(port) = std::env::var("OCTO_PORT") {
            if let Ok(p) = port.parse() {
                config.server.port = p;
            }
        }

        // CORS origins
        if let Ok(origins) = std::env::var("OCTO_CORS_ORIGINS") {
            config.server.cors_origins = origins
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }

        // Provider - read first to determine other fields
        if let Ok(provider_name) = std::env::var("LLM_PROVIDER") {
            config.provider.name = provider_name;
        }

        // Read api_key, base_url, model based on provider
        match config.provider.name.as_str() {
            "openai" => {
                if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
                    if !api_key.is_empty() {
                        config.provider.api_key = Some(api_key);
                    }
                }
                if let Ok(url) = std::env::var("OPENAI_BASE_URL") {
                    config.provider.base_url = Some(url);
                }
                if let Ok(model) = std::env::var("OPENAI_MODEL_NAME") {
                    config.provider.model = Some(model);
                }
            }
            _ => {
                // Default to anthropic
                if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
                    if !api_key.is_empty() {
                        config.provider.api_key = Some(api_key);
                    }
                }
                if let Ok(url) = std::env::var("ANTHROPIC_BASE_URL") {
                    config.provider.base_url = Some(url);
                }
                if let Ok(model) = std::env::var("ANTHROPIC_MODEL_NAME") {
                    config.provider.model = Some(model);
                }
            }
        }

        // Database
        if let Ok(path) = std::env::var("OCTO_DB_PATH") {
            config.database.path = path;
        }

        // Logging
        if let Ok(level) = std::env::var("RUST_LOG") {
            config.logging.level = level;
        }

        // Working directory
        if let Ok(dir) = std::env::var("OCTO_WORKING_DIR") {
            config.working_dir = Some(dir);
        }

        // Event bus
        if let Ok(enabled) = std::env::var("OCTO_ENABLE_EVENT_BUS") {
            config.enable_event_bus = enabled.parse().unwrap_or(false);
        }

        // TLS
        if let Ok(enabled) = std::env::var("OCTO_TLS_ENABLED") {
            config.tls.enabled = enabled.parse().unwrap_or(false);
        }
        if let Ok(path) = std::env::var("OCTO_TLS_CERT_PATH") {
            config.tls.cert_path = Some(PathBuf::from(path));
        }
        if let Ok(path) = std::env::var("OCTO_TLS_KEY_PATH") {
            config.tls.key_path = Some(PathBuf::from(path));
        }
        if let Ok(v) = std::env::var("OCTO_TLS_SELF_SIGNED") {
            config.tls.self_signed = v.parse().unwrap_or(false);
        }

        // Auth: OCTO_AUTH_MODE and OCTO_API_KEY override config.yaml
        if let Ok(mode) = std::env::var("OCTO_AUTH_MODE") {
            let m = match mode.to_lowercase().as_str() {
                "none" => Some(octo_engine::auth::AuthMode::None),
                "api_key" | "apikey" => Some(octo_engine::auth::AuthMode::ApiKey),
                "full" => Some(octo_engine::auth::AuthMode::Full),
                _ => None,
            };
            if let Some(m) = m {
                config.auth.mode = Some(m);
            }
        }
        if let Ok(key) = std::env::var("OCTO_API_KEY") {
            if !key.is_empty() {
                use octo_engine::auth::ApiKeyConfig;
                let keys = config.auth.api_keys.get_or_insert_with(Vec::new);
                keys.push(ApiKeyConfig {
                    key,
                    user_id: Some(
                        std::env::var("OCTO_API_KEY_USER").unwrap_or_else(|_| "default".into()),
                    ),
                    permissions: vec!["read".into(), "write".into(), "admin".into()],
                    role: None,
                    expires_at: None,
                });
            }
        }

        config
    }

    /// Generate a default config.yaml with all parameters commented
    /// Programmatically generated from actual code defaults
    pub fn generate_default_yaml() -> String {
        let defaults = Self::default();
        let mut output = String::new();

        output.push_str(
            "# =============================================================================\n",
        );
        output.push_str("# Octo Server Configuration\n");
        output.push_str(
            "# =============================================================================\n",
        );
        output.push_str(
            "# Copy this file to config.yaml and uncomment/modify parameters as needed.\n",
        );
        output.push_str("# Priority: config.yaml < CLI args < .env\n");
        output.push_str(
            "# =============================================================================\n\n",
        );

        // Server
        output.push_str("# Server configuration\n");
        output.push_str("# server:\n");
        output.push_str(&format!(
            "#   host: {}    # Server bind address\n",
            defaults.server.host
        ));
        output.push_str(&format!(
            "#   port: {}          # Server port\n\n",
            defaults.server.port
        ));

        // Provider
        output.push_str("# LLM Provider configuration\n");
        output.push_str("# provider:\n");
        output.push_str(&format!(
            "#   name: {}     # Provider: anthropic or openai\n",
            defaults.provider.name
        ));
        output.push_str("#   api_key: \"\"         # API key (required)\n");
        output.push_str(&format!(
            "#   base_url: {:?}      # Optional proxy URL\n",
            defaults.provider.base_url
        ));
        output.push_str(&format!(
            "#   model: {:?}         # Optional model override\n\n",
            defaults.provider.model
        ));

        // Database
        output.push_str("# Database configuration\n");
        output.push_str("# database:\n");
        output.push_str(&format!(
            "#   path: {}  # SQLite database path\n\n",
            defaults.database.path
        ));

        // Logging
        output.push_str("# Logging configuration\n");
        output.push_str("# logging:\n");
        output.push_str(&format!("#   level: {}\n\n", defaults.logging.level));

        // MCP
        output.push_str("# MCP configuration\n");
        output.push_str("# mcp:\n");
        output.push_str(&format!(
            "#   servers_dir: {:?}   # Optional MCP servers directory\n\n",
            defaults.mcp.servers_dir
        ));

        // Skills
        output.push_str("# Skills configuration\n");
        output.push_str("# skills:\n");
        output.push_str(&format!(
            "#   dirs: {:?}            # List of skills directories\n\n",
            defaults.skills.dirs
        ));

        // Working directory
        output.push_str("# Working directory for sandbox (optional)\n");
        output.push_str("# working_dir: \"./data/sandbox\"   # Optional working directory\n\n");

        // Event bus
        output.push_str("# Enable event bus for observability\n");
        output.push_str(&format!(
            "# enable_event_bus: {}    # Enable event bus (default: false)\n",
            defaults.enable_event_bus
        ));

        // TLS
        output.push_str("\n# TLS configuration\n");
        output.push_str("# tls:\n");
        output.push_str("#   enabled: false        # Enable HTTPS\n");
        output.push_str("#   cert_path: null       # Path to PEM certificate\n");
        output.push_str("#   key_path: null        # Path to PEM private key\n");
        output.push_str("#   self_signed: false    # Auto-generate self-signed cert\n");
        output.push_str("#   self_signed_dir: null # Output dir for self-signed certs\n");

        // Auth
        output.push_str("\n# Auth configuration\n");
        output.push_str("# Configure via environment variables (recommended) or inline below.\n");
        output.push_str("#\n");
        output.push_str("# Option 1: Disable auth (local dev only)\n");
        output.push_str("#   OCTO_AUTH_MODE=none\n");
        output.push_str("#\n");
        output.push_str("# Option 2: API key auth\n");
        output.push_str("#   OCTO_AUTH_MODE=api_key\n");
        output.push_str("#   OCTO_API_KEY=your-secret-key     # key clients use in Authorization: Bearer header\n");
        output.push_str(
            "#   OCTO_API_KEY_USER=dev            # optional user id (default: \"default\")\n",
        );
        output.push_str("#\n");
        output.push_str("# auth:\n");
        output.push_str("#   mode: api_key   # none | api_key\n");
        output.push_str("#   api_keys:\n");
        output.push_str("#     - key: \"your-secret-key\"\n");
        output.push_str("#       user_id: \"dev\"\n");
        output.push_str("#       permissions: [\"read\", \"write\", \"admin\"]\n");

        // Scheduler
        output.push_str("\n# Scheduler configuration\n");
        output.push_str("# scheduler:\n");
        output.push_str(&format!(
            "#   enabled: {}          # Enable cron scheduler\n",
            defaults.scheduler.enabled
        ));
        output.push_str(&format!(
            "#   check_interval_secs: {}  # Interval between task checks (seconds)\n",
            defaults.scheduler.check_interval_secs
        ));
        output.push_str(&format!(
            "#   max_concurrent: {}       # Max concurrent scheduled tasks\n",
            defaults.scheduler.max_concurrent
        ));

        // Provider Chain
        output.push_str("\n# Provider chain configuration (optional, multi-provider failover)\n");
        output.push_str("# provider_chain:\n");
        output.push_str("#   failover_policy: automatic  # automatic | manual\n");
        output.push_str("#   health_check_interval_sec: 30\n");
        output.push_str("#   instances:\n");
        output.push_str("#     - name: primary\n");
        output.push_str("#       provider: anthropic\n");
        output.push_str("#       model: claude-sonnet-4-20250514\n");
        output.push_str("#       priority: 1\n");
        output.push_str("#     - name: fallback\n");
        output.push_str("#       provider: openai\n");
        output.push_str("#       model: gpt-4o\n");
        output.push_str("#       priority: 2\n");

        // Smart Routing
        output.push_str("\n# Smart routing configuration (optional, complexity-based model routing)\n");
        output.push_str("# smart_routing:\n");
        output.push_str("#   enabled: false         # Enable smart routing\n");
        output.push_str("#   default_tier: medium    # Default tier: low | medium | high\n");
        output.push_str("#   tiers: {}               # Custom tier configurations\n");
        output.push_str("#   thresholds: null        # Custom analyzer thresholds\n");

        // Sync
        output.push_str("\n# Sync configuration (offline-first sync with HLC timestamps)\n");
        output.push_str("# sync:\n");
        output.push_str(&format!(
            "#   enabled: {}          # Enable offline sync\n",
            defaults.sync.enabled
        ));
        output.push_str("#   node_id: null          # Node identifier (auto-generated UUID if not set)\n");

        output
    }
}
