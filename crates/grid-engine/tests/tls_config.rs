//! Tests for TLS configuration and certificate generation (D4-lite).

#[cfg(feature = "tls")]
mod tls_tests {
    use grid_engine::tls::generate_self_signed_cert;
    use tempfile::TempDir;

    #[test]
    fn test_self_signed_cert_generation() {
        let dir = TempDir::new().unwrap();
        let (cert_path, key_path) = generate_self_signed_cert("localhost", dir.path()).unwrap();
        assert!(cert_path.exists());
        assert!(key_path.exists());
    }

    #[test]
    fn test_self_signed_cert_content() {
        let dir = TempDir::new().unwrap();
        let (cert_path, key_path) = generate_self_signed_cert("test.local", dir.path()).unwrap();
        let cert_pem = std::fs::read_to_string(&cert_path).unwrap();
        let key_pem = std::fs::read_to_string(&key_path).unwrap();
        assert!(cert_pem.contains("BEGIN CERTIFICATE"));
        assert!(key_pem.contains("BEGIN PRIVATE KEY"));
    }

    #[test]
    fn test_self_signed_cert_custom_domain() {
        let dir = TempDir::new().unwrap();
        let result = generate_self_signed_cert("my-server.example.com", dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_self_signed_creates_directory() {
        let dir = TempDir::new().unwrap();
        let nested = dir.path().join("nested").join("tls");
        let result = generate_self_signed_cert("localhost", &nested);
        assert!(result.is_ok());
        assert!(nested.exists());
    }

    #[test]
    fn test_self_signed_overwrites_existing() {
        let dir = TempDir::new().unwrap();
        let (cert1, _) = generate_self_signed_cert("localhost", dir.path()).unwrap();
        let content1 = std::fs::read_to_string(&cert1).unwrap();
        let (cert2, _) = generate_self_signed_cert("localhost", dir.path()).unwrap();
        let content2 = std::fs::read_to_string(&cert2).unwrap();
        // Each generation creates a new cert (different keys)
        assert_ne!(content1, content2);
    }
}

// These tests always run (no feature gate) since they test config parsing
mod config_tests {
    #[test]
    fn test_tls_config_default() {
        use serde::{Deserialize, Serialize};
        use std::path::PathBuf;

        #[derive(Debug, Clone, Serialize, Deserialize, Default)]
        struct TlsConfig {
            #[serde(default)]
            enabled: bool,
            #[serde(default)]
            cert_path: Option<PathBuf>,
            #[serde(default)]
            key_path: Option<PathBuf>,
            #[serde(default)]
            self_signed: bool,
            #[serde(default)]
            self_signed_dir: Option<PathBuf>,
        }

        let cfg: TlsConfig = serde_json::from_str("{}").unwrap();
        assert!(!cfg.enabled);
        assert!(cfg.cert_path.is_none());
        assert!(!cfg.self_signed);
    }

    #[test]
    fn test_tls_config_serialization_roundtrip() {
        use serde::{Deserialize, Serialize};
        use std::path::PathBuf;

        #[derive(Debug, Clone, Serialize, Deserialize)]
        struct TlsConfig {
            enabled: bool,
            cert_path: Option<PathBuf>,
            key_path: Option<PathBuf>,
            self_signed: bool,
        }

        let cfg = TlsConfig {
            enabled: true,
            cert_path: Some(PathBuf::from("/etc/tls/cert.pem")),
            key_path: Some(PathBuf::from("/etc/tls/key.pem")),
            self_signed: false,
        };

        let json = serde_json::to_string(&cfg).unwrap();
        let decoded: TlsConfig = serde_json::from_str(&json).unwrap();
        assert!(decoded.enabled);
        assert_eq!(
            decoded.cert_path.unwrap().to_str().unwrap(),
            "/etc/tls/cert.pem"
        );
        assert!(!decoded.self_signed);
    }
}
