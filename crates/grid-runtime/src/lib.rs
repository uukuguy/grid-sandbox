//! grid-runtime — EAASP L1 Agent Runtime (Tier 1 Harness)
//!
//! This crate implements the EAASP 16-method Runtime Contract as a gRPC
//! service. Grid is a Tier 1 Harness runtime with native hooks, MCP,
//! and skills support — zero adapter overhead.
//!
//! ## Architecture
//!
//! - `contract` — RuntimeContract trait + Rust-native types (P1-P5 priority blocks)
//! - `session_payload` — `SessionPayload::trim_for_budget()` helper
//! - `harness` — GridHarness: impl RuntimeContract via grid-engine
//! - `service` — gRPC service mapping to v2 RuntimeService
//! - `telemetry` — EAASP telemetry event collection and conversion
//!
//! ## Proto (v2.0)
//!
//! The gRPC service + shared types live under a single
//! `eaasp.runtime.v2` package sourced from `proto/eaasp/runtime/v2/`.
//! v2 collapses v1's `common`/`runtime` split into one flat package and
//! introduces the 5-block structured SessionPayload (P1-P5) per spec §8.6.

pub mod config;
pub mod contract;
pub mod harness;
pub mod l2_client;
pub mod service;
pub mod session_payload;
pub mod telemetry;

/// Generated gRPC types from EAASP v2 proto (common + runtime).
pub mod proto {
    tonic::include_proto!("eaasp.runtime.v2");
}
