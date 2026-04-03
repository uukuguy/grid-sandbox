//! Dashboard authentication middleware
//!
//! Extracts API keys from the Authorization header and verifies them
//! against the ApiKeyStorage. Injects UserContext into request extensions.

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use grid_engine::auth::{Permission, Role, UserContext};
use std::path::PathBuf;
use std::sync::Arc;

/// Shared state for auth middleware
#[derive(Clone)]
pub struct DashboardAuthState {
    /// Path to the API key database
    db_path: PathBuf,
    /// Whether auth is required
    pub require_auth: bool,
}

impl DashboardAuthState {
    pub fn new(db_path: PathBuf, require_auth: bool) -> Self {
        Self {
            db_path,
            require_auth,
        }
    }
}

/// Auth middleware: extracts Bearer token, verifies against ApiKeyStorage
pub async fn dashboard_auth_middleware(
    State(auth_state): State<Arc<DashboardAuthState>>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if !auth_state.require_auth {
        // Auth not required -- inject anonymous context and pass through
        req.extensions_mut().insert(UserContext::anonymous());
        return Ok(next.run(req).await);
    }

    let auth_header = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    match auth_header {
        Some(ref header) if header.starts_with("Bearer ") => {
            let api_key = &header[7..];
            let db_path = auth_state.db_path.clone();
            let key_string = api_key.to_string();

            // ApiKeyStorage::verify is sync (rusqlite), so use spawn_blocking
            let verify_result = tokio::task::spawn_blocking(move || {
                let storage = grid_engine::auth::ApiKeyStorage::new(&db_path)
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                storage
                    .verify(&key_string)
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
            })
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            match verify_result? {
                Some((user_id, role)) => {
                    let permissions = match role {
                        Role::Viewer => vec![Permission::Read],
                        Role::User => vec![Permission::Read, Permission::Write],
                        Role::Admin | Role::Owner => {
                            vec![Permission::Read, Permission::Write, Permission::Admin]
                        }
                    };
                    let ctx = UserContext::new(Some(user_id), permissions, Some(role));
                    req.extensions_mut().insert(ctx);
                    Ok(next.run(req).await)
                }
                None => Err(StatusCode::UNAUTHORIZED),
            }
        }
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

/// Require a minimum role for an endpoint
pub fn require_role(ctx: &UserContext, min_role: Role) -> Result<(), StatusCode> {
    match ctx.role {
        Some(role) if role.has_at_least(&min_role) => Ok(()),
        _ => Err(StatusCode::FORBIDDEN),
    }
}
