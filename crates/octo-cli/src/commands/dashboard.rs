//! Dashboard command — launches an embedded web dashboard
//!
//! Serves a lightweight single-page app directly from the CLI binary.
//! All assets are compiled in via `include_str!()`.

use anyhow::Result;
use axum::{extract::Path, extract::Query, routing::get, Json, Router};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use super::dashboard_auth::{DashboardAuthState, dashboard_auth_middleware};
use super::dashboard_security::{build_cors_layer, security_headers_middleware};

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
    /// Enable TLS/HTTPS
    pub tls_enabled: bool,
    /// Path to TLS certificate (PEM)
    pub cert_path: Option<String>,
    /// Path to TLS private key (PEM)
    pub key_path: Option<String>,
    /// Require API key authentication
    pub require_auth: bool,
    /// Allowed CORS origins (empty = same-origin only)
    pub allowed_origins: Vec<String>,
}

impl Default for DashboardOptions {
    fn default() -> Self {
        Self {
            port: 8080,
            host: "127.0.0.1".to_string(),
            open: false,
            tls_enabled: false,
            cert_path: None,
            key_path: None,
            require_auth: false,
            allowed_origins: Vec::new(),
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

/// D2-3: Chat message endpoint (stub — echoes back in preview mode)
async fn api_chat_send(Json(body): Json<serde_json::Value>) -> Json<serde_json::Value> {
    let user_msg = body["message"].as_str().unwrap_or("");
    Json(serde_json::json!({
        "response": format!(
            "Dashboard preview: received '{}'. Use CLI for full agent interaction.",
            user_msg
        ),
        "model": "preview",
    }))
}

/// D2-4: List sessions (stub)
async fn api_sessions_list() -> Json<serde_json::Value> {
    Json(serde_json::json!([
        {"id": "session-001", "created_at": "2026-03-10T10:00:00Z", "messages": 12, "status": "active"},
        {"id": "session-002", "created_at": "2026-03-09T15:30:00Z", "messages": 45, "status": "closed"},
    ]))
}

/// D2-4: Session detail (stub)
async fn api_session_detail(Path(id): Path<String>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "id": id,
        "created_at": "2026-03-10T10:00:00Z",
        "messages": 12,
        "status": "active",
        "model": "claude-sonnet-4-6",
    }))
}

/// D2-5: List memories (stub)
async fn api_memories_list() -> Json<serde_json::Value> {
    Json(serde_json::json!([
        {"id": "mem-001", "category": "project_structure", "content": "Main entry: src/main.rs", "score": 0.95},
        {"id": "mem-002", "category": "user_preference", "content": "Always use cargo test --test-threads=1", "score": 0.88},
        {"id": "mem-003", "category": "technical_decision", "content": "Use Axum for HTTP server", "score": 0.82},
    ]))
}

/// D2-5: Search memories (stub)
async fn api_memories_search(
    Query(params): Query<HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let query = params.get("q").cloned().unwrap_or_default();
    Json(serde_json::json!({
        "query": query,
        "results": [
            {"id": "mem-001", "category": "project_structure", "content": "Main entry: src/main.rs", "score": 0.95},
        ],
    }))
}

/// D2-6: MCP servers (stub)
async fn api_mcp_servers() -> Json<serde_json::Value> {
    Json(serde_json::json!([
        {"name": "filesystem", "running": true, "tools": 5, "transport": "stdio"},
        {"name": "github", "running": false, "tools": 12, "transport": "sse"},
    ]))
}

/// D2-7: Available themes list
async fn api_themes_list() -> Json<serde_json::Value> {
    Json(serde_json::json!([
        "cyan", "sgcc", "blue", "indigo", "violet", "emerald",
        "amber", "coral", "rose", "teal", "sunset", "slate"
    ]))
}

// ── Router ──────────────────────────────────────────────────────

pub fn build_router(require_auth: bool, allowed_origins: &[String], db_path: Option<PathBuf>) -> Router {
    // Public routes — no auth required
    let public_routes = Router::new()
        .route("/", get(index_handler))
        .route("/app.js", get(app_js_handler))
        .route("/style.css", get(style_css_handler))
        .route("/api/health", get(api_health))
        .route("/api/themes", get(api_themes_list));

    // Protected routes — auth required when enabled
    let protected_routes = Router::new()
        // Viewer+ endpoints
        .route("/api/sessions", get(api_sessions_list))
        .route("/api/sessions/{id}", get(api_session_detail))
        .route("/api/memories", get(api_memories_list))
        .route("/api/memories/search", get(api_memories_search))
        // User+ endpoints
        .route("/api/chat", axum::routing::post(api_chat_send))
        // Admin+ endpoints
        .route("/api/mcp/servers", get(api_mcp_servers));

    // Apply auth middleware to protected routes
    let auth_state = Arc::new(DashboardAuthState::new(
        db_path.unwrap_or_else(|| PathBuf::from("./data/dashboard_keys.db")),
        require_auth,
    ));

    let protected_routes = protected_routes.layer(axum::middleware::from_fn_with_state(
        auth_state,
        dashboard_auth_middleware,
    ));

    // Merge and apply global layers
    public_routes
        .merge(protected_routes)
        .layer(axum::middleware::from_fn(security_headers_middleware))
        .layer(build_cors_layer(allowed_origins))
}

/// Run the dashboard server.
pub async fn run_dashboard(opts: &DashboardOptions) -> Result<()> {
    if opts.host != "127.0.0.1" && opts.host != "localhost" && !opts.require_auth {
        eprintln!("⚠️  WARNING: Binding to {} without --require-auth. Remote access is unprotected!", opts.host);
        eprintln!("   Consider adding --require-auth for security.\n");
    }

    let addr: SocketAddr = format!("{}:{}", opts.host, opts.port).parse()?;
    let router = build_router(opts.require_auth, &opts.allowed_origins, None);

    let scheme = if opts.tls_enabled { "https" } else { "http" };
    eprintln!("Dashboard running at {}://{}", scheme, addr);
    eprintln!("Press Ctrl+C to stop.\n");

    if opts.open {
        let url = format!("{}://{}", scheme, addr);
        let _ = open_browser(&url);
    }

    #[cfg(feature = "dashboard-tls")]
    if opts.tls_enabled {
        use axum_server::tls_rustls::RustlsConfig;

        let cert_path = opts
            .cert_path
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("--cert-path required when TLS is enabled"))?;
        let key_path = opts
            .key_path
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("--key-path required when TLS is enabled"))?;

        let tls_config = RustlsConfig::from_pem_file(cert_path, key_path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to load TLS certificates: {}", e))?;

        axum_server::bind_rustls(addr, tls_config)
            .serve(router.into_make_service())
            .await?;

        return Ok(());
    }

    #[cfg(not(feature = "dashboard-tls"))]
    if opts.tls_enabled {
        anyhow::bail!(
            "TLS support requires the 'dashboard-tls' feature. \
             Rebuild with: cargo build --features dashboard-tls"
        );
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
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    /// Helper: GET a URI, assert 200, parse JSON body.
    async fn get_json(uri: &str) -> serde_json::Value {
        let app = build_router(false, &[], None);
        let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), 200);
        let body = res.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&body).unwrap()
    }

    /// Helper: GET a URI, return status code.
    async fn get_status(uri: &str) -> u16 {
        let app = build_router(false, &[], None);
        let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
        let res = app.oneshot(req).await.unwrap();
        res.status().as_u16()
    }

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
        for tab in ["Chat", "Sessions", "Memory", "MCP"] {
            assert!(INDEX_HTML.contains(tab), "Missing tab: {tab}");
        }
    }

    #[test]
    fn test_style_css_has_root_variables() {
        for var in [":root", "--accent", "--bg"] {
            assert!(STYLE_CSS.contains(var), "Missing CSS var: {var}");
        }
    }

    #[test]
    fn test_app_js_has_app_function() {
        for token in ["function app()", "checkHealth", "sendMessage"] {
            assert!(APP_JS.contains(token), "Missing JS token: {token}");
        }
    }

    #[test]
    fn test_router_builds() {
        let _ = build_router(false, &[], None);
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        assert_eq!(get_status("/api/health").await, 200);
    }

    #[tokio::test]
    async fn test_index_endpoint() {
        assert_eq!(get_status("/").await, 200);
    }

    #[tokio::test]
    async fn test_js_endpoint() {
        let app = build_router(false, &[], None);
        let req = Request::builder().uri("/app.js").body(Body::empty()).unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), 200);
        let ct = res.headers().get("content-type").unwrap().to_str().unwrap();
        assert_eq!(ct, "application/javascript");
    }

    #[tokio::test]
    async fn test_css_endpoint() {
        let app = build_router(false, &[], None);
        let req = Request::builder().uri("/style.css").body(Body::empty()).unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), 200);
        let ct = res.headers().get("content-type").unwrap().to_str().unwrap();
        assert_eq!(ct, "text/css");
    }

    #[tokio::test]
    async fn test_chat_endpoint() {
        let app = build_router(false, &[], None);
        let req = Request::builder()
            .method("POST")
            .uri("/api/chat")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"message":"hello"}"#))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), 200);
        let body = res.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["response"].as_str().unwrap().contains("hello"));
        assert_eq!(json["model"], "preview");
    }

    #[tokio::test]
    async fn test_sessions_list_endpoint() {
        let json = get_json("/api/sessions").await;
        assert!(json.as_array().unwrap().len() >= 2);
    }

    #[tokio::test]
    async fn test_session_detail_endpoint() {
        let json = get_json("/api/sessions/test-123").await;
        assert_eq!(json["id"], "test-123");
        assert_eq!(json["status"], "active");
    }

    #[tokio::test]
    async fn test_memories_list_endpoint() {
        let json = get_json("/api/memories").await;
        assert!(json.as_array().unwrap().len() >= 3);
    }

    #[tokio::test]
    async fn test_memories_search_endpoint() {
        let json = get_json("/api/memories/search?q=main").await;
        assert_eq!(json["query"], "main");
        assert!(json["results"].as_array().unwrap().len() >= 1);
    }

    #[tokio::test]
    async fn test_mcp_servers_endpoint() {
        let json = get_json("/api/mcp/servers").await;
        let servers = json.as_array().unwrap();
        assert_eq!(servers.len(), 2);
        assert_eq!(servers[0]["name"], "filesystem");
    }

    #[tokio::test]
    async fn test_themes_endpoint() {
        let json = get_json("/api/themes").await;
        let themes = json.as_array().unwrap();
        assert_eq!(themes.len(), 12);
        assert!(themes.contains(&serde_json::json!("cyan")));
        assert!(themes.contains(&serde_json::json!("slate")));
    }

    // ── D2-8 Integration Tests ─────────────────────────────────────

    #[tokio::test]
    async fn test_all_static_assets_serve() {
        let app = build_router(false, &[], None);
        let req = Request::builder().uri("/").body(Body::empty()).unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), 200);
        let ct = res.headers().get("content-type").unwrap().to_str().unwrap();
        assert!(ct.contains("text/html"), "index content-type: {ct}");

        assert_eq!(get_status("/app.js").await, 200);
        assert_eq!(get_status("/style.css").await, 200);
    }

    #[tokio::test]
    async fn test_health_json_structure() {
        let json = get_json("/api/health").await;
        assert_eq!(json["status"], "ok");
        assert!(json["version"].as_str().is_some(), "missing version field");
        assert!(!json["version"].as_str().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_chat_echoes_message() {
        let app = build_router(false, &[], None);
        let req = Request::builder()
            .method("POST")
            .uri("/api/chat")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"message":"test input"}"#))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), 200);
        let body = res.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["response"].as_str().unwrap().contains("test input"));
    }

    #[tokio::test]
    async fn test_sessions_returns_array_with_ids() {
        let json = get_json("/api/sessions").await;
        let arr = json.as_array().expect("sessions should be an array");
        assert!(!arr.is_empty());
        for entry in arr {
            assert!(entry["id"].as_str().is_some(), "session entry missing id");
        }
    }

    #[tokio::test]
    async fn test_session_detail_uses_id() {
        let json = get_json("/api/sessions/my-session-123").await;
        assert_eq!(json["id"], "my-session-123");
    }

    #[tokio::test]
    async fn test_memories_have_categories() {
        let json = get_json("/api/memories").await;
        let arr = json.as_array().expect("memories should be an array");
        for entry in arr {
            assert!(entry["category"].as_str().is_some(), "missing category");
            assert!(entry["content"].as_str().is_some(), "missing content");
        }
    }

    #[tokio::test]
    async fn test_memory_search_accepts_query() {
        let json = get_json("/api/memories/search?q=test").await;
        assert_eq!(json["query"], "test");
        assert!(json["results"].as_array().is_some());
    }

    #[tokio::test]
    async fn test_mcp_servers_have_fields() {
        let json = get_json("/api/mcp/servers").await;
        let servers = json.as_array().unwrap();
        for s in servers {
            assert!(s["name"].as_str().is_some(), "missing name");
            assert!(s["running"].is_boolean(), "missing running");
            assert!(s["tools"].is_number(), "missing tools");
        }
    }

    #[tokio::test]
    async fn test_themes_returns_12() {
        let json = get_json("/api/themes").await;
        let themes = json.as_array().unwrap();
        assert_eq!(themes.len(), 12);
        assert!(themes.contains(&serde_json::json!("cyan")));
        assert!(themes.contains(&serde_json::json!("slate")));
    }

    #[test]
    fn test_html_alpine_completeness() {
        for directive in ["x-data", "x-show", "x-for", "@click", "x-model"] {
            assert!(INDEX_HTML.contains(directive), "Missing directive: {directive}");
        }
    }

    #[test]
    fn test_css_has_all_themes() {
        for theme in [
            "cyan", "sgcc", "blue", "indigo", "violet", "emerald",
            "amber", "coral", "rose", "teal", "sunset", "slate",
        ] {
            assert!(
                STYLE_CSS.contains(&format!("[data-theme=\"{}\"]", theme)),
                "Missing theme: {}",
                theme
            );
        }
    }

    #[tokio::test]
    async fn test_unknown_route_returns_404() {
        assert_eq!(get_status("/api/nonexistent").await, 404);
        assert_eq!(get_status("/no-such-page").await, 404);
    }

    // ── D7-8 Security & Auth Integration Tests ──────────────────────

    #[tokio::test]
    async fn test_security_headers_present() {
        let app = build_router(false, &[], None);
        let req = Request::builder().uri("/api/health").body(Body::empty()).unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), 200);

        let headers = res.headers();
        assert!(headers.contains_key("strict-transport-security"), "Missing HSTS header");
        assert!(headers.contains_key("content-security-policy"), "Missing CSP header");
        assert!(headers.contains_key("x-frame-options"), "Missing X-Frame-Options");
        assert!(headers.contains_key("x-content-type-options"), "Missing X-Content-Type-Options");
        assert!(headers.contains_key("referrer-policy"), "Missing Referrer-Policy");
    }

    #[tokio::test]
    async fn test_security_header_values() {
        let app = build_router(false, &[], None);
        let req = Request::builder().uri("/").body(Body::empty()).unwrap();
        let res = app.oneshot(req).await.unwrap();

        assert_eq!(res.headers()["x-frame-options"], "DENY");
        assert_eq!(res.headers()["x-content-type-options"], "nosniff");
        assert!(res.headers()["strict-transport-security"].to_str().unwrap().contains("max-age="));
        assert!(res.headers()["content-security-policy"].to_str().unwrap().contains("default-src"));
    }

    #[tokio::test]
    async fn test_no_auth_public_routes_accessible() {
        // With auth disabled, all routes should work without tokens
        let app = build_router(false, &[], None);

        // Public routes
        for uri in ["/", "/app.js", "/style.css", "/api/health", "/api/themes"] {
            let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
            let res = app.clone().oneshot(req).await.unwrap();
            assert_eq!(res.status(), 200, "Public route {} should return 200", uri);
        }
    }

    #[tokio::test]
    async fn test_no_auth_protected_routes_accessible() {
        // With auth disabled, protected routes should also work (anonymous pass-through)
        let app = build_router(false, &[], None);

        for uri in ["/api/sessions", "/api/memories", "/api/memories/search?q=test", "/api/mcp/servers"] {
            let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
            let res = app.clone().oneshot(req).await.unwrap();
            assert_eq!(res.status(), 200, "Route {} should return 200 when auth disabled", uri);
        }
    }

    #[tokio::test]
    async fn test_auth_enabled_public_routes_no_token() {
        // With auth enabled, public routes should still work without tokens
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("test_keys.db");

        let app = build_router(true, &[], Some(db_path));

        for uri in ["/", "/app.js", "/style.css", "/api/health", "/api/themes"] {
            let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
            let res = app.clone().oneshot(req).await.unwrap();
            assert_eq!(res.status(), 200, "Public route {} should return 200 even with auth enabled", uri);
        }
    }

    #[tokio::test]
    async fn test_auth_enabled_protected_routes_no_token() {
        // With auth enabled, protected routes should return 401 without token
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("test_keys.db");

        let app = build_router(true, &[], Some(db_path));

        for uri in ["/api/sessions", "/api/memories", "/api/mcp/servers"] {
            let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
            let res = app.clone().oneshot(req).await.unwrap();
            assert_eq!(res.status(), 401, "Protected route {} should return 401 without token", uri);
        }
    }

    #[tokio::test]
    async fn test_auth_enabled_invalid_token() {
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("test_keys.db");
        // Create the DB so ApiKeyStorage can init
        let _storage = octo_engine::auth::ApiKeyStorage::new(db_path.as_path()).unwrap();

        let app = build_router(true, &[], Some(db_path));

        let req = Request::builder()
            .uri("/api/sessions")
            .header("authorization", "Bearer invalid-key-12345")
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), 401, "Invalid token should return 401");
    }

    #[tokio::test]
    async fn test_auth_enabled_valid_token() {
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("test_keys.db");

        // Create storage and generate a valid API key
        let storage = octo_engine::auth::ApiKeyStorage::new(db_path.as_path()).unwrap();
        let (stored_key, raw_key) = octo_engine::auth::StoredApiKey::generate("test-user", octo_engine::auth::Role::Admin);
        storage.create(&stored_key).unwrap();

        let app = build_router(true, &[], Some(db_path));

        let req = Request::builder()
            .uri("/api/sessions")
            .header("authorization", format!("Bearer {}", raw_key))
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), 200, "Valid token should grant access");
    }

    #[tokio::test]
    async fn test_auth_bearer_prefix_required() {
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("test_keys.db");
        let _storage = octo_engine::auth::ApiKeyStorage::new(db_path.as_path()).unwrap();

        let app = build_router(true, &[], Some(db_path));

        // Token without "Bearer " prefix
        let req = Request::builder()
            .uri("/api/sessions")
            .header("authorization", "just-a-raw-key")
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), 401, "Token without Bearer prefix should be rejected");
    }

    #[tokio::test]
    async fn test_chat_endpoint_auth_required() {
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("test_keys.db");
        let _storage = octo_engine::auth::ApiKeyStorage::new(db_path.as_path()).unwrap();

        let app = build_router(true, &[], Some(db_path));

        // POST to chat without auth
        let req = Request::builder()
            .method("POST")
            .uri("/api/chat")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"message":"hello"}"#))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), 401, "Chat endpoint should require auth");
    }

    #[tokio::test]
    async fn test_chat_endpoint_with_valid_auth() {
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("test_keys.db");
        let storage = octo_engine::auth::ApiKeyStorage::new(db_path.as_path()).unwrap();
        let (stored_key, raw_key) = octo_engine::auth::StoredApiKey::generate("chat-user", octo_engine::auth::Role::User);
        storage.create(&stored_key).unwrap();

        let app = build_router(true, &[], Some(db_path));

        let req = Request::builder()
            .method("POST")
            .uri("/api/chat")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {}", raw_key))
            .body(Body::from(r#"{"message":"hello auth"}"#))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), 200, "Chat with valid auth should succeed");
        let body = res.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["response"].as_str().unwrap().contains("hello auth"));
    }

    #[test]
    fn test_dashboard_options_new_fields_default() {
        let opts = DashboardOptions::default();
        assert!(!opts.tls_enabled);
        assert!(opts.cert_path.is_none());
        assert!(opts.key_path.is_none());
        assert!(!opts.require_auth);
        assert!(opts.allowed_origins.is_empty());
    }

    #[tokio::test]
    async fn test_cors_headers_with_origins() {
        let origins = vec!["http://example.com".to_string()];
        let app = build_router(false, &origins, None);

        let req = Request::builder()
            .uri("/api/health")
            .header("origin", "http://example.com")
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), 200);
        // CORS headers should be present when origin matches
        assert!(
            res.headers().contains_key("access-control-allow-origin"),
            "CORS allow-origin header should be present for matching origin"
        );
    }

    #[tokio::test]
    async fn test_no_cors_without_origins() {
        let app = build_router(false, &[], None);

        let req = Request::builder()
            .uri("/api/health")
            .header("origin", "http://evil.com")
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), 200);
        // No CORS headers when no origins configured
        assert!(
            !res.headers().contains_key("access-control-allow-origin"),
            "CORS headers should not be present when no origins configured"
        );
    }
}
