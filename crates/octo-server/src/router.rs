use std::net::SocketAddr;

use std::sync::Arc;

use axum::{
    extract::Request,
    routing::get,
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::api;
use crate::middleware::RateLimiter;
use crate::state::AppState;
use crate::ws::ws_handler;

async fn health() -> &'static str {
    "ok"
}

/// Rate limiting middleware
async fn rate_limit_middleware(
    rate_limiter: axum::extract::State<RateLimiter>,
    req: Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    use tracing::debug;

    // Extract client IP: try ConnectInfo extension first, then X-Forwarded-For header
    let client_ip = req
        .extensions()
        .get::<axum::extract::connect_info::ConnectInfo<std::net::SocketAddr>>()
        .map(|connect_info| connect_info.0.ip().to_string())
        .or_else(|| {
            req.headers()
                .get("x-forwarded-for")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.split(',').next())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "127.0.0.1".to_string());

    debug!(client_ip = %client_ip, "Rate limit check");

    if !rate_limiter.check(&client_ip).await {
        debug!("Rate limit exceeded for {}", client_ip);
        return (
            axum::http::StatusCode::TOO_MANY_REQUESTS,
            [("retry-after", "60")],
            "Rate limit exceeded. Please try again later.",
        )
            .into_response();
    }

    next.run(req).await
}

pub fn build_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Rate limiter: 100 requests per minute per IP
    let rate_limiter = RateLimiter::new(100, 60);

    Router::new()
        .route("/api/health", get(health))
        .route("/ws", get(ws_handler))
        .nest("/api", api::routes())
        .with_state(state)
        .with_state(rate_limiter.clone())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .layer(axum::middleware::from_fn_with_state(
            rate_limiter,
            rate_limit_middleware,
        ))
}
