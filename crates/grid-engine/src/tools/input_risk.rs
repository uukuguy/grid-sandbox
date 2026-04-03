//! Input-level risk classification helpers for tools.
//!
//! Provides path and URL risk analysis used by file and web tools
//! to dynamically classify risk based on actual input parameters.

use grid_types::RiskLevel;

/// Classify risk based on a file path.
///
/// - System paths (/etc, /usr, /boot, etc.) → HighRisk
/// - Hidden/config paths (.env, .ssh, .git/config, credentials) → HighRisk
/// - Home directory root operations → HighRisk
/// - Project-relative paths → None (use tool's static risk_level)
pub fn classify_path_risk(path: Option<&str>) -> Option<RiskLevel> {
    let path = match path {
        Some(p) if !p.is_empty() => p,
        _ => return None,
    };

    let path_lower = path.to_lowercase();

    // High risk: system directories
    let system_prefixes = [
        "/etc/", "/usr/", "/boot/", "/var/", "/sys/", "/proc/",
        "/dev/", "/sbin/", "/bin/", "/lib/",
        "c:\\windows", "c:\\system32",
    ];
    if system_prefixes.iter().any(|p| path_lower.starts_with(p)) {
        return Some(RiskLevel::HighRisk);
    }

    // High risk: sensitive files
    let sensitive_patterns = [
        ".env", ".ssh/", ".gnupg/", ".aws/", "credentials",
        "secrets", ".git/config", "id_rsa", "id_ed25519",
        ".npmrc", ".pypirc", "token", "password",
    ];
    if sensitive_patterns.iter().any(|p| path_lower.contains(p)) {
        return Some(RiskLevel::HighRisk);
    }

    // Medium risk: home directory root
    if path.starts_with('~') || path_lower.starts_with("/home/") || path_lower.starts_with("/users/") {
        // But project subdirectories are fine — only flag root-level ops
        let depth = path.matches('/').count();
        if depth <= 2 {
            return Some(RiskLevel::HighRisk);
        }
    }

    None // project-relative path → use static risk_level
}

/// Classify risk based on a URL.
///
/// - Internal/localhost URLs → LowRisk
/// - Known safe domains → LowRisk
/// - Unknown external URLs → None (use tool's static risk_level)
pub fn classify_url_risk(url: Option<&str>) -> Option<RiskLevel> {
    let url = match url {
        Some(u) if !u.is_empty() => u,
        _ => return None,
    };

    let url_lower = url.to_lowercase();

    // Localhost is always safe
    if url_lower.contains("localhost") || url_lower.contains("127.0.0.1") || url_lower.contains("[::1]") {
        return Some(RiskLevel::LowRisk);
    }

    // Known documentation/API domains
    let safe_domains = [
        "github.com", "raw.githubusercontent.com",
        "docs.rs", "crates.io", "npmjs.com",
        "pypi.org", "stackoverflow.com",
        "developer.mozilla.org", "wikipedia.org",
    ];
    if safe_domains.iter().any(|d| url_lower.contains(d)) {
        return Some(RiskLevel::LowRisk);
    }

    None // unknown domain → use static risk_level
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_risk_system_dirs() {
        assert_eq!(classify_path_risk(Some("/etc/passwd")), Some(RiskLevel::HighRisk));
        assert_eq!(classify_path_risk(Some("/usr/bin/something")), Some(RiskLevel::HighRisk));
    }

    #[test]
    fn test_path_risk_sensitive_files() {
        assert_eq!(classify_path_risk(Some("/home/user/.env")), Some(RiskLevel::HighRisk));
        assert_eq!(classify_path_risk(Some("src/.ssh/id_rsa")), Some(RiskLevel::HighRisk));
        assert_eq!(classify_path_risk(Some("credentials.json")), Some(RiskLevel::HighRisk));
    }

    #[test]
    fn test_path_risk_home_directory() {
        assert_eq!(classify_path_risk(Some("~/file.txt")), Some(RiskLevel::HighRisk));
        assert_eq!(classify_path_risk(Some("/home/user")), Some(RiskLevel::HighRisk));
    }

    #[test]
    fn test_path_risk_project_relative() {
        assert_eq!(classify_path_risk(Some("src/main.rs")), None);
        assert_eq!(classify_path_risk(Some("./tests/test.rs")), None);
        assert_eq!(classify_path_risk(Some("Cargo.toml")), None);
    }

    #[test]
    fn test_path_risk_none_or_empty() {
        assert_eq!(classify_path_risk(None), None);
        assert_eq!(classify_path_risk(Some("")), None);
    }

    #[test]
    fn test_url_risk_localhost() {
        assert_eq!(classify_url_risk(Some("http://localhost:3000")), Some(RiskLevel::LowRisk));
        assert_eq!(classify_url_risk(Some("http://127.0.0.1:8080")), Some(RiskLevel::LowRisk));
    }

    #[test]
    fn test_url_risk_safe_domains() {
        assert_eq!(classify_url_risk(Some("https://github.com/user/repo")), Some(RiskLevel::LowRisk));
        assert_eq!(classify_url_risk(Some("https://docs.rs/serde")), Some(RiskLevel::LowRisk));
    }

    #[test]
    fn test_url_risk_unknown_domain() {
        assert_eq!(classify_url_risk(None), None);
        assert_eq!(classify_url_risk(Some("https://example.com/api")), None);
    }
}
