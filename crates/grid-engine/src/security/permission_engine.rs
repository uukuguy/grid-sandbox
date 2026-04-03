//! PermissionEngine — 6-layer rule engine for tool call authorization.
//!
//! Evaluation order:
//! 1. Deny rules pierce through all layers (highest to lowest priority)
//! 2. Allow/Ask use first-match semantics (highest to lowest priority)
//! 3. No match → UseToolDefault (fall back to Tool trait declaration)

use std::path::Path;

use tracing::debug;

use super::permission_rule::{load_rules_from_yaml, PermissionRuleSet};
use super::permission_types::{PermissionDecision, RuleSource};

/// 6-layer permission engine for tool call authorization.
///
/// Rules are organized by source (Platform > Tenant > Project > User > Session > ToolDefault).
/// Deny rules from any layer cannot be overridden by allow rules from lower layers.
pub struct PermissionEngine {
    rule_sets: Vec<PermissionRuleSet>,
}

impl PermissionEngine {
    /// Create an empty engine (all calls → UseToolDefault)
    pub fn empty() -> Self {
        Self {
            rule_sets: Vec::new(),
        }
    }

    /// Load rules from project and/or user YAML files (for CLI/workbench).
    pub fn from_files(
        project_rules: Option<&Path>,
        user_rules: Option<&Path>,
    ) -> anyhow::Result<Self> {
        let mut engine = Self::empty();

        // User-level rules (lower priority, loaded first)
        if let Some(path) = user_rules {
            if path.exists() {
                match load_rules_from_yaml(path, RuleSource::User) {
                    Ok(rules) => {
                        debug!(path = %path.display(), "Loaded user permission rules");
                        engine.add_rule_set(rules);
                    }
                    Err(e) => {
                        tracing::warn!(
                            path = %path.display(),
                            error = %e,
                            "Failed to load user permission rules"
                        );
                    }
                }
            }
        }

        // Project-level rules (higher priority)
        if let Some(path) = project_rules {
            if path.exists() {
                match load_rules_from_yaml(path, RuleSource::Project) {
                    Ok(rules) => {
                        debug!(path = %path.display(), "Loaded project permission rules");
                        engine.add_rule_set(rules);
                    }
                    Err(e) => {
                        tracing::warn!(
                            path = %path.display(),
                            error = %e,
                            "Failed to load project permission rules"
                        );
                    }
                }
            }
        }

        Ok(engine)
    }

    /// Add a rule set. Sets are kept sorted by source priority.
    pub fn add_rule_set(&mut self, rules: PermissionRuleSet) {
        self.rule_sets.push(rules);
        // Sort by source (lower ordinal = higher priority = checked first)
        self.rule_sets.sort_by_key(|r| r.source);
    }

    /// Add session-level temporary rules.
    pub fn add_session_rules(&mut self, rules: PermissionRuleSet) {
        self.add_rule_set(rules);
    }

    /// Evaluate a tool call against all rule layers.
    ///
    /// Algorithm:
    /// 1. Scan ALL layers for deny matches — deny pierces through (cannot be overridden)
    /// 2. Scan layers in priority order for allow/ask — first match wins
    /// 3. No match → UseToolDefault
    pub fn evaluate(
        &self,
        tool_name: &str,
        input: &serde_json::Value,
    ) -> PermissionDecision {
        // Phase 1: Check deny rules across all layers (deny pierces through)
        for rule_set in &self.rule_sets {
            for rule in &rule_set.deny_rules {
                if rule.matches(tool_name, input) {
                    let decision = PermissionDecision::Deny {
                        source: rule_set.source,
                        rule_description: rule.description(),
                        reason: format!(
                            "{:?} rule denies {}",
                            rule_set.source,
                            rule.description()
                        ),
                    };
                    debug!(
                        tool = tool_name,
                        decision = %decision,
                        "Permission denied by rule"
                    );
                    return decision;
                }
            }
        }

        // Phase 2: Check allow/ask rules (first match wins, priority order)
        for rule_set in &self.rule_sets {
            for rule in &rule_set.allow_rules {
                if rule.matches(tool_name, input) {
                    let decision = PermissionDecision::Allow {
                        source: rule_set.source,
                        rule_description: rule.description(),
                    };
                    debug!(
                        tool = tool_name,
                        decision = %decision,
                        "Permission allowed by rule"
                    );
                    return decision;
                }
            }
            for rule in &rule_set.ask_rules {
                if rule.matches(tool_name, input) {
                    let decision = PermissionDecision::Ask {
                        source: rule_set.source,
                        rule_description: rule.description(),
                    };
                    debug!(
                        tool = tool_name,
                        decision = %decision,
                        "Permission requires approval"
                    );
                    return decision;
                }
            }
        }

        // Phase 3: No rule matched
        PermissionDecision::UseToolDefault
    }

    /// Check if any rules are loaded
    pub fn has_rules(&self) -> bool {
        !self.rule_sets.is_empty()
    }

    /// Total number of rules across all layers
    pub fn rule_count(&self) -> usize {
        self.rule_sets
            .iter()
            .map(|rs| rs.allow_rules.len() + rs.deny_rules.len() + rs.ask_rules.len())
            .sum()
    }
}

impl std::fmt::Debug for PermissionEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PermissionEngine")
            .field("layers", &self.rule_sets.len())
            .field("total_rules", &self.rule_count())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::permission_rule::{PermissionRule, PermissionRuleSet};
    use crate::security::permission_types::RuleSource;
    use serde_json::json;

    fn make_rule_set(
        source: RuleSource,
        allow: &[&str],
        deny: &[&str],
        ask: &[&str],
    ) -> PermissionRuleSet {
        let mut rs = PermissionRuleSet::new(source);
        for s in allow {
            rs.allow_rules.push(PermissionRule::parse(s).unwrap());
        }
        for s in deny {
            rs.deny_rules.push(PermissionRule::parse(s).unwrap());
        }
        for s in ask {
            rs.ask_rules.push(PermissionRule::parse(s).unwrap());
        }
        rs
    }

    #[test]
    fn test_empty_engine_returns_default() {
        let engine = PermissionEngine::empty();
        let decision = engine.evaluate("bash", &json!({"command": "ls"}));
        assert!(matches!(decision, PermissionDecision::UseToolDefault));
    }

    #[test]
    fn test_allow_rule() {
        let mut engine = PermissionEngine::empty();
        engine.add_rule_set(make_rule_set(
            RuleSource::User,
            &["bash(git *)"],
            &[],
            &[],
        ));
        let decision = engine.evaluate("bash", &json!({"command": "git status"}));
        assert!(decision.is_allow());
    }

    #[test]
    fn test_deny_rule() {
        let mut engine = PermissionEngine::empty();
        engine.add_rule_set(make_rule_set(
            RuleSource::Project,
            &[],
            &["bash(rm -rf *)"],
            &[],
        ));
        let decision = engine.evaluate("bash", &json!({"command": "rm -rf /tmp"}));
        assert!(decision.is_deny());
    }

    #[test]
    fn test_ask_rule() {
        let mut engine = PermissionEngine::empty();
        engine.add_rule_set(make_rule_set(
            RuleSource::User,
            &[],
            &[],
            &["bash(pip install *)"],
        ));
        let decision = engine.evaluate("bash", &json!({"command": "pip install requests"}));
        assert!(decision.is_ask());
    }

    #[test]
    fn test_deny_pierces_through_allow() {
        let mut engine = PermissionEngine::empty();
        // Project denies rm -rf
        engine.add_rule_set(make_rule_set(
            RuleSource::Project,
            &[],
            &["bash(rm -rf *)"],
            &[],
        ));
        // User allows all bash — but deny from Project should pierce through
        engine.add_rule_set(make_rule_set(
            RuleSource::User,
            &["bash"],
            &[],
            &[],
        ));
        let decision = engine.evaluate("bash", &json!({"command": "rm -rf /"}));
        assert!(decision.is_deny(), "Deny should pierce through allow: {decision}");
    }

    #[test]
    fn test_higher_priority_allow_wins() {
        let mut engine = PermissionEngine::empty();
        // Project allows git
        engine.add_rule_set(make_rule_set(
            RuleSource::Project,
            &["bash(git *)"],
            &[],
            &[],
        ));
        // User asks for all bash
        engine.add_rule_set(make_rule_set(
            RuleSource::User,
            &[],
            &[],
            &["bash"],
        ));
        // Project (priority 3) beats User (priority 4)
        let decision = engine.evaluate("bash", &json!({"command": "git push"}));
        assert!(decision.is_allow(), "Project allow should win over User ask: {decision}");
    }

    #[test]
    fn test_no_match_falls_through() {
        let mut engine = PermissionEngine::empty();
        engine.add_rule_set(make_rule_set(
            RuleSource::User,
            &["bash(git *)"],
            &[],
            &[],
        ));
        // cargo test doesn't match "git *"
        let decision = engine.evaluate("bash", &json!({"command": "cargo test"}));
        assert!(matches!(decision, PermissionDecision::UseToolDefault));
    }

    #[test]
    fn test_wildcard_tool() {
        let mut engine = PermissionEngine::empty();
        engine.add_rule_set(make_rule_set(
            RuleSource::Session,
            &["*"],
            &[],
            &[],
        ));
        let decision = engine.evaluate("file_read", &json!({"file_path": "/etc/hosts"}));
        assert!(decision.is_allow());
    }

    #[test]
    fn test_multi_layer_merge() {
        let mut engine = PermissionEngine::empty();
        // Platform denies file_edit on /etc
        engine.add_rule_set(make_rule_set(
            RuleSource::Platform,
            &[],
            &["file_edit(/etc/**)"],
            &[],
        ));
        // Project allows file_edit on src
        engine.add_rule_set(make_rule_set(
            RuleSource::Project,
            &["file_edit(src/**)"],
            &[],
            &[],
        ));
        // User allows all file_edit
        engine.add_rule_set(make_rule_set(
            RuleSource::User,
            &["file_edit"],
            &[],
            &[],
        ));

        // /etc/passwd → denied by Platform
        assert!(engine.evaluate("file_edit", &json!({"file_path": "/etc/passwd"})).is_deny());
        // src/main.rs → allowed by Project
        assert!(engine.evaluate("file_edit", &json!({"file_path": "src/main.rs"})).is_allow());
        // other → allowed by User
        assert!(engine.evaluate("file_edit", &json!({"file_path": "README.md"})).is_allow());
    }

    #[test]
    fn test_rule_count() {
        let mut engine = PermissionEngine::empty();
        engine.add_rule_set(make_rule_set(
            RuleSource::User,
            &["file_read", "grep"],
            &["bash(rm -rf *)"],
            &["file_write"],
        ));
        assert_eq!(engine.rule_count(), 4);
    }
}
