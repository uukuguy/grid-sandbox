// DockerAdapter integration tests
//
// These tests require Docker to be running on the system.
// Use the `sandbox-docker` feature to run these tests.

use octo_engine::sandbox::{RuntimeAdapter, SandboxConfig, SandboxType};

#[tokio::test]
#[cfg(feature = "sandbox-docker")]
async fn test_docker_adapter_create() {
    use octo_engine::sandbox::DockerAdapter;

    // Skip if Docker is not available
    let adapter = DockerAdapter::new("alpine:latest");
    if !adapter.is_available() {
        eprintln!("Skipping test: Docker not available");
        return;
    }

    let config = SandboxConfig::new(SandboxType::Docker);

    let id = adapter.create(&config).await.unwrap();
    assert_eq!(adapter.sandbox_type(), SandboxType::Docker);
    assert!(!id.to_string().is_empty());

    // Cleanup
    adapter.destroy(&id).await.unwrap();
}

#[tokio::test]
#[cfg(feature = "sandbox-docker")]
async fn test_docker_adapter_create_and_execute() {
    use octo_engine::sandbox::DockerAdapter;

    let adapter = DockerAdapter::new("alpine:latest");
    if !adapter.is_available() {
        eprintln!("Skipping test: Docker not available");
        return;
    }

    let config = SandboxConfig::new(SandboxType::Docker);

    // Create sandbox
    let id = adapter.create(&config).await.unwrap();

    // Execute command
    let result = adapter
        .execute(&id, "echo 'hello from docker'", "bash")
        .await
        .unwrap();

    assert!(result.stdout.contains("hello from docker"));
    assert_eq!(result.exit_code, 0);
    assert!(result.success);
    assert!(result.execution_time_ms > 0);

    // Destroy sandbox
    adapter.destroy(&id).await.unwrap();
}

#[tokio::test]
#[cfg(feature = "sandbox-docker")]
async fn test_docker_adapter_execute_stderr() {
    use octo_engine::sandbox::DockerAdapter;

    let adapter = DockerAdapter::new("alpine:latest");
    if !adapter.is_available() {
        eprintln!("Skipping test: Docker not available");
        return;
    }

    let config = SandboxConfig::new(SandboxType::Docker);

    let id = adapter.create(&config).await.unwrap();

    // Execute command that writes to stderr
    let result = adapter
        .execute(&id, "echo 'error message' >&2", "bash")
        .await
        .unwrap();

    assert!(result.stderr.contains("error message"));
    assert_eq!(result.exit_code, 0);

    adapter.destroy(&id).await.unwrap();
}

#[tokio::test]
#[cfg(feature = "sandbox-docker")]
async fn test_docker_adapter_failed_command() {
    use octo_engine::sandbox::DockerAdapter;

    let adapter = DockerAdapter::new("alpine:latest");
    if !adapter.is_available() {
        eprintln!("Skipping test: Docker not available");
        return;
    }

    let config = SandboxConfig::new(SandboxType::Docker);

    let id = adapter.create(&config).await.unwrap();

    // Execute command that fails
    let result = adapter
        .execute(&id, "exit 1", "bash")
        .await
        .unwrap();

    assert_eq!(result.exit_code, 1);
    assert!(!result.success);

    adapter.destroy(&id).await.unwrap();
}

#[tokio::test]
#[cfg(feature = "sandbox-docker")]
async fn test_docker_adapter_not_found() {
    use octo_engine::sandbox::{DockerAdapter, SandboxId};

    let adapter = DockerAdapter::new("alpine:latest");
    if !adapter.is_available() {
        eprintln!("Skipping test: Docker not available");
        return;
    }

    let fake_id = SandboxId::new("non-existent-id");

    let result = adapter.execute(&fake_id, "echo test", "bash").await;
    assert!(result.is_err());
}

#[tokio::test]
#[cfg(feature = "sandbox-docker")]
async fn test_docker_adapter_destroy_not_found() {
    use octo_engine::sandbox::{DockerAdapter, SandboxId};

    let adapter = DockerAdapter::new("alpine:latest");
    if !adapter.is_available() {
        eprintln!("Skipping test: Docker not available");
        return;
    }

    let fake_id = SandboxId::new("non-existent-id");

    // Destroying a non-existent sandbox should not fail
    let result = adapter.destroy(&fake_id).await;
    assert!(result.is_ok());
}

#[tokio::test]
#[cfg(feature = "sandbox-docker")]
async fn test_docker_adapter_with_env_vars() {
    use octo_engine::sandbox::DockerAdapter;

    let adapter = DockerAdapter::new("alpine:latest");
    if !adapter.is_available() {
        eprintln!("Skipping test: Docker not available");
        return;
    }

    let mut config = SandboxConfig::new(SandboxType::Docker);
    config.env.insert("TEST_VAR".to_string(), "test_value".to_string());

    let id = adapter.create(&config).await.unwrap();

    // Execute command that uses the environment variable
    let result = adapter
        .execute(&id, "echo $TEST_VAR", "bash")
        .await
        .unwrap();

    assert!(result.stdout.trim().contains("test_value"));

    adapter.destroy(&id).await.unwrap();
}

#[tokio::test]
#[cfg(not(feature = "sandbox-docker"))]
async fn test_docker_adapter_without_feature() {
    use octo_engine::sandbox::{DockerAdapter, SandboxId, SandboxType};

    // Without the feature, DockerAdapter should return UnsupportedType errors
    let adapter = DockerAdapter::new("alpine:latest");

    let config = SandboxConfig::new(SandboxType::Docker);

    // Create should fail
    let result = adapter.create(&config).await;
    assert!(result.is_err());

    // The error should be UnsupportedType
    if let Err(e) = result {
        let error_str = e.to_string();
        assert!(
            error_str.contains("not enabled") || error_str.contains("Unsupported"),
            "Expected unsupported error, got: {}",
            error_str
        );
    }
}

#[tokio::test]
#[cfg(feature = "sandbox-docker")]
async fn test_docker_adapter_is_ready() {
    use octo_engine::sandbox::DockerAdapter;

    let adapter = DockerAdapter::new("alpine:latest");

    // Check if the adapter reports ready (depends on Docker daemon availability)
    let ready = adapter.is_ready().await;
    if !ready {
        eprintln!("Docker daemon not available - adapter reports not ready");
    }
}

#[tokio::test]
async fn test_docker_adapter_default_image() {
    use octo_engine::sandbox::DockerAdapter;

    // Test with default image
    let adapter = DockerAdapter::default();
    assert_eq!(adapter.image(), "alpine:latest");
}
