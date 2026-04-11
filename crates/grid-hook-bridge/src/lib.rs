//! grid-hook-bridge — EAASP HookBridge for L1 runtime hook evaluation.
//!
//! Provides two modes:
//! - `InProcessHookBridge` — in-process evaluation (testing, T1 simulation)
//! - `GrpcHookBridge` — gRPC client to external HookBridge sidecar (T2/T3)
//!
//! Also includes `HookBridgeGrpcServer` — gRPC server for sidecar deployment.
//!
//! # Proto packaging (v2)
//!
//! EAASP v2.0 collapses `common.v1`, `runtime.v1`, and `hook.v1` into a
//! single `eaasp.runtime.v2` package. All generated types live under
//! `crate::proto`.

pub mod grpc_bridge;
pub mod in_process;
pub mod server;
pub mod traits;

/// Generated gRPC types from EAASP v2 proto (common + runtime + hook).
pub mod proto {
    tonic::include_proto!("eaasp.runtime.v2");
}
