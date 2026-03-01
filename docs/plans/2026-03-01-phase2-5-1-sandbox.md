# Phase 2.5.1 沙箱系统实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 替换当前不安全的 Subprocess 沙箱，实现 WASM + Docker 双沙箱架构

**Architecture:**
- 创建 RuntimeAdapter Trait 抽象沙箱接口
- 实现 WasmAdapter (wasmtime) 用于轻量计算任务
- 实现 DockerAdapter (bollard) 用于完整 Agent 环境
- 工具根据类型自动路由到对应沙箱

**Tech Stack:** wasmtime, bollard, tokio

**参考:** openfang `crates/openfang-runtime/src/sandbox.rs`, `docker_sandbox.rs`, `subprocess_sandbox.rs`

---

## 任务 1: 创建沙箱模块结构

**Files:**
- Create: `crates/octo-engine/src/sandbox/mod.rs`
- Create: `crates/octo-engine/src/sandbox/traits.rs`
- Test: `crates/octo-engine/tests/sandbox_test.rs`

**Step 1: 创建沙箱模块入口**

```rust
// crates/octo-engine/src/sandbox/mod.rs

pub mod traits;
pub mod wasm;
pub mod docker;
pub mod subprocess;

pub use traits::*;
pub use wasm::WasmAdapter;
pub use docker::DockerAdapter;
pub use subprocess::SubprocessAdapter;
```

**Step 2: 运行 cargo check 验证模块创建**

Run: `cargo check -p octo-engine`
Expected: SUCCESS (new files detected)

**Step 3: 提交**

```bash
git add crates/octo-engine/src/sandbox/
git commit -m "feat(sandbox): create sandbox module structure"
```

---

## 任务 2: RuntimeAdapter Trait 定义

**Files:**
- Modify: `crates/octo-engine/src/sandbox/traits.rs`
- Test: `crates/octo-engine/tests/sandbox_trait_test.rs`

**Step 1: 编写 Trait 定义**

```rust
// crates/octo-engine/src/sandbox/traits.rs

use async_trait::async_trait;
use std::path::PathBuf;
use std::time::Duration;
use std::collections::HashMap;

/// 沙箱类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxType {
    Wasm,
    Docker,
    Subprocess,
}

/// 沙箱配置
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    pub sandbox_type: SandboxType,
    pub max_memory_mb: usize,
    pub max_fuel: u64,
    pub max_duration_secs: u64,
    pub allowed_paths: Vec<PathBuf>,
    pub env_vars: HashMap<String, String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            sandbox_type: SandboxType::Subprocess,
            max_memory_mb: 16,
            max_fuel: 1_000_000,
            max_duration_secs: 30,
            allowed_paths: vec![],
            env_vars: HashMap::new(),
        }
    }
}

/// 执行结果
#[derive(Debug, Clone)]
pub struct ExecResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
}

/// 沙箱 ID
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SandboxId(String);

impl SandboxId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// 沙箱运行时适配器 Trait
#[async_trait]
pub trait RuntimeAdapter: Send + Sync {
    /// 返回沙箱类型
    fn runtime_type(&self) -> SandboxType;

    /// 创建沙箱实例
    async fn create(&self, config: &SandboxConfig) -> Result<SandboxId, SandboxError>;

    /// 在沙箱中执行命令
    async fn execute(&self, id: &SandboxId, cmd: &str) -> Result<ExecResult, SandboxError>;

    /// 销毁沙箱实例
    async fn destroy(&self, id: &SandboxId) -> Result<(), SandboxError>;
}

/// 沙箱错误
#[derive(Debug, thiserror::Error)]
pub enum SandboxError {
    #[error("Sandbox creation failed: {0}")]
    CreateFailed(String),

    #[error("Sandbox execution failed: {0}")]
    ExecuteFailed(String),

    #[error("Sandbox destroyed: {0}")]
    DestroyFailed(String),

    #[error("Sandbox not found: {0}")]
    NotFound(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Timeout: {0}")]
    Timeout(String),
}
```

**Step 2: 运行 cargo check 验证**

Run: `cargo check -p octo-engine`
Expected: SUCCESS

**Step 3: 编写基础测试**

```rust
// crates/octo-engine/tests/sandbox_trait_test.rs

use octo_engine::sandbox::*;

#[tokio::test]
async fn test_sandbox_config_default() {
    let config = SandboxConfig::default();
    assert_eq!(config.sandbox_type, SandboxType::Subprocess);
    assert_eq!(config.max_memory_mb, 16);
    assert_eq!(config.max_duration_secs, 30);
}

#[tokio::test]
async fn test_sandbox_id() {
    let id = SandboxId::new("test-001");
    assert_eq!(id.0, "test-001");
}
```

**Step 4: 运行测试验证**

Run: `cargo test -p octo-engine sandbox_trait_test`
Expected: PASS

**Step 5: 提交**

```bash
git add crates/octo-engine/src/sandbox/traits.rs crates/octo-engine/tests/sandbox_trait_test.rs
git commit -m "feat(sandbox): define RuntimeAdapter trait"
```

---

## 任务 3: SubprocessAdapter (保留现有实现)

**Files:**
- Create: `crates/octo-engine/src/sandbox/subprocess.rs`
- Modify: `crates/octo-engine/src/sandbox/mod.rs`
- Test: `crates/octo-engine/tests/sandbox_subprocess_test.rs`

**Step 1: 实现 SubprocessAdapter**

```rust
// crates/octo-engine/src/sandbox/subprocess.rs

use super::{ExecResult, SandboxConfig, SandboxError, SandboxId, SandboxType, RuntimeAdapter};
use async_trait::async_trait;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::RwLock;

pub struct SubprocessAdapter {
    instances: Arc<RwLock<HashMap<SandboxId, SubprocessInstance>>>,
}

struct SubprocessInstance {
    config: SandboxConfig,
    working_dir: std::path::PathBuf,
}

impl SubprocessAdapter {
    pub fn new() -> Self {
        Self {
            instances: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for SubprocessAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RuntimeAdapter for SubprocessAdapter {
    fn runtime_type(&self) -> SandboxType {
        SandboxType::Subprocess
    }

    async fn create(&self, config: &SandboxConfig) -> Result<SandboxId, SandboxError> {
        let id = SandboxId::new(uuid::Uuid::new_v4().to_string());
        let instance = SubprocessInstance {
            config: config.clone(),
            working_dir: std::env::temp_dir().join(&id.0),
        };

        // 创建工作目录
        std::fs::create_dir_all(&instance.working_dir)
            .map_err(|e| SandboxError::CreateFailed(e.to_string()))?;

        let mut instances = self.instances.write().await;
        instances.insert(id.clone(), instance);

        Ok(id)
    }

    async fn execute(&self, id: &SandboxId, cmd: &str) -> Result<ExecResult, SandboxError> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(id)
            .ok_or_else(|| SandboxError::NotFound(id.0.clone()))?;

        let start = std::time::Instant::now();

        let mut cmd = Command::new("sh");
        cmd.arg("-c")
            .arg(cmd)
            .current_dir(&instance.working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env_clear();

        // 设置允许的环境变量
        for (key, value) in &instance.config.env_vars {
            cmd.env(key, value);
        }

        let output = cmd
            .output()
            .await
            .map_err(|e| SandboxError::ExecuteFailed(e.to_string()))?;

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(ExecResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            duration_ms,
        })
    }

    async fn destroy(&self, id: &SandboxId) -> Result<(), SandboxError> {
        let mut instances = self.instances.write().await;
        if let Some(instance) = instances.remove(id) {
            let _ = std::fs::remove_dir_all(&instance.working_dir);
        }
        Ok(())
    }
}
```

**Step 2: 运行 cargo check**

Run: `cargo check -p octo-engine`
Expected: SUCCESS

**Step 3: 编写测试**

```rust
// crates/octo-engine/tests/sandbox_subprocess_test.rs

use octo_engine::sandbox::*;

#[tokio::test]
async fn test_subprocess_create_and_execute() {
    let adapter = SubprocessAdapter::new();
    let config = SandboxConfig::default();

    // 创建沙箱
    let id = adapter.create(&config).await.unwrap();

    // 执行命令
    let result = adapter.execute(&id, "echo 'hello'").await.unwrap();
    assert_eq!(result.stdout.trim(), "hello");
    assert_eq!(result.exit_code, 0);

    // 销毁沙箱
    adapter.destroy(&id).await.unwrap();
}

#[tokio::test]
async fn test_subprocess_not_found() {
    let adapter = SubprocessAdapter::new();
    let fake_id = SandboxId::new("non-existent");

    let result = adapter.execute(&fake_id, "echo test").await;
    assert!(result.is_err());
}
```

**Step 4: 运行测试**

Run: `cargo test -p octo-engine sandbox_subprocess_test`
Expected: PASS

**Step 5: 提交**

```bash
git add crates/octo-engine/src/sandbox/subprocess.rs crates/octo-engine/tests/sandbox_subprocess_test.rs
git commit -m "feat(sandbox): implement SubprocessAdapter"
```

---

## 任务 4: WasmAdapter 实现

**Files:**
- Create: `crates/octo-engine/src/sandbox/wasm.rs`
- Modify: `crates/octo-engine/src/sandbox/mod.rs`
- Test: `crates/octo-engine/tests/sandbox_wasm_test.rs`

**Step 1: 添加依赖**

Modify: `crates/octo-engine/Cargo.toml`

```toml
[dependencies]
wasmtime = { version = "25", optional = true }
wasmtime-wasi = { version = "25", optional = true }

[features]
default = []
sandbox-wasm = ["wasmtime", "wasmtime-wasi"]
```

**Step 2: 实现 WasmAdapter (简化版)**

```rust
// crates/octo-engine/src/sandbox/wasm.rs

use super::{ExecResult, SandboxConfig, SandboxError, SandboxId, SandboxType, RuntimeAdapter};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

#[cfg(feature = "sandbox-wasm")]
use wasmtime::{Engine, Module, Store, Instance, WasiCtx};

pub struct WasmAdapter {
    instances: Arc<RwLock<HashMap<SandboxId, WasmInstance>>>,
    engine: Option<Engine>,
}

struct WasmInstance {
    config: SandboxConfig,
    #[cfg(feature = "sandbox-wasm")]
    store: Option<Store<WasiCtx>>,
}

impl WasmAdapter {
    pub fn new() -> Self {
        #[cfg(feature = "sandbox-wasm")]
        let engine = Engine::default();
        #[cfg(not(feature = "sandbox-wasm"))]
        let engine = None;

        Self {
            instances: Arc::new(RwLock::new(HashMap::new())),
            engine,
        }
    }

    /// 从 WASM 模块执行计算
    #[cfg(feature = "sandbox-wasm")]
    pub async fn execute_wasm(&self, id: &SandboxId, wasm_bytes: &[u8], func_name: &str) -> Result<ExecResult, SandboxError> {
        use std::time::Instant;
        let start = Instant::now();

        let module = Module::from_binary(self.engine.as_ref().unwrap(), wasm_bytes)
            .map_err(|e| SandboxError::ExecuteFailed(e.to_string()))?;

        let wasi = WasiCtx::default();
        let mut store = Store::new(self.engine.as_ref().unwrap(), wasi);

        let instance = Instance::new(&mut store, &module, &[])
            .map_err(|e| SandboxError::ExecuteFailed(e.to_string()))?;

        let func = instance
            .get_typed_func::<(), i32>(&mut store, func_name)
            .map_err(|e| SandboxError::ExecuteFailed(e.to_string()))?;

        let result = func.call(&mut store, ())
            .map_err(|e| SandboxError::ExecuteFailed(e.to_string()))?;

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(ExecResult {
            stdout: format!("Exit code: {}\n", result),
            stderr: String::new(),
            exit_code: result,
            duration_ms,
        })
    }

    #[cfg(not(feature = "sandbox-wasm"))]
    pub async fn execute_wasm(&self, _id: &SandboxId, _wasm_bytes: &[u8], _func_name: &str) -> Result<ExecResult, SandboxError> {
        Err(SandboxError::ConfigError("WASM support not enabled. Enable sandbox-wasm feature".to_string()))
    }
}

impl Default for WasmAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RuntimeAdapter for WasmAdapter {
    fn runtime_type(&self) -> SandboxType {
        SandboxType::Wasm
    }

    async fn create(&self, config: &SandboxConfig) -> Result<SandboxId, SandboxError> {
        let id = SandboxId::new(uuid::Uuid::new_v4().to_string());

        #[cfg(feature = "sandbox-wasm")]
        let instance = WasmInstance {
            config: config.clone(),
            store: None,
        };

        #[cfg(not(feature = "sandbox-wasm"))]
        let instance = WasmInstance {
            config: config.clone(),
        };

        let mut instances = self.instances.write().await;
        instances.insert(id.clone(), instance);

        Ok(id)
    }

    async fn execute(&self, id: &SandboxId, _cmd: &str) -> Result<ExecResult, SandboxError> {
        // WASM 沙箱目前需要显式加载 WASM 模块
        // 这里返回一个提示信息
        Err(SandboxError::ExecuteFailed(
            "WASM adapter requires explicit WASM module. Use execute_wasm() instead".to_string()
        ))
    }

    async fn destroy(&self, id: &SandboxId) -> Result<(), SandboxError> {
        let mut instances = self.instances.write().await;
        instances.remove(id);
        Ok(())
    }
}
```

**Step 3: 运行 cargo check**

Run: `cargo check -p octo-engine --features sandbox-wasm`
Expected: SUCCESS

**Step 4: 编写测试**

```rust
// crates/octo-engine/tests/sandbox_wasm_test.rs

use octo_engine::sandbox::*;

#[tokio::test]
async fn test_wasm_adapter_create() {
    let adapter = WasmAdapter::new();
    let config = SandboxConfig {
        sandbox_type: SandboxType::Wasm,
        max_memory_mb: 16,
        max_fuel: 1_000_000,
        max_duration_secs: 30,
        allowed_paths: vec![],
        env_vars: Default::default(),
    };

    let id = adapter.create(&config).await.unwrap();
    assert_eq!(adapter.runtime_type(), SandboxType::Wasm);

    adapter.destroy(&id).await.unwrap();
}
```

**Step 5: 运行测试**

Run: `cargo test -p octo-engine --features sandbox-wasm sandbox_wasm_test`
Expected: PASS

**Step 6: 提交**

```bash
git add crates/octo-engine/src/sandbox/wasm.rs crates/octo-engine/Cargo.toml crates/octo-engine/tests/sandbox_wasm_test.rs
git commit -m "feat(sandbox): implement WasmAdapter with wasmtime"
```

---

## 任务 5: DockerAdapter 实现

**Files:**
- Create: `crates/octo-engine/src/sandbox/docker.rs`
- Modify: `crates/octo-engine/src/sandbox/mod.rs`
- Test: `crates/octo-engine/tests/sandbox_docker_test.rs`

**Step 1: 添加依赖**

Modify: `crates/octo-engine/Cargo.toml`

```toml
[dependencies]
bollard = { version = "0.18", optional = true }

[features]
default = []
sandbox-docker = ["bollard"]
```

**Step 2: 实现 DockerAdapter**

```rust
// crates/octo-engine/src/sandbox/docker.rs

use super::{ExecResult, SandboxConfig, SandboxError, SandboxId, SandboxType, RuntimeAdapter};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg(feature = "sandbox-docker")]
use bollard::{Docker, ContainerConfig, ImageConfig, Config, create_container::CreateContainerOptions};

pub struct DockerAdapter {
    instances: Arc<RwLock<HashMap<SandboxId, DockerInstance>>>,
    image: String,
    #[cfg(feature = "sandbox-docker")]
    client: Option<Docker>,
}

struct DockerInstance {
    config: SandboxConfig,
    container_id: Option<String>,
}

impl DockerAdapter {
    pub fn new(image: impl Into<String>) -> Self {
        #[cfg(feature = "sandbox-docker")]
        let client = Docker::connect_with_local_defaults().ok();

        Self {
            instances: Arc::new(RwLock::new(HashMap::new())),
            image: image.into(),
            #[cfg(feature = "sandbox-docker")]
            client,
        }
    }

    /// 在 Docker 容器中执行命令
    #[cfg(feature = "sandbox-docker")]
    pub async fn execute_in_container(&self, id: &SandboxId, cmd: &str) -> Result<ExecResult, SandboxError> {
        use std::time::Instant;
        let start = Instant::now();

        let instances = self.instances.read().await;
        let instance = instances
            .get(id)
            .ok_or_else(|| SandboxError::NotFound(id.0.clone()))?;

        let container_id = instance
            .container_id
            .as_ref()
            .ok_or_else(|| SandboxError::ExecuteFailed("Container not started".to_string()))?;

        let client = self.client.as_ref()
            .ok_or_else(|| SandboxError::ConfigError("Docker client not available".to_string()))?;

        // 在容器中执行命令
        let output = client
            .exec_create(
                container_id,
                vec!["sh", "-c", cmd],
                None,
                None,
            )
            .await
            .map_err(|e| SandboxError::ExecuteFailed(e.to_string()))?;

        let output = client
            .exec_start(output.id, None)
            .await
            .map_err(|e| SandboxError::ExecuteFailed(e.to_string()))?;

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(ExecResult {
            stdout: String::from_utf8_lossy(&output).to_string(),
            stderr: String::new(),
            exit_code: 0,
            duration_ms,
        })
    }

    #[cfg(not(feature = "sandbox-docker"))]
    pub async fn execute_in_container(&self, _id: &SandboxId, _cmd: &str) -> Result<ExecResult, SandboxError> {
        Err(SandboxError::ConfigError("Docker support not enabled. Enable sandbox-docker feature".to_string()))
    }
}

#[async_trait]
impl RuntimeAdapter for DockerAdapter {
    fn runtime_type(&self) -> SandboxType {
        SandboxType::Docker
    }

    async fn create(&self, config: &SandboxConfig) -> Result<SandboxId, SandboxError> {
        let id = SandboxId::new(uuid::Uuid::new_v4().to_string());

        #[cfg(feature = "sandbox-docker")]
        {
            let client = self.client.as_ref()
                .ok_or_else(|| SandboxError::ConfigError("Docker client not available".to_string()))?;

            // 拉取镜像（如果不存在）
            let _ = client
                .create_image(
                    Some(bollard::image::CreateImageParams {
                        from_image: self.image.clone(),
                        ..Default::default()
                    }),
                    None,
                    None,
                )
                .await;

            // 创建容器
            let container_config = ContainerConfig {
                image: Some(self.image.clone()),
                cmd: Some(vec!["sleep", "infinity"]),
                env: Some(
                    config
                        .env_vars
                        .iter()
                        .map(|(k, v)| format!("{}={}", k, v))
                        .collect()
                ),
                ..Default::default()
            };

            let response = client
                .create_container(
                    None,
                    CreateContainerOptions {
                        name: id.0.clone(),
                        platform: None,
                    },
                    container_config,
                )
                .await
                .map_err(|e| SandboxError::CreateFailed(e.to_string()))?;

            // 启动容器
            client
                .start_container::<String>(&response.id, None)
                .await
                .map_err(|e| SandboxError::CreateFailed(e.to_string()))?;

            let instance = DockerInstance {
                config: config.clone(),
                container_id: Some(response.id),
            };

            let mut instances = self.instances.write().await;
            instances.insert(id.clone(), instance);
        }

        #[cfg(not(feature = "sandbox-docker"))]
        {
            let instance = DockerInstance {
                config: config.clone(),
                container_id: None,
            };

            let mut instances = self.instances.write().await;
            instances.insert(id.clone(), instance);
        }

        Ok(id)
    }

    async fn execute(&self, id: &SandboxId, cmd: &str) -> Result<ExecResult, SandboxError> {
        self.execute_in_container(id, cmd).await
    }

    async fn destroy(&self, id: &SandboxId) -> Result<(), SandboxError> {
        #[cfg(feature = "sandbox-docker")]
        {
            let mut instances = self.instances.write().await;
            if let Some(instance) = instances.remove(id) {
                if let Some(container_id) = instance.container_id {
                    if let Some(client) = &self.client {
                        let _ = client.stop_container(&container_id, None).await;
                        let _ = client.remove_container(&container_id, None).await;
                    }
                }
            }
        }

        #[cfg(not(feature = "sandbox-docker"))]
        {
            let mut instances = self.instances.write().await;
            instances.remove(id);
        }

        Ok(())
    }
}
```

**Step 3: 运行 cargo check**

Run: `cargo check -p octo-engine --features sandbox-docker`
Expected: SUCCESS

**Step 4: 编写测试**

```rust
// crates/octo-engine/tests/sandbox_docker_test.rs

use octo_engine::sandbox::*;

#[tokio::test]
async fn test_docker_adapter_create() {
    let adapter = DockerAdapter::new("alpine:latest");
    let config = SandboxConfig {
        sandbox_type: SandboxType::Docker,
        max_memory_mb: 512,
        max_fuel: 0,
        max_duration_secs: 60,
        allowed_paths: vec![],
        env_vars: Default::default(),
    };

    let id = adapter.create(&config).await.unwrap();
    assert_eq!(adapter.runtime_type(), SandboxType::Docker);

    // 清理
    let _ = adapter.destroy(&id).await;
}
```

**Step 5: 运行测试（可选，需要 Docker）**

Run: `cargo test -p octo-engine --features sandbox-docker sandbox_docker_test -- --ignored`
Expected: PASS (if Docker available)

**Step 6: 提交**

```bash
git add crates/octo-engine/src/sandbox/docker.rs crates/octo-engine/tests/sandbox_docker_test.rs
git commit -m "feat(sandbox): implement DockerAdapter with bollard"
```

---

## 任务 6: Tool -> Sandbox 路由

**Files:**
- Create: `crates/octo-engine/src/sandbox/router.rs`
- Modify: `crates/octo-engine/src/sandbox/mod.rs`
- Modify: `crates/octo-engine/src/tools/mod.rs`

**Step 1: 实现沙箱路由器**

```rust
// crates/octo-engine/src/sandbox/router.rs

use super::{RuntimeAdapter, SandboxType, SandboxConfig, SandboxId, ExecResult, SandboxError};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

/// 工具类型到沙箱的映射
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCategory {
    /// 需要完整 shell 环境
    Shell,
    /// 无状态计算任务
    Compute,
    /// 文件系统操作
    FileSystem,
    /// 网络请求
    Network,
}

/// 沙箱路由器 - 根据工具类型选择合适的沙箱
pub struct SandboxRouter {
    adapters: HashMap<SandboxType, Arc<dyn RuntimeAdapter>>,
    default_sandbox: SandboxType,
    tool_mapping: HashMap<ToolCategory, SandboxType>,
}

impl SandboxRouter {
    pub fn new() -> Self {
        let mut tool_mapping = HashMap::new();
        tool_mapping.insert(ToolCategory::Shell, SandboxType::Docker);
        tool_mapping.insert(ToolCategory::Compute, SandboxType::Wasm);
        tool_mapping.insert(ToolCategory::FileSystem, SandboxType::Docker);
        tool_mapping.insert(ToolCategory::Network, SandboxType::Wasm);

        Self {
            adapters: HashMap::new(),
            default_sandbox: SandboxType::Subprocess,
            tool_mapping,
        }
    }

    /// 注册沙箱适配器
    pub fn register_adapter(&mut self, adapter: Arc<dyn RuntimeAdapter>) {
        self.adapters.insert(adapter.runtime_type(), adapter);
    }

    /// 设置默认沙箱
    pub fn set_default(&mut self, sandbox_type: SandboxType) {
        self.default_sandbox = sandbox_type;
    }

    /// 设置工具类别到沙箱的映射
    pub fn set_tool_mapping(&mut self, category: ToolCategory, sandbox_type: SandboxType) {
        self.tool_mapping.insert(category, sandbox_type);
    }

    /// 获取工具类别对应的沙箱类型
    pub fn get_sandbox_type(&self, category: ToolCategory) -> SandboxType {
        self.tool_mapping.get(&category).copied().unwrap_or(self.default_sandbox)
    }

    /// 执行工具命令
    pub async fn execute(&self, category: ToolCategory, cmd: &str) -> Result<ExecResult, SandboxError> {
        let sandbox_type = self.get_sandbox_type(category);

        let adapter = self.adapters.get(&sandbox_type)
            .ok_or_else(|| SandboxError::NotFound(format!("{:?} adapter not registered", sandbox_type)))?;

        // 简化版：每次创建新沙箱执行
        let config = SandboxConfig::default();
        let id = adapter.create(&config).await?;
        let result = adapter.execute(&id, cmd).await;
        let _ = adapter.destroy(&id).await;

        result
    }
}

impl Default for SandboxRouter {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 2: 运行 cargo check**

Run: `cargo check -p octo-engine`
Expected: SUCCESS

**Step 3: 编写测试**

```rust
// crates/octo-engine/tests/sandbox_router_test.rs

use octo_engine::sandbox::*;

#[tokio::test]
async fn test_router_tool_mapping() {
    let mut router = SandboxRouter::new();

    // 默认应该映射到 Subprocess
    assert_eq!(router.get_sandbox_type(ToolCategory::Shell), SandboxType::Docker);
    assert_eq!(router.get_sandbox_type(ToolCategory::Compute), SandboxType::Wasm);
}
```

**Step 4: 运行测试**

Run: `cargo test -p octo-engine sandbox_router_test`
Expected: PASS

**Step 5: 提交**

```bash
git add crates/octo-engine/src/sandbox/router.rs crates/octo-engine/tests/sandbox_router_test.rs
git commit -m "feat(sandbox): implement SandboxRouter for tool routing"
```

---

## 任务 7: 集成到工具系统

**Files:**
- Modify: `crates/octo-engine/src/tools/bash.rs`
- Modify: `crates/octo-engine/src/tools/mod.rs`

**Step 1: 修改 BashTool 使用沙箱**

```rust
// crates/octo-engine/src/tools/bash.rs

use crate::sandbox::{SandboxRouter, ToolCategory, SandboxType, SandboxConfig};
use crate::tools::{Tool, ToolCall, ToolResult};

// 替换现有的 bash 执行逻辑
pub async fn execute_bash_in_sandbox(
    router: &SandboxRouter,
    command: &str,
) -> Result<String, String> {
    // 使用 Docker 沙箱执行 bash 命令
    let result = router
        .execute(ToolCategory::Shell, command)
        .await
        .map_err(|e| e.to_string())?;

    if result.exit_code != 0 {
        return Err(format!("Command failed: {}", result.stderr));
    }

    Ok(result.stdout)
}
```

**Step 2: 添加 feature flag 到 Cargo.toml**

```toml
[features]
default = []
sandbox-wasm = ["octo-engine/sandbox-wasm"]
sandbox-docker = ["octo-engine/sandbox-docker"]
```

**Step 3: 运行 cargo check**

Run: `cargo check -p octo-server`
Expected: SUCCESS

**Step 4: 提交**

```bash
git add crates/octo-engine/src/tools/bash.rs crates/octo-engine/Cargo.toml
git commit -m "feat(sandbox): integrate sandbox into bash tool"
```

---

## 验收标准

1. ✅ RuntimeAdapter Trait 定义完成
2. ✅ SubprocessAdapter 保留现有功能
3. ✅ WasmAdapter 可编译（需要 sandbox-wasm feature）
4. ✅ DockerAdapter 可编译（需要 sandbox-docker feature）
5. ✅ SandboxRouter 可根据工具类型路由
6. ✅ 所有测试通过

---

## 依赖

```toml
# crates/octo-engine/Cargo.toml

[dependencies]
wasmtime = { version = "25", optional = true }
wasmtime-wasi = { version = "25", optional = true }
bollard = { version = "0.18", optional = true }
uuid = { version = "1", features = ["v4"] }
thiserror = "2"
async-trait = "0.1"

[features]
default = []
sandbox-wasm = ["wasmtime", "wasmtime-wasi"]
sandbox-docker = ["bollard"]
```
