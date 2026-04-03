//! Self-signed certificate generation for development TLS
//!
//! Uses `rcgen` to generate a self-signed certificate + private key pair
//! for local development and testing.

use anyhow::Result;
use std::path::Path;

/// Generate a self-signed certificate and private key for development use.
///
/// Writes `cert.pem` and `key.pem` to the specified directory.
/// Returns the paths to the generated files.
#[cfg(feature = "dashboard-tls")]
pub fn generate_self_signed_cert(output_dir: &Path) -> Result<(String, String)> {
    use rcgen::{generate_simple_self_signed, CertifiedKey};

    std::fs::create_dir_all(output_dir)?;

    let subject_alt_names = vec![
        "localhost".to_string(),
        "127.0.0.1".to_string(),
        "0.0.0.0".to_string(),
    ];

    let CertifiedKey { cert, key_pair } = generate_simple_self_signed(subject_alt_names)?;

    let cert_path = output_dir.join("cert.pem");
    let key_path = output_dir.join("key.pem");

    std::fs::write(&cert_path, cert.pem())?;
    std::fs::write(&key_path, key_pair.serialize_pem())?;

    eprintln!("Generated self-signed certificate:");
    eprintln!("  Certificate: {}", cert_path.display());
    eprintln!("  Private key: {}", key_path.display());
    eprintln!("  SANs: localhost, 127.0.0.1, 0.0.0.0");
    eprintln!();

    Ok((
        cert_path.to_string_lossy().to_string(),
        key_path.to_string_lossy().to_string(),
    ))
}

/// Stub when TLS feature is not enabled
#[cfg(not(feature = "dashboard-tls"))]
pub fn generate_self_signed_cert(_output_dir: &Path) -> Result<(String, String)> {
    anyhow::bail!(
        "Certificate generation requires the 'dashboard-tls' feature. \
         Rebuild with: cargo build --features dashboard-tls"
    );
}
