//! Tests for WASM sandbox adapter
//!
//! These tests require the `sandbox-wasm` feature to be enabled.

#![cfg(feature = "sandbox-wasm")]

use octo_engine::sandbox::{RuntimeAdapter, SandboxConfig, SandboxId, SandboxType, WasmAdapter};

/// Test creating a WASM adapter
#[tokio::test]
async fn test_wasm_adapter_create() {
    let adapter = WasmAdapter::new();
    let config = SandboxConfig::new(SandboxType::Wasm);

    let id = adapter.create(&config).await.unwrap();
    assert!(!id.to_string().is_empty());

    adapter.destroy(&id).await.unwrap();
}

/// Test creating multiple sandboxes
#[tokio::test]
async fn test_wasm_adapter_multiple() {
    let adapter = WasmAdapter::new();
    let config = SandboxConfig::new(SandboxType::Wasm);

    let id1 = adapter.create(&config).await.unwrap();
    let id2 = adapter.create(&config).await.unwrap();

    // IDs should be different
    assert_ne!(id1, id2);

    adapter.destroy(&id1).await.unwrap();
    adapter.destroy(&id2).await.unwrap();
}

/// Test destroy non-existent sandbox
#[tokio::test]
async fn test_wasm_adapter_destroy_not_found() {
    let adapter = WasmAdapter::new();
    let fake_id = SandboxId::new("non-existent-id");

    // Destroying non-existent sandbox should succeed (idempotent)
    let result = adapter.destroy(&fake_id).await;
    assert!(result.is_ok());
}

/// Test sandbox type
#[tokio::test]
async fn test_wasm_adapter_type() {
    let adapter = WasmAdapter::new();
    assert_eq!(adapter.sandbox_type(), SandboxType::Wasm);
}

/// Test is_available
#[tokio::test]
async fn test_wasm_adapter_available() {
    let adapter = WasmAdapter::new();
    assert!(adapter.is_available());
}

/// Test execute without creating sandbox first
#[tokio::test]
async fn test_wasm_adapter_execute_no_sandbox() {
    let adapter = WasmAdapter::new();
    let fake_id = SandboxId::new("non-existent-id");

    let result = adapter.execute(&fake_id, "test", "wasm").await;
    assert!(result.is_err());
}
