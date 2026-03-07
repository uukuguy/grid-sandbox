# ADR-004: Middleware Execution Order Fix (LIFO)

## Status
Accepted

## Context

Axum's `.layer()` follows LIFO (Last In, First Out) semantics: **the last added middleware executes first**.

Before refactoring, the middleware addition order was:

```rust
.layer(audit_middleware)    // First added → Last to execute
.layer(auth_middleware)     // Second added → Executes in middle
.layer(rate_limit)          // Third added → First to execute
```

This should produce correct order `rate_limit → auth → audit`, but there's ambiguity between code comments and actual semantics, causing maintainers to misunderstand the order and risking security breakage if code is rearranged.

A deeper issue: if `audit` middleware runs before `auth`, the `UserContext` extension hasn't been injected yet, so audit logs would be missing user identity information, resulting in incomplete audit records.

## Decision

Explicitly comment on the LIFO middleware addition rules and confirm the correct addition order:

```rust
// Middleware layers use LIFO ordering: last added = first to run.
// Desired execution order: rate_limit → auth → audit
// So we add them in reverse: audit first, rate_limit last.
//
// Audit middleware - logs all requests (runs AFTER auth, so UserContext is available)
.layer(axum::middleware::from_fn_with_state(audit_state, audit_middleware))
// Auth middleware - validates API keys and injects UserContext
.layer(axum::middleware::from_fn_with_state(auth_state, auth_middleware_wrapper))
// Rate limiting middleware (runs FIRST - before auth and audit)
.layer(axum::middleware::from_fn_with_state(rate_limiter, rate_limit_middleware))
```

Final execution order (inbound request direction):

```
Request → rate_limit → auth → audit → Business Handler
Response ← rate_limit ← auth ← audit ← Business Handler
```

This order ensures:

1. `rate_limit`: First checks rate, rejects exceeding requests, reduces invalid auth validation overhead
2. `auth`: Validates API Key and injects `UserContext` into request extensions
3. `audit`: When logging request, can already read `UserContext`, audit logs include user identity

## Consequences

### Positive

- Audit logs now correctly record authenticated user identity, meeting audit compliance requirements
- Rate limit runs before auth, unauthenticated requests are also rate-limited, preventing DoS probing
- Explicit LIFO comments reduce risk of introducing errors during future maintenance

### Negative

- If introducing middleware that needs to run before auth (like CORS preflight), need to pay special attention to LIFO rules
- Axum's LIFO semantics are opposite to Express/Django conventions, adding learning curve for new contributors

### Neutral

- `TraceLayer` and `CorsLayer` are added via separate `.layer()` calls, outside all custom middleware, their execution order is not affected by this change

## References

- Code paths: `crates/octo-server/src/router.rs`
