//! Sandbox profile configuration
//!
//! SandboxProfile provides a one-line configuration switch that controls
//! all sandbox behavior: which backends are used, whether approval gates
//! are required, timeout limits, audit levels, and environment passthrough.

use serde::{Deserialize, Serialize};
use std::fmt;

use super::traits::SandboxPolicy;

/// Sandbox execution profile — one-line config switch for sandbox behavior.
///
/// Each profile bundles a complete set of sandbox parameters:
/// - `Development`: Zero-friction local execution (no sandbox overhead)
/// - `Staging`: Docker required, no fallback + audit warnings
/// - `Production`: Strict isolation required (Docker/WASM/External only)
/// - `Custom`: User-defined fine-grained control
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SandboxProfile {
    /// Zero-friction development mode.
    /// All tools execute locally via subprocess. No Docker startup delay.
    /// Full environment passthrough. No approval gates.
    Development,

    /// Staging mode — Docker required, no local fallback.
    /// Approval gate for destructive operations.
    /// Limited environment passthrough (no API keys).
    Staging,

    /// Production mode — strict isolation required.
    /// Only Docker, WASM, or External backends allowed.
    /// Subprocess execution rejected. Full audit logging.
    Production,

    /// Custom profile with user-defined parameters.
    Custom(CustomSandboxConfig),
}

impl Default for SandboxProfile {
    fn default() -> Self {
        SandboxProfile::Development
    }
}

impl fmt::Display for SandboxProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SandboxProfile::Development => write!(f, "development"),
            SandboxProfile::Staging => write!(f, "staging"),
            SandboxProfile::Production => write!(f, "production"),
            SandboxProfile::Custom(_) => write!(f, "custom"),
        }
    }
}

impl SandboxProfile {
    /// Get the SandboxPolicy corresponding to this profile.
    pub fn policy(&self) -> SandboxPolicy {
        match self {
            SandboxProfile::Development => SandboxPolicy::Development,
            SandboxProfile::Staging => SandboxPolicy::Preferred,
            SandboxProfile::Production => SandboxPolicy::Strict,
            SandboxProfile::Custom(c) => c.policy,
        }
    }

    /// Whether environment variables (including API keys) should be passed through.
    pub fn env_passthrough(&self) -> bool {
        match self {
            SandboxProfile::Development => true,
            SandboxProfile::Staging => false,
            SandboxProfile::Production => false,
            SandboxProfile::Custom(c) => c.env_passthrough,
        }
    }

    /// Whether an approval gate is required before destructive tool execution.
    pub fn approval_gate(&self) -> bool {
        match self {
            SandboxProfile::Development => false,
            SandboxProfile::Staging => true,
            SandboxProfile::Production => true,
            SandboxProfile::Custom(c) => c.approval_gate,
        }
    }

    /// Default execution timeout in seconds.
    pub fn timeout_secs(&self) -> u64 {
        match self {
            SandboxProfile::Development => 120,
            SandboxProfile::Staging => 60,
            SandboxProfile::Production => 30,
            SandboxProfile::Custom(c) => c.timeout_secs,
        }
    }

    /// Audit level: "none", "warnings", "full".
    pub fn audit_level(&self) -> &str {
        match self {
            SandboxProfile::Development => "none",
            SandboxProfile::Staging => "warnings",
            SandboxProfile::Production => "full",
            SandboxProfile::Custom(c) => &c.audit_level,
        }
    }

    /// Memory limit in bytes for Docker containers.
    /// Development: unlimited, Staging: 2GB, Production: 1GB
    pub fn memory_limit(&self) -> Option<i64> {
        match self {
            SandboxProfile::Development => None,
            SandboxProfile::Staging => Some(2 * 1024 * 1024 * 1024), // 2 GiB
            SandboxProfile::Production => Some(1024 * 1024 * 1024),  // 1 GiB
            SandboxProfile::Custom(c) => c.memory_limit,
        }
    }

    /// CPU quota (in microseconds per 100ms period).
    /// Development: unlimited, Staging: 200% (2 cores), Production: 100% (1 core)
    pub fn cpu_quota(&self) -> Option<i64> {
        match self {
            SandboxProfile::Development => None,
            SandboxProfile::Staging => Some(200_000),  // 200% = 2 cores
            SandboxProfile::Production => Some(100_000), // 100% = 1 core
            SandboxProfile::Custom(c) => c.cpu_quota,
        }
    }

    /// Network mode for Docker containers.
    /// Development: "bridge" (full access), Staging: "bridge", Production: "none" (no network)
    pub fn network_mode(&self) -> &str {
        match self {
            SandboxProfile::Development => "bridge",
            SandboxProfile::Staging => "bridge",
            SandboxProfile::Production => "none",
            SandboxProfile::Custom(c) => c.network_mode.as_deref().unwrap_or("bridge"),
        }
    }

    /// Resolve SandboxProfile from multiple sources with priority:
    /// `--sandbox-bypass` > `--sandbox-profile` > `GRID_SANDBOX_PROFILE` > config > default
    pub fn resolve(
        bypass: bool,
        cli_profile: Option<&str>,
        config_profile: Option<&str>,
    ) -> Self {
        if bypass {
            return SandboxProfile::Development;
        }

        // CLI flag
        if let Some(p) = cli_profile {
            if let Some(profile) = Self::from_str_opt(p) {
                return profile;
            }
        }

        // Environment variable
        if let Ok(env_val) = std::env::var("GRID_SANDBOX_PROFILE") {
            if let Some(profile) = Self::from_str_opt(&env_val) {
                return profile;
            }
        }

        // Config file
        if let Some(p) = config_profile {
            if let Some(profile) = Self::from_str_opt(p) {
                return profile;
            }
        }

        // Default
        SandboxProfile::default()
    }

    /// Parse a profile name string into a SandboxProfile.
    /// Returns None for unrecognized names (Custom requires structured config).
    fn from_str_opt(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "development" | "dev" => Some(SandboxProfile::Development),
            "staging" | "stg" => Some(SandboxProfile::Staging),
            "production" | "prod" => Some(SandboxProfile::Production),
            _ => None,
        }
    }
}

/// Custom sandbox configuration for fine-grained control.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomSandboxConfig {
    /// Sandbox execution policy
    #[serde(default)]
    pub policy: SandboxPolicy,
    /// Whether to pass environment variables through to sandbox
    #[serde(default)]
    pub env_passthrough: bool,
    /// Whether approval gate is required for destructive operations
    #[serde(default)]
    pub approval_gate: bool,
    /// Execution timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// Audit level: "none", "warnings", "full"
    #[serde(default = "default_audit_level")]
    pub audit_level: String,
    /// Memory limit in bytes for Docker containers (None = unlimited)
    #[serde(default)]
    pub memory_limit: Option<i64>,
    /// CPU quota in microseconds per 100ms period (None = unlimited)
    #[serde(default)]
    pub cpu_quota: Option<i64>,
    /// Docker network mode (None defaults to "bridge")
    #[serde(default)]
    pub network_mode: Option<String>,
}

fn default_timeout() -> u64 {
    60
}

fn default_audit_level() -> String {
    "warnings".to_string()
}

impl Default for CustomSandboxConfig {
    fn default() -> Self {
        Self {
            policy: SandboxPolicy::Preferred,
            env_passthrough: false,
            approval_gate: true,
            timeout_secs: default_timeout(),
            audit_level: default_audit_level(),
            memory_limit: None,
            cpu_quota: None,
            network_mode: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_development() {
        assert_eq!(SandboxProfile::default(), SandboxProfile::Development);
    }

    #[test]
    fn test_development_profile_params() {
        let p = SandboxProfile::Development;
        assert_eq!(p.policy(), SandboxPolicy::Development);
        assert!(p.env_passthrough());
        assert!(!p.approval_gate());
        assert_eq!(p.timeout_secs(), 120);
        assert_eq!(p.audit_level(), "none");
    }

    #[test]
    fn test_staging_profile_params() {
        let p = SandboxProfile::Staging;
        assert_eq!(p.policy(), SandboxPolicy::Preferred);
        assert!(!p.env_passthrough());
        assert!(p.approval_gate());
        assert_eq!(p.timeout_secs(), 60);
        assert_eq!(p.audit_level(), "warnings");
    }

    #[test]
    fn test_production_profile_params() {
        let p = SandboxProfile::Production;
        assert_eq!(p.policy(), SandboxPolicy::Strict);
        assert!(!p.env_passthrough());
        assert!(p.approval_gate());
        assert_eq!(p.timeout_secs(), 30);
        assert_eq!(p.audit_level(), "full");
    }

    #[test]
    fn test_custom_profile_params() {
        let custom = CustomSandboxConfig {
            policy: SandboxPolicy::Strict,
            env_passthrough: true,
            approval_gate: false,
            timeout_secs: 90,
            audit_level: "full".to_string(),
            memory_limit: Some(512 * 1024 * 1024),
            cpu_quota: Some(50_000),
            network_mode: Some("host".to_string()),
        };
        let p = SandboxProfile::Custom(custom);
        assert_eq!(p.policy(), SandboxPolicy::Strict);
        assert!(p.env_passthrough());
        assert!(!p.approval_gate());
        assert_eq!(p.timeout_secs(), 90);
        assert_eq!(p.audit_level(), "full");
        assert_eq!(p.memory_limit(), Some(512 * 1024 * 1024));
        assert_eq!(p.cpu_quota(), Some(50_000));
        assert_eq!(p.network_mode(), "host");
    }

    #[test]
    fn test_display() {
        assert_eq!(SandboxProfile::Development.to_string(), "development");
        assert_eq!(SandboxProfile::Staging.to_string(), "staging");
        assert_eq!(SandboxProfile::Production.to_string(), "production");
        assert_eq!(
            SandboxProfile::Custom(CustomSandboxConfig::default()).to_string(),
            "custom"
        );
    }

    #[test]
    fn test_from_str_opt() {
        assert_eq!(
            SandboxProfile::from_str_opt("development"),
            Some(SandboxProfile::Development)
        );
        assert_eq!(
            SandboxProfile::from_str_opt("dev"),
            Some(SandboxProfile::Development)
        );
        assert_eq!(
            SandboxProfile::from_str_opt("staging"),
            Some(SandboxProfile::Staging)
        );
        assert_eq!(
            SandboxProfile::from_str_opt("stg"),
            Some(SandboxProfile::Staging)
        );
        assert_eq!(
            SandboxProfile::from_str_opt("production"),
            Some(SandboxProfile::Production)
        );
        assert_eq!(
            SandboxProfile::from_str_opt("prod"),
            Some(SandboxProfile::Production)
        );
        assert_eq!(SandboxProfile::from_str_opt("unknown"), None);
    }

    #[test]
    fn test_resolve_bypass_wins() {
        let p = SandboxProfile::resolve(true, Some("production"), Some("staging"));
        assert_eq!(p, SandboxProfile::Development);
    }

    #[test]
    fn test_resolve_cli_over_config() {
        let p = SandboxProfile::resolve(false, Some("staging"), Some("production"));
        assert_eq!(p, SandboxProfile::Staging);
    }

    #[test]
    fn test_resolve_config_fallback() {
        let p = SandboxProfile::resolve(false, None, Some("production"));
        assert_eq!(p, SandboxProfile::Production);
    }

    #[test]
    fn test_resolve_default() {
        // Clear env var if set
        std::env::remove_var("GRID_SANDBOX_PROFILE");
        let p = SandboxProfile::resolve(false, None, None);
        assert_eq!(p, SandboxProfile::Development);
    }

    #[test]
    fn test_resolve_env_var() {
        std::env::set_var("GRID_SANDBOX_PROFILE", "production");
        let p = SandboxProfile::resolve(false, None, None);
        assert_eq!(p, SandboxProfile::Production);
        std::env::remove_var("GRID_SANDBOX_PROFILE");
    }

    #[test]
    fn test_serde_roundtrip() {
        let profiles = vec![
            SandboxProfile::Development,
            SandboxProfile::Staging,
            SandboxProfile::Production,
        ];
        for profile in profiles {
            let json = serde_json::to_string(&profile).unwrap();
            let deserialized: SandboxProfile = serde_json::from_str(&json).unwrap();
            assert_eq!(profile, deserialized);
        }
    }

    #[test]
    fn test_memory_limit() {
        assert!(SandboxProfile::Development.memory_limit().is_none());
        assert_eq!(
            SandboxProfile::Staging.memory_limit(),
            Some(2 * 1024 * 1024 * 1024)
        );
        assert_eq!(
            SandboxProfile::Production.memory_limit(),
            Some(1024 * 1024 * 1024)
        );
    }

    #[test]
    fn test_cpu_quota() {
        assert!(SandboxProfile::Development.cpu_quota().is_none());
        assert_eq!(SandboxProfile::Staging.cpu_quota(), Some(200_000));
        assert_eq!(SandboxProfile::Production.cpu_quota(), Some(100_000));
    }

    #[test]
    fn test_network_mode() {
        assert_eq!(SandboxProfile::Development.network_mode(), "bridge");
        assert_eq!(SandboxProfile::Staging.network_mode(), "bridge");
        assert_eq!(SandboxProfile::Production.network_mode(), "none");
    }

    #[test]
    fn test_custom_network_mode_default() {
        let custom = CustomSandboxConfig {
            network_mode: None,
            ..Default::default()
        };
        let p = SandboxProfile::Custom(custom);
        assert_eq!(p.network_mode(), "bridge");
    }

    #[test]
    fn test_custom_serde_roundtrip() {
        let custom = SandboxProfile::Custom(CustomSandboxConfig {
            policy: SandboxPolicy::Strict,
            env_passthrough: true,
            approval_gate: false,
            timeout_secs: 45,
            audit_level: "full".to_string(),
            memory_limit: Some(1024 * 1024 * 1024),
            cpu_quota: Some(100_000),
            network_mode: Some("none".to_string()),
        });
        let json = serde_json::to_string(&custom).unwrap();
        let deserialized: SandboxProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(custom, deserialized);
    }
}
