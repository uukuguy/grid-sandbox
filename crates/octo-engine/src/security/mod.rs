//! Security module for octo-engine.
//!
//! Provides security policies, action tracking, and command/path validation
//! for safe tool execution in enterprise environments.

pub mod ai_defence;
pub mod permission_engine;
pub mod permission_rule;
pub mod permission_types;
pub mod pipeline;
pub mod policy;
pub mod tracker;

pub use ai_defence::{AiDefence, DefenceViolation, InjectionDetector, OutputValidator, PiiScanner};
pub use pipeline::{
    CanaryGuardLayer, CredentialScrubber, InjectionDetectorLayer, PiiScannerLayer, SafetyDecision,
    SafetyLayer, SafetyPipeline,
};
pub use permission_engine::PermissionEngine;
pub use permission_rule::{PermissionRule, PermissionRuleSet};
pub use permission_types::{PermissionBehavior, PermissionDecision, RuleSource};
pub use policy::{AutonomyLevel, CommandRiskLevel, SecurityPolicy};
pub use tracker::ActionTracker;
