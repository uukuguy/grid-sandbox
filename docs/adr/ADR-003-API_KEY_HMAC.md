# ADR-003: API Key Hash Algorithm Upgrade to HMAC-SHA256

## Status
Accepted

## Context

The original API Key storage scheme uses saltless SHA-256 hashing. This approach has the following security flaws:

1. **Rainbow Table Attack**: Saltless hashes allow attackers to precompute hash lookup tables for common API Keys. Once the database is leaked, they can batch-reverse lookup original Keys.

2. **Same Key Produces Same Hash**: Identical API Keys in different systems produce identical hashes, compounding cross-system leakage risk.

3. **No Key Binding**: Hash values are not bound to any system secret. Attackers only need database access to perform offline brute force attacks.

HMAC-SHA256 solves these problems by introducing a server-side secret (HMAC Secret). Even if attackers obtain hash values from the database, they cannot reverse-lookup the original Key without knowing the HMAC Secret.

## Decision

Upgrade the API Key hash algorithm from saltless SHA-256 to HMAC-SHA256:

**Implementation changes** (`crates/octo-engine/src/auth/config.rs`):

```rust
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

fn hash_api_key(key: &str, secret: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC accepts any key length");
    mac.update(key.as_bytes());
    format!("{:x}", mac.finalize().into_bytes())
}
```

**Configuration changes**:

- `AuthConfig` adds `hmac_secret: String` field
- Loaded from `OCTO_HMAC_SECRET` environment variable; falls back to hardcoded default and outputs warning log when not set:

  ```rust
  let hmac_secret = std::env::var("OCTO_HMAC_SECRET").unwrap_or_else(|_| {
      tracing::warn!(
          "OCTO_HMAC_SECRET is not set. Using insecure default HMAC secret. \
           Set this environment variable in production."
      );
      DEFAULT_HMAC_SECRET.to_string()
  });
  ```

- `ApiKey::new()` signature changed to `new(key: &str, secret: &str, ...)` to receive HMAC Secret
- `add_api_key()` and `add_api_key_with_role()` automatically use `self.hmac_secret`

## Consequences

### Positive

- Rainbow table attacks are ineffective: hashes are bound to server-side secret, offline cracking requires the secret as well
- Database leak does not equal API Key leak, system has defense in depth
- Production environment injects secret via `OCTO_HMAC_SECRET` environment variable, supports key rotation

### Negative

- **Breaking change**: All API Keys stored with old SHA-256 hashes become invalid immediately, need regeneration
- Before upgrade: (1) Stop service (2) Clear old Key records (3) Re-add Keys with new algorithm
- Different `OCTO_HMAC_SECRET` values produce different hashes; losing the secret invalidates all API Keys

### Neutral

- Development environment still works without `OCTO_HMAC_SECRET`, but outputs security warning on startup
- Default HMAC Secret `"octo-default-hmac-secret-change-in-production"` is explicitly marked as not for production use

## References

- Code paths: `crates/octo-engine/src/auth/config.rs`
- Related: ADR-006-HMAC (HMAC Secret Force Check)
- Migration: Set `OCTO_HMAC_SECRET=<random-strong-password>`, delete old API Key records, re-register with new algorithm
