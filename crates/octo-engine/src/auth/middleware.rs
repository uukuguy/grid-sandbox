// crates/octo-engine/src/auth/middleware.rs

use crate::auth::{AuthConfig, AuthMode, Permission};
use axum::{body::Body, extract::Request, http::StatusCode, middleware::Next, response::Response};

/// 用户上下文
#[derive(Debug, Clone)]
pub struct UserContext {
    pub user_id: Option<String>,
    pub permissions: Vec<Permission>,
}

/// 认证中间件
pub async fn auth_middleware(
    req: Request<Body>,
    next: Next,
    config: &AuthConfig,
) -> Result<Response, StatusCode> {
    match config.mode {
        AuthMode::None => {
            // 无认证模式，直接放行，注入匿名用户
            let mut req = req;
            req.extensions_mut().insert(UserContext {
                user_id: None,
                permissions: vec![],
            });
            Ok(next.run(req).await)
        }
        AuthMode::ApiKey => {
            // 验证 API Key
            let key = req.headers().get("X-API-Key").and_then(|v| v.to_str().ok());

            match key {
                Some(k) if config.validate_key(k) => {
                    let user_id = config.get_user_id(k);
                    let permissions = config.get_permissions(k);

                    let mut req = req;
                    req.extensions_mut().insert(UserContext {
                        user_id,
                        permissions,
                    });
                    Ok(next.run(req).await)
                }
                _ => Err(StatusCode::UNAUTHORIZED),
            }
        }
        AuthMode::Full => {
            // 完整认证：JWT Bearer token
            let auth_header = req.headers().get("authorization");
            let token = auth_header
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "));

            match token {
                Some(t) => {
                    if let Some(claims) = config.validate_jwt(t) {
                        // Convert role string to permissions
                        let permissions = match claims.role.as_str() {
                            "admin" => vec![Permission::Admin],
                            "member" => vec![Permission::Read, Permission::Write],
                            "viewer" => vec![Permission::Read],
                            _ => vec![],
                        };

                        let mut req = req;
                        req.extensions_mut().insert(UserContext {
                            user_id: Some(claims.sub),
                            permissions,
                        });
                        Ok(next.run(req).await)
                    } else {
                        Err(StatusCode::UNAUTHORIZED)
                    }
                }
                _ => Err(StatusCode::UNAUTHORIZED),
            }
        }
    }
}

/// 从请求中提取用户上下文
pub fn get_user_context<B>(req: &Request<B>) -> Option<UserContext> {
    req.extensions().get::<UserContext>().cloned()
}
