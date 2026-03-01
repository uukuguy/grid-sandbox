//! WebAssembly sandbox adapter using wasmtime
//!
//! This adapter provides secure WASM module execution using the wasmtime runtime.
//! It is feature-gated behind the `sandbox-wasm` feature flag.

use super::{ExecResult, RuntimeAdapter, SandboxConfig, SandboxError, SandboxId, SandboxType};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// WebAssembly sandbox adapter using wasmtime
///
/// This adapter executes WASM modules in a secure, sandboxed environment.
/// It requires the `sandbox-wasm` feature to be enabled.
pub struct WasmAdapter {
    /// Active WASM instances
    instances: Arc<RwLock<HashMap<SandboxId, WasmInstance>>>,

    /// WASM engine (only available with sandbox-wasm feature)
    #[cfg(feature = "sandbox-wasm")]
    engine: Option<wasmtime::Engine>,
}

/// Internal representation of a WASM sandbox instance
struct WasmInstance {
    /// Sandbox configuration
    config: SandboxConfig,
    /// Module bytes (kept for potential re-execution)
    module_bytes: Vec<u8>,
}

impl WasmAdapter {
    /// Create a new WasmAdapter
    pub fn new() -> Self {
        #[cfg(feature = "sandbox-wasm")]
        let engine = wasmtime::Engine::default();

        Self {
            instances: Arc::new(RwLock::new(HashMap::new())),
            #[cfg(feature = "sandbox-wasm")]
            engine: Some(engine),
        }
    }

    /// Execute a WASM module with a specific function name
    ///
    /// This is a convenience method for executing WASM modules directly.
    #[cfg(feature = "sandbox-wasm")]
    pub async fn execute_wasm(
        &self,
        id: &SandboxId,
        wasm_bytes: &[u8],
        func_name: &str,
    ) -> Result<ExecResult, SandboxError> {
        // Validate sandbox exists
        let instances = self.instances.read().await;
        if !instances.contains_key(id) {
            return Err(SandboxError::NotFound(id.clone()));
        }
        drop(instances);

        let start = std::time::Instant::now();

        // Create store and module
        let engine = self
            .engine
            .as_ref()
            .ok_or_else(|| SandboxError::ExecutionFailed("WASM engine not initialized".into()))?;

        // Load the WASM module
        let module = wasmtime::Module::from_binary(engine, wasm_bytes)
            .map_err(|e| SandboxError::ExecutionFailed(format!("Failed to load WASM module: {}", e)))?;

        // Create a basic store without WASI
        let mut store = wasmtime::Store::new(engine, ());

        // Create a linker for basic imports (needs engine, not store)
        let mut linker = wasmtime::Linker::new(store.engine());

        // Try to instantiate the module
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| SandboxError::ExecutionFailed(format!("Failed to instantiate WASM: {}", e)))?;

        // Try to get the requested function
        let duration_ms = start.elapsed().as_millis() as u64;

        // Try different function signatures (get_typed_func returns Result, not Option)
        if let Ok(func) = instance.get_typed_func::<(), i32>(&mut store, func_name) {
            match func.call(&mut store, ()) {
                Ok(exit_code) => {
                    tracing::debug!(
                        "Executed WASM function '{}' in sandbox {}: exit_code={}, duration_ms={}",
                        func_name,
                        id,
                        exit_code,
                        duration_ms
                    );

                    return Ok(ExecResult {
                        stdout: format!("WASM function '{}' executed successfully", func_name),
                        stderr: String::new(),
                        exit_code,
                        execution_time_ms: duration_ms,
                        success: true,
                    });
                }
                Err(trap) => {
                    let err_msg = format!("WASM trap: {}", trap);
                    tracing::error!("WASM execution failed in sandbox {}: {}", id, err_msg);

                    return Ok(ExecResult {
                        stdout: String::new(),
                        stderr: err_msg,
                        exit_code: -1,
                        execution_time_ms: duration_ms,
                        success: false,
                    });
                }
            }
        }

        // Try () -> ()
        if let Ok(func) = instance.get_typed_func::<(), ()>(&mut store, func_name) {
            match func.call(&mut store, ()) {
                Ok(()) => {
                    tracing::debug!(
                        "Executed WASM function '{}' in sandbox {}: duration_ms={}",
                        func_name,
                        id,
                        duration_ms
                    );

                    return Ok(ExecResult {
                        stdout: format!("WASM function '{}' executed successfully", func_name),
                        stderr: String::new(),
                        exit_code: 0,
                        execution_time_ms: duration_ms,
                        success: true,
                    });
                }
                Err(trap) => {
                    let err_msg = format!("WASM trap: {}", trap);
                    tracing::error!("WASM execution failed in sandbox {}: {}", id, err_msg);

                    return Ok(ExecResult {
                        stdout: String::new(),
                        stderr: err_msg,
                        exit_code: -1,
                        execution_time_ms: duration_ms,
                        success: false,
                    });
                }
            }
        }

        // Function not found
        Ok(ExecResult {
            stdout: format!(
                "WASM module loaded successfully, function '{}' not found",
                func_name
            ),
            stderr: String::new(),
            exit_code: 0,
            execution_time_ms: duration_ms,
            success: true,
        })
    }

    /// Execute a WASM module with a specific function name (stub without feature)
    #[cfg(not(feature = "sandbox-wasm"))]
    pub async fn execute_wasm(
        &self,
        _id: &SandboxId,
        _wasm_bytes: &[u8],
        _func_name: &str,
    ) -> Result<ExecResult, SandboxError> {
        Err(SandboxError::UnsupportedType(
            "WASM support not enabled. Enable sandbox-wasm feature".to_string(),
        ))
    }

    /// Check if WASM support is available
    pub fn is_available(&self) -> bool {
        #[cfg(feature = "sandbox-wasm")]
        return self.engine.is_some();

        #[cfg(not(feature = "sandbox-wasm"))]
        return false;
    }
}

impl Default for WasmAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeAdapter for WasmAdapter {
    /// Get the sandbox type
    fn sandbox_type(&self) -> SandboxType {
        SandboxType::Wasm
    }

    /// Create a new WASM sandbox instance
    async fn create(&self, config: &SandboxConfig) -> Result<SandboxId, SandboxError> {
        #[cfg(not(feature = "sandbox-wasm"))]
        {
            return Err(SandboxError::UnsupportedType(
                "WASM support not enabled. Enable sandbox-wasm feature".to_string(),
            ));
        }

        #[cfg(feature = "sandbox-wasm")]
        {
            // Check if engine is available
            if self.engine.is_none() {
                return Err(SandboxError::ExecutionFailed(
                    "WASM engine not initialized".to_string(),
                ));
            }

            let id = SandboxId::new(uuid::Uuid::new_v4().to_string());

            // Validate config
            if config.memory_limit.is_some() {
                tracing::debug!(
                    "Memory limit specified: {} bytes",
                    config.memory_limit.unwrap()
                );
            }
            if config.time_limit.is_some() {
                tracing::debug!(
                    "Time limit specified: {} seconds",
                    config.time_limit.unwrap()
                );
            }

            let instance = WasmInstance {
                config: config.clone(),
                module_bytes: Vec::new(),
            };

            let mut instances = self.instances.write().await;
            instances.insert(id.clone(), instance);

            tracing::debug!("Created WASM sandbox: {}", id);
            Ok(id)
        }
    }

    /// Execute code in the WASM sandbox
    async fn execute(
        &self,
        id: &SandboxId,
        code: &str,
        _language: &str,
    ) -> Result<ExecResult, SandboxError> {
        #[cfg(not(feature = "sandbox-wasm"))]
        {
            let _ = (id, code);
            return Err(SandboxError::UnsupportedType(
                "WASM support not enabled. Enable sandbox-wasm feature".to_string(),
            ));
        }

        #[cfg(feature = "sandbox-wasm")]
        {
            let instances = self.instances.read().await;

            // Check if sandbox exists
            if !instances.contains_key(id) {
                return Err(SandboxError::NotFound(id.clone()));
            }

            drop(instances);

            let start = std::time::Instant::now();

            // Try to decode as base64 WASM module
            let wasm_bytes = match base64_decode(code) {
                Ok(bytes) => bytes,
                Err(_) => {
                    // If not valid base64, treat as a simple function name to call
                    tracing::debug!(
                        "Code is not WASM module, treating as function name: {}",
                        code
                    );
                    return self.execute_wasm(id, &[], code).await;
                }
            };

            // If we have actual WASM bytes, try to execute a default function
            if !wasm_bytes.is_empty() {
                return self.execute_wasm(id, &wasm_bytes, "_start").await;
            }

            let duration_ms = start.elapsed().as_millis() as u64;

            Ok(ExecResult {
                stdout: "WASM sandbox ready".to_string(),
                stderr: String::new(),
                exit_code: 0,
                execution_time_ms: duration_ms,
                success: true,
            })
        }
    }

    /// Destroy a WASM sandbox instance
    async fn destroy(&self, id: &SandboxId) -> Result<(), SandboxError> {
        #[cfg(not(feature = "sandbox-wasm"))]
        {
            let _ = id;
            return Err(SandboxError::UnsupportedType(
                "WASM support not enabled. Enable sandbox-wasm feature".to_string(),
            ));
        }

        #[cfg(feature = "sandbox-wasm")]
        {
            let mut instances = self.instances.write().await;

            if instances.remove(id).is_some() {
                tracing::debug!("Destroyed WASM sandbox: {}", id);
            }

            Ok(())
        }
    }

    /// Check if the sandbox is ready
    async fn is_ready(&self) -> bool {
        #[cfg(feature = "sandbox-wasm")]
        return self.engine.is_some();

        #[cfg(not(feature = "sandbox-wasm"))]
        return false;
    }
}

/// Decode base64 string to bytes
#[cfg(feature = "sandbox-wasm")]
fn base64_decode(input: &str) -> Result<Vec<u8>, SandboxError> {
    use base64::{engine::general_purpose::STANDARD, Engine};

    // Try standard base64
    if let Ok(bytes) = STANDARD.decode(input) {
        return Ok(bytes);
    }

    // Try URL-safe base64
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    URL_SAFE_NO_PAD
        .decode(input)
        .map_err(|e| SandboxError::ExecutionFailed(format!("Failed to decode base64: {}", e)))
}
