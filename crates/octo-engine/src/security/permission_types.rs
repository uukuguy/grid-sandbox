//! Permission system type definitions for the 6-layer rule engine.

use serde::{Deserialize, Serialize};

/// Rule source — determines priority (lower ordinal = higher priority).
/// Deny rules pierce through all layers; allow/ask use first-match semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RuleSource {
    /// Platform administrator rules (octo-platform-server, from DB)
    Platform = 1,
    /// Tenant administrator rules (octo-platform-server, from DB)
    Tenant = 2,
    /// Project-level rules ($PROJECT/.octo/security_rules.yaml, git-committed)
    Project = 3,
    /// User-level rules (~/.octo/security_rules.yaml)
    User = 4,
    /// Session-level rules (CLI args / API request)
    Session = 5,
    /// Tool's own default declaration (Tool trait risk_level/approval)
    ToolDefault = 6,
}

/// Permission behavior for a rule
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionBehavior {
    Allow,
    Deny,
    Ask,
}

/// Decision returned by PermissionEngine::evaluate()
#[derive(Debug, Clone)]
pub enum PermissionDecision {
    /// Rule explicitly allows the tool call
    Allow {
        source: RuleSource,
        rule_description: String,
    },
    /// Rule explicitly denies the tool call
    Deny {
        source: RuleSource,
        rule_description: String,
        reason: String,
    },
    /// Rule requires human confirmation
    Ask {
        source: RuleSource,
        rule_description: String,
    },
    /// No rule matched — fall back to Tool trait default
    UseToolDefault,
}

impl PermissionDecision {
    /// Whether this decision allows execution without further checks
    pub fn is_allow(&self) -> bool {
        matches!(self, Self::Allow { .. })
    }

    /// Whether this decision denies execution
    pub fn is_deny(&self) -> bool {
        matches!(self, Self::Deny { .. })
    }

    /// Whether this decision requires asking the user
    pub fn is_ask(&self) -> bool {
        matches!(self, Self::Ask { .. })
    }
}

impl std::fmt::Display for PermissionDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Allow { source, .. } => write!(f, "Allow ({source:?})"),
            Self::Deny { source, reason, .. } => write!(f, "Deny ({source:?}): {reason}"),
            Self::Ask { source, .. } => write!(f, "Ask ({source:?})"),
            Self::UseToolDefault => write!(f, "UseToolDefault"),
        }
    }
}
