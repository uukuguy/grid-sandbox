//! eaasp-certifier — EAASP v2.0 Runtime Contract verification library.
//!
//! Verifies that a gRPC endpoint correctly implements the v2 16-method
//! RuntimeService contract, split into 12 MUST + 4 OPTIONAL + 1 PLACEHOLDER.

pub mod mock_l3;
pub mod report;
pub mod runtime_pool;
pub mod blindbox;
pub mod selector;
pub mod v2_must_methods;
pub mod verifier;

/// Generated gRPC types from EAASP v2 proto (common + runtime).
pub mod proto {
    tonic::include_proto!("eaasp.runtime.v2");
}
