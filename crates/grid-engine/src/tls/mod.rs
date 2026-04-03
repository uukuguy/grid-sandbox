//! TLS utilities for self-signed certificate generation.

use std::path::{Path, PathBuf};

use anyhow::Result;

/// Generate a self-signed TLS certificate and private key.
///
/// Creates PEM files at `output_dir/cert.pem` and `output_dir/key.pem`.
/// Returns `(cert_path, key_path)`.
#[cfg(feature = "tls")]
pub fn generate_self_signed_cert(
    domain: &str,
    output_dir: &Path,
) -> Result<(PathBuf, PathBuf)> {
    use anyhow::Context;
    use rcgen::{generate_simple_self_signed, CertifiedKey};

    std::fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "failed to create TLS output directory: {}",
            output_dir.display()
        )
    })?;

    let cert_path = output_dir.join("cert.pem");
    let key_path = output_dir.join("key.pem");

    let subject_alt_names = vec![domain.to_string()];
    let CertifiedKey { cert, key_pair } =
        generate_simple_self_signed(subject_alt_names)
            .context("failed to generate self-signed certificate")?;

    std::fs::write(&cert_path, cert.pem())
        .with_context(|| format!("failed to write certificate: {}", cert_path.display()))?;
    std::fs::write(&key_path, key_pair.serialize_pem())
        .with_context(|| format!("failed to write private key: {}", key_path.display()))?;

    Ok((cert_path, key_path))
}

/// Stub when TLS feature is not enabled.
#[cfg(not(feature = "tls"))]
pub fn generate_self_signed_cert(
    _domain: &str,
    _output_dir: &Path,
) -> Result<(PathBuf, PathBuf)> {
    anyhow::bail!(
        "Certificate generation requires the 'tls' feature. \
         Rebuild with: cargo build -p octo-engine --features tls"
    );
}
