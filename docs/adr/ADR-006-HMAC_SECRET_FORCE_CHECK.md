# ADR-006-HMAC: HMAC Secret Force Check (Fail-Fast)

## Status
Accepted

## Context

ADR-003 upgraded API Key hashing to HMAC-SHA256, with the secret injected via `OCTO_HMAC_SECRET` environment variable. However, the original implementation only outputs `tracing::warn` when the secret is not set and falls back to hardcoded default `"octo-default-hmac-secret-change-in-production"`:

```rust
const DEFAULT_HMAC_SECRET: &str = "octo-default-hmac-secret-change-in-production";

let hmac_secret = std::env::var("OCTO_HMAC_SECRET").unwrap_or_else(|_| {
    tracing::warn!("OCTO_HMAC_SECRET is not set. Using insecure default...");
    DEFAULT_HMAC_SECRET.to_string()
});
```

Code review (security agent) discovered this default has been committed to a public repository; anyone can obtain it. Using this default in `AuthMode::ApiKey` or `AuthMode::Full` mode means:

1. Attacker knows HMAC Secret → Can forge hash values for any API Key
2. Server cannot distinguish between legitimate and forged Keys → Complete auth bypass
3. Server can start in insecure configuration → Operators may unknowingly deploy vulnerable service

## Decision

Add force check in `AuthConfig::warn_if_insecure()`: when auth mode is non-`None`, if `hmac_secret` still equals hardcoded default, immediately `panic!` to prevent server startup.

```rust
pub fn warn_if_insecure(&self) {
    if self.mode == AuthMode::None {
        tracing::warn!("Authentication is DISABLED (mode=none)...");
    } else if self.hmac_secret == DEFAULT_HMAC_SECRET {
        panic!(
            "OCTO_HMAC_SECRET is not set but authentication is enabled (mode={:?}). \
             The hardcoded default HMAC secret must NOT be used in production because \
             it allows API key hash forgery. Set OCTO_HMAC_SECRET to a strong random \
             secret before starting the server.",
            self.mode
        );
    }
}
```

`warn_if_insecure()` is called after auth config loading in `main.rs`, ensuring the check completes before binding to ports.

## Consequences

### Positive

- **Fail-fast principle**: Configuration errors exposed immediately at startup, not exploited by attackers at runtime
- Production deployments that forget to configure `OCTO_HMAC_SECRET` will not start, forcing operators to fix config
- `AuthMode::None` development mode unaffected (no HMAC Secret needed)

### Negative

- Existing deployments without `OCTO_HMAC_SECRET` configured will fail to start after upgrade, need to configure environment variable first
- `.env.example` needs to explicitly list `OCTO_HMAC_SECRET` and provide generation guidance

### Neutral

- Development environment can bypass this check via `OCTO_AUTH_MODE=none`
- `panic!` chosen over `process::exit(1)` because panic prints full error message and stack trace

## References

- Code paths: `crates/octo-engine/src/auth/config.rs`
- Related: ADR-003 (API Key HMAC)
