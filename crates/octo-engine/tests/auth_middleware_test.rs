// crates/octo-engine/tests/auth_middleware_test.rs

use axum::{body::Body, extract::Request};
use octo_engine::auth::*;

// Test get_user_context function directly
#[test]
fn test_get_user_context_none() {
    let req = Request::builder().uri("/").body(Body::empty()).unwrap();
    let ctx = get_user_context(&req);
    assert!(ctx.is_none());
}

#[test]
fn test_get_user_context_with_context() {
    let mut req = Request::builder().uri("/").body(Body::empty()).unwrap();

    req.extensions_mut().insert(UserContext {
        user_id: Some("user-001".to_string()),
        permissions: vec![Permission::Read, Permission::Write],
    });

    let ctx = get_user_context(&req).unwrap();
    assert_eq!(ctx.user_id, Some("user-001".to_string()));
    assert!(ctx.permissions.contains(&Permission::Read));
    assert!(ctx.permissions.contains(&Permission::Write));
}

// Test that AuthConfig::validate_key works correctly
#[test]
fn test_auth_config_none_mode_allows_any_key() {
    let mut config = AuthConfig::new();
    config.mode = AuthMode::None;

    // In None mode, any key should be valid (backward compatibility)
    assert!(config.validate_key("any-key"));
    assert!(config.validate_key(""));
}

#[test]
fn test_auth_config_api_key_mode_requires_valid_key() {
    let mut config = AuthConfig::new();
    config.mode = AuthMode::ApiKey;

    // No keys added, should reject
    assert!(!config.validate_key("any-key"));

    // Add a valid key
    config.add_api_key("valid-key", None, vec![]);
    assert!(config.validate_key("valid-key"));
    assert!(!config.validate_key("invalid-key"));
}

#[test]
fn test_auth_config_requires_user_id() {
    let mut config = AuthConfig::new();
    config.require_user_id = true;

    // With require_user_id, API key must have user_id
    config.mode = AuthMode::ApiKey;
    config.add_api_key("key-without-user", None, vec![]);

    // This key has no user_id but config requires it
    // The validation logic doesn't check require_user_id in validate_key
    // That would be done at the middleware level
    assert!(config.validate_key("key-without-user"));
}
