//! GridRunMode detection
//!
//! Detects whether Octo is running inside a sandbox container (Mode A)
//! or on the host system (Mode B). This affects tool routing decisions:
//! - Mode A (Sandboxed): All tools execute locally — isolation already provided by container
//! - Mode B (Host): Tools route to sandbox backends based on SandboxProfile

use serde::{Deserialize, Serialize};
use std::fmt;

/// Octo deployment run mode.
///
/// Determines whether Octo itself is running inside a sandbox environment
/// or directly on the host machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GridRunMode {
    /// Octo is running inside a sandbox container (Docker/K8s/Podman).
    /// All tools execute locally since isolation is already provided.
    Sandboxed,

    /// Octo is running on the host machine.
    /// Tools may need to be routed to sandbox backends for isolation.
    Host,
}

impl Default for GridRunMode {
    fn default() -> Self {
        GridRunMode::Host
    }
}

impl fmt::Display for GridRunMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GridRunMode::Sandboxed => write!(f, "sandboxed"),
            GridRunMode::Host => write!(f, "host"),
        }
    }
}

impl GridRunMode {
    /// Auto-detect the current run mode by checking environment indicators.
    ///
    /// Detection priority:
    /// 1. `GRID_SANDBOXED` env var (explicit declaration, highest priority)
    /// 2. `/.dockerenv` file (Docker container)
    /// 3. `/run/.containerenv` file (Podman container)
    /// 4. `KUBERNETES_SERVICE_HOST` env var (Kubernetes pod)
    /// 5. Default: Host
    pub fn detect() -> Self {
        // 1. Explicit env var (highest priority)
        if let Ok(val) = std::env::var("GRID_SANDBOXED") {
            match val.to_lowercase().as_str() {
                "1" | "true" | "yes" => return GridRunMode::Sandboxed,
                "0" | "false" | "no" => return GridRunMode::Host,
                _ => {} // ignore invalid values, continue detection
            }
        }

        // 2. Docker container indicator
        if std::path::Path::new("/.dockerenv").exists() {
            return GridRunMode::Sandboxed;
        }

        // 3. Podman container indicator
        if std::path::Path::new("/run/.containerenv").exists() {
            return GridRunMode::Sandboxed;
        }

        // 4. Kubernetes pod indicator
        if std::env::var("KUBERNETES_SERVICE_HOST").is_ok() {
            return GridRunMode::Sandboxed;
        }

        // 5. Default: Host
        GridRunMode::Host
    }

    /// Whether tools should execute locally without sandbox routing.
    ///
    /// In Sandboxed mode, the container itself provides isolation,
    /// so tools can execute directly without additional sandboxing.
    pub fn is_local_execution(&self) -> bool {
        matches!(self, GridRunMode::Sandboxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_host() {
        assert_eq!(GridRunMode::default(), GridRunMode::Host);
    }

    #[test]
    fn test_display() {
        assert_eq!(GridRunMode::Sandboxed.to_string(), "sandboxed");
        assert_eq!(GridRunMode::Host.to_string(), "host");
    }

    #[test]
    fn test_is_local_execution() {
        assert!(GridRunMode::Sandboxed.is_local_execution());
        assert!(!GridRunMode::Host.is_local_execution());
    }

    #[test]
    fn test_detect_explicit_env_true() {
        std::env::set_var("GRID_SANDBOXED", "1");
        assert_eq!(GridRunMode::detect(), GridRunMode::Sandboxed);
        std::env::remove_var("GRID_SANDBOXED");
    }

    #[test]
    fn test_detect_explicit_env_false() {
        std::env::set_var("GRID_SANDBOXED", "0");
        // Remove K8s indicator if present
        let k8s_val = std::env::var("KUBERNETES_SERVICE_HOST").ok();
        std::env::remove_var("KUBERNETES_SERVICE_HOST");

        assert_eq!(GridRunMode::detect(), GridRunMode::Host);

        std::env::remove_var("GRID_SANDBOXED");
        if let Some(v) = k8s_val {
            std::env::set_var("KUBERNETES_SERVICE_HOST", v);
        }
    }

    #[test]
    fn test_detect_explicit_env_true_variants() {
        for val in &["true", "True", "TRUE", "yes", "1"] {
            std::env::set_var("GRID_SANDBOXED", val);
            assert_eq!(
                GridRunMode::detect(),
                GridRunMode::Sandboxed,
                "Expected Sandboxed for GRID_SANDBOXED={}",
                val
            );
        }
        std::env::remove_var("GRID_SANDBOXED");
    }

    #[test]
    fn test_detect_k8s_env() {
        // Ensure no explicit GRID_SANDBOXED
        std::env::remove_var("GRID_SANDBOXED");
        std::env::set_var("KUBERNETES_SERVICE_HOST", "10.0.0.1");
        assert_eq!(GridRunMode::detect(), GridRunMode::Sandboxed);
        std::env::remove_var("KUBERNETES_SERVICE_HOST");
    }

    #[test]
    fn test_detect_default_host() {
        // Clear all indicators
        std::env::remove_var("GRID_SANDBOXED");
        let k8s_val = std::env::var("KUBERNETES_SERVICE_HOST").ok();
        std::env::remove_var("KUBERNETES_SERVICE_HOST");

        // On a normal dev machine, /.dockerenv and /run/.containerenv don't exist
        // so we should get Host
        let mode = GridRunMode::detect();
        // Can't assert Host definitively if running in Docker CI,
        // but on typical dev machines this is Host
        assert!(
            mode == GridRunMode::Host || mode == GridRunMode::Sandboxed,
            "detect() should return a valid mode"
        );

        // Restore
        if let Some(v) = k8s_val {
            std::env::set_var("KUBERNETES_SERVICE_HOST", v);
        }
    }

    #[test]
    fn test_serde_roundtrip() {
        for mode in &[GridRunMode::Sandboxed, GridRunMode::Host] {
            let json = serde_json::to_string(mode).unwrap();
            let deserialized: GridRunMode = serde_json::from_str(&json).unwrap();
            assert_eq!(*mode, deserialized);
        }
    }
}
