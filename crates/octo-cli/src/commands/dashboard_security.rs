//! Dashboard security layers — CORS + security response headers
//!
//! Provides CorsLayer configuration and a security headers middleware
//! for the embedded Dashboard.

use axum::{http::header, middleware::Next, response::Response};
use tower_http::cors::{AllowOrigin, CorsLayer};

/// Build a CORS layer from the allowed origins list.
///
/// - If `origins` is empty, allows same-origin only (no CORS headers).
/// - If `origins` contains `"*"`, allows any origin.
/// - Otherwise, allows only the specified origins.
pub fn build_cors_layer(origins: &[String]) -> CorsLayer {
    if origins.is_empty() {
        // No CORS headers — same-origin requests only
        return CorsLayer::new();
    }

    if origins.iter().any(|o| o == "*") {
        return CorsLayer::permissive();
    }

    let allowed: Vec<header::HeaderValue> = origins
        .iter()
        .filter_map(|o| o.parse().ok())
        .collect();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(allowed))
        .allow_methods(tower_http::cors::Any)
        .allow_headers(vec![
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
        ])
        .max_age(std::time::Duration::from_secs(3600))
}

/// Security headers middleware — adds HSTS, CSP, X-Frame-Options, etc.
pub async fn security_headers_middleware(
    req: axum::extract::Request,
    next: Next,
) -> Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();

    // Strict Transport Security (only meaningful over HTTPS, but safe to set always)
    headers.insert(
        header::STRICT_TRANSPORT_SECURITY,
        "max-age=31536000; includeSubDomains".parse().unwrap(),
    );

    // Content Security Policy
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        "default-src 'self'; script-src 'self' 'unsafe-inline' https://cdn.jsdelivr.net; style-src 'self' 'unsafe-inline'; connect-src 'self'; img-src 'self' data:; font-src 'self' https://cdn.jsdelivr.net".parse().unwrap(),
    );

    // X-Frame-Options — prevent clickjacking
    headers.insert(
        header::X_FRAME_OPTIONS,
        "DENY".parse().unwrap(),
    );

    // X-Content-Type-Options — prevent MIME sniffing
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        "nosniff".parse().unwrap(),
    );

    // Referrer-Policy
    headers.insert(
        header::REFERRER_POLICY,
        "strict-origin-when-cross-origin".parse().unwrap(),
    );

    response
}
