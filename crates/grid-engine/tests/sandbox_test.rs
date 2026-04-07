// Basic sandbox module test

#[test]
fn test_sandbox_module_exists() {
    // Basic test to verify module loads
    let _ = grid_engine::sandbox::SubprocessAdapter::new();
}

#[test]
fn test_sandbox_types() {
    use grid_engine::sandbox::SandboxType;

    assert_eq!(format!("{}", SandboxType::Wasm), "wasm");
    assert_eq!(format!("{}", SandboxType::Docker), "docker");
    assert_eq!(format!("{}", SandboxType::Subprocess), "subprocess");
}

#[test]
fn test_sandbox_config() {
    use grid_engine::sandbox::SandboxConfig;

    let config = SandboxConfig::new(grid_engine::sandbox::SandboxType::Subprocess)
        .with_working_dir("/tmp".into())
        .with_env("KEY", "value")
        .with_memory_limit(1024 * 1024)
        .with_time_limit(60);

    assert_eq!(
        config.sandbox_type,
        grid_engine::sandbox::SandboxType::Subprocess
    );
    assert!(config.working_dir.is_some());
    assert_eq!(config.working_dir.unwrap().to_string_lossy(), "/tmp");
    assert!(config.env.contains_key("KEY"));
    assert_eq!(config.memory_limit, Some(1024 * 1024));
    assert_eq!(config.time_limit, Some(60));
}

#[test]
fn test_sandbox_id() {
    use grid_engine::sandbox::SandboxId;

    let id = SandboxId::new("test-123");
    assert_eq!(id.to_string(), "test-123");
}

#[test]
fn test_adapters_exist() {
    let _wasm = grid_engine::sandbox::WasmAdapter::new();
    let _docker = grid_engine::sandbox::DockerAdapter::new("alpine:latest");
    let _subprocess = grid_engine::sandbox::SubprocessAdapter::new();
}
