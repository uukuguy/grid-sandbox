//! Dashboard command — launches an embedded web dashboard
//!
//! Serves a lightweight single-page app directly from the CLI binary.
//! All assets are compiled in via `include_str!()`.

use anyhow::Result;
use axum::{routing::get, Json, Router};
use std::net::SocketAddr;

// ── Embedded Assets ──────────────────────────────────────────────

const INDEX_HTML: &str = include_str!("../dashboard/assets/index.html");
const APP_JS: &str = include_str!("../dashboard/assets/app.js");
const STYLE_CSS: &str = include_str!("../dashboard/assets/style.css");

// ── Dashboard Options ────────────────────────────────────────────

/// Configuration for the dashboard server.
pub struct DashboardOptions {
    /// Port to listen on (default: 8080)
    pub port: u16,
    /// Host to bind to (default: 127.0.0.1)
    pub host: String,
    /// Open browser on start
    pub open: bool,
}

impl Default for DashboardOptions {
    fn default() -> Self {
        Self {
            port: 8080,
            host: "127.0.0.1".to_string(),
            open: false,
        }
    }
}

// ── Route Handlers ──────────────────────────────────────────────

async fn index_handler() -> axum::response::Html<&'static str> {
    axum::response::Html(INDEX_HTML)
}

async fn app_js_handler() -> (
    [(axum::http::header::HeaderName, &'static str); 1],
    &'static str,
) {
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "application/javascript",
        )],
        APP_JS,
    )
}

async fn style_css_handler() -> (
    [(axum::http::header::HeaderName, &'static str); 1],
    &'static str,
) {
    (
        [(axum::http::header::CONTENT_TYPE, "text/css")],
        STYLE_CSS,
    )
}

/// API: health check
async fn api_health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

// ── Router ──────────────────────────────────────────────────────

fn build_router() -> Router {
    Router::new()
        // Static assets
        .route("/", get(index_handler))
        .route("/app.js", get(app_js_handler))
        .route("/style.css", get(style_css_handler))
        // API endpoints
        .route("/api/health", get(api_health))
}

/// Run the dashboard server.
pub async fn run_dashboard(opts: &DashboardOptions) -> Result<()> {
    let addr: SocketAddr = format!("{}:{}", opts.host, opts.port).parse()?;
    let router = build_router();

    eprintln!("Dashboard running at http://{}", addr);
    eprintln!("Press Ctrl+C to stop.\n");

    if opts.open {
        // Best-effort: try to open the browser
        let url = format!("http://{}", addr);
        let _ = open_browser(&url);
    }

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;

    Ok(())
}

/// Attempt to open a URL in the default browser.
fn open_browser(url: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(url).spawn()?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open").arg(url).spawn()?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/c", "start", url])
            .spawn()?;
    }
    Ok(())
}

// ── Tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_options_default() {
        let opts = DashboardOptions::default();
        assert_eq!(opts.port, 8080);
        assert_eq!(opts.host, "127.0.0.1");
        assert!(!opts.open);
    }

    #[test]
    fn test_embedded_assets_not_empty() {
        assert!(!INDEX_HTML.is_empty());
        assert!(!APP_JS.is_empty());
        assert!(!STYLE_CSS.is_empty());
    }

    #[test]
    fn test_index_html_contains_alpine_directives() {
        assert!(INDEX_HTML.contains("x-data"));
        assert!(INDEX_HTML.contains("x-init"));
        assert!(INDEX_HTML.contains("x-show"));
    }

    #[test]
    fn test_index_html_contains_all_tabs() {
        assert!(INDEX_HTML.contains("Chat"));
        assert!(INDEX_HTML.contains("Sessions"));
        assert!(INDEX_HTML.contains("Memory"));
        assert!(INDEX_HTML.contains("MCP"));
    }

    #[test]
    fn test_style_css_has_root_variables() {
        assert!(STYLE_CSS.contains(":root"));
        assert!(STYLE_CSS.contains("--accent"));
        assert!(STYLE_CSS.contains("--bg"));
    }

    #[test]
    fn test_app_js_has_app_function() {
        assert!(APP_JS.contains("function app()"));
        assert!(APP_JS.contains("checkHealth"));
        assert!(APP_JS.contains("sendMessage"));
    }

    #[test]
    fn test_router_builds() {
        let _ = build_router();
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        let app = build_router();
        let request = Request::builder()
            .uri("/api/health")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), 200);
    }

    #[tokio::test]
    async fn test_index_endpoint() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        let app = build_router();
        let request = Request::builder()
            .uri("/")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), 200);
    }

    #[tokio::test]
    async fn test_js_endpoint() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        let app = build_router();
        let request = Request::builder()
            .uri("/app.js")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), 200);
        let content_type = response
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(content_type, "application/javascript");
    }

    #[tokio::test]
    async fn test_css_endpoint() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        let app = build_router();
        let request = Request::builder()
            .uri("/style.css")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), 200);
        let content_type = response
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(content_type, "text/css");
    }
}
