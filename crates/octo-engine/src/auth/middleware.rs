// crates/octo-engine/src/auth/middleware.rs

use crate::auth::{AuthConfig, AuthMode, Permission};
use axum::{
    body::Body,
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};

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
            let key = req
                .headers()
                .get("X-API-Key")
                .and_then(|v| v.to_str().ok());

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
            // 完整认证（octo-platform 实现）
            Err(StatusCode::NOT_IMPLEMENTED)
        }
    }
}

/// 从请求中提取用户上下文
pub fn get_user_context<B>(req: &Request<B>) -> Option<UserContext> {
    req.extensions().get::<UserContext>().cloned()
}
