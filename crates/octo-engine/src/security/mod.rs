//! Security module for octo-engine.
//!
//! Provides security policies, action tracking, and command/path validation
//! for safe tool execution in enterprise environments.

pub mod ai_defence;
pub mod policy;
pub mod tracker;

pub use ai_defence::{AiDefence, DefenceViolation, InjectionDetector, OutputValidator, PiiScanner};
pub use policy::{AutonomyLevel, CommandRiskLevel, SecurityPolicy};
pub use tracker::ActionTracker;
