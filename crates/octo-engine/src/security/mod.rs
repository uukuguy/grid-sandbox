//! Security module for octo-engine.
//!
//! Provides security policies, action tracking, and command/path validation
//! for safe tool execution in enterprise environments.

pub mod policy;
pub mod tracker;

pub use policy::{AutonomyLevel, CommandRiskLevel, SecurityPolicy};
pub use tracker::ActionTracker;
