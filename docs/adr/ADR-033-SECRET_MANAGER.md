# ADR-033: Secret Manager

## Status

Accepted

## Date

2026-03-07

## Context

The system requires secure management of sensitive information:
- Encrypted API key storage
- Provider authentication credentials
- Webhook callback addresses
- Database connection strings

## Decision

Implement an AES-GCM based secret manager:

### Core Architecture

```rust
// Secret manager
pub struct SecretManager {
    cipher: Aes256Gcm,
    key_store: Arc<RwLock<HashMap<String, EncryptedSecret>>>,
    keyring: Option<Box<dyn KeyringBackend>>,
}

// Encrypted secret
pub struct EncryptedSecret {
    id: SecretId,
    encrypted_value: Vec<u8>,
    nonce: [u8; 12],
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
```

### Encryption Scheme

- **Algorithm**: AES-256-GCM
- **Key Derivation**: Argon2id
- **Storage**: Encrypted value stored in database

### Keyring Integration

- **Linux**: D-Bus Secret Service
- **macOS**: Keychain
- **Windows**: Credential Manager

### Operation Interface

```rust
pub trait SecretStore: Send + Sync {
    fn get(&self, key: &str) -> Result<String>;
    fn set(&self, key: &str, value: &str) -> Result<()>;
    fn delete(&self, key: &str) -> Result<()>;
    fn list(&self) -> Result<Vec<SecretMeta>>;
}
```

## Consequences

### Positive

- Sensitive data encrypted at rest
- System Keyring integration support
- Audit logging for access

### Negative

- Master key management complexity
- Keyring depends on platform features

## Related

- [ADR-003: API Key HMAC](ADR-003-API_KEY_HMAC.md)
