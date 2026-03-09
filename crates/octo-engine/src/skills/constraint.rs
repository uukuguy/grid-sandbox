use octo_types::skill::SkillDefinition;

/// Result of a constraint check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstraintResult {
    Allowed,
    Denied(String),
}

/// Enforces tool constraints from active skills.
///
/// Merges allowed/denied tool patterns from all active skills.
/// Denied patterns always take priority over allowed patterns.
/// Supports simple glob matching where `*` matches any characters.
pub struct ToolConstraintEnforcer {
    allowed_patterns: Vec<String>,
    denied_patterns: Vec<String>,
    has_constraints: bool,
}

/// Check if a tool name matches a glob pattern.
/// Supports `*` as a wildcard that matches any sequence of characters.
fn glob_match(pattern: &str, name: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if !pattern.contains('*') {
        return pattern == name;
    }

    let parts: Vec<&str> = pattern.split('*').collect();

    // Single wildcard: prefix*suffix
    if parts.len() == 2 {
        let prefix = parts[0];
        let suffix = parts[1];
        return name.len() >= prefix.len() + suffix.len()
            && name.starts_with(prefix)
            && name.ends_with(suffix);
    }

    // Multiple wildcards: greedy left-to-right matching
    let mut remaining = name;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if i == 0 {
            // First segment must be a prefix
            if !remaining.starts_with(part) {
                return false;
            }
            remaining = &remaining[part.len()..];
        } else if i == parts.len() - 1 {
            // Last segment must be a suffix
            if !remaining.ends_with(part) {
                return false;
            }
        } else if let Some(pos) = remaining.find(part) {
            remaining = &remaining[pos + part.len()..];
        } else {
            return false;
        }
    }
    true
}

impl ToolConstraintEnforcer {
    /// Build from a set of active skills.
    ///
    /// Merges all skills' `allowed_tools` and `denied_tools`.
    /// If no skills have constraints, all tools are allowed (backward compatibility).
    pub fn from_active_skills(skills: &[SkillDefinition]) -> Self {
        let mut allowed_patterns = Vec::new();
        let mut denied_patterns = Vec::new();
        let mut has_constraints = false;

        for skill in skills {
            if let Some(ref allowed) = skill.allowed_tools {
                has_constraints = true;
                allowed_patterns.extend(allowed.iter().cloned());
            }
            if let Some(ref denied) = skill.denied_tools {
                has_constraints = true;
                denied_patterns.extend(denied.iter().cloned());
            }
        }

        Self {
            allowed_patterns,
            denied_patterns,
            has_constraints,
        }
    }

    /// Check if a tool is permitted.
    ///
    /// Logic:
    /// 1. If no constraints exist, all tools are allowed.
    /// 2. If the tool matches any denied pattern, it is denied.
    /// 3. If allowed patterns exist and the tool matches one, it is allowed.
    /// 4. If allowed patterns exist but the tool matches none, it is denied.
    /// 5. If only denied patterns exist (no allowed), everything not denied is allowed.
    pub fn check(&self, tool_name: &str) -> ConstraintResult {
        if !self.has_constraints {
            return ConstraintResult::Allowed;
        }

        // Denied always takes priority
        for pattern in &self.denied_patterns {
            if glob_match(pattern, tool_name) {
                return ConstraintResult::Denied(format!(
                    "Tool '{}' is denied by pattern '{}'",
                    tool_name, pattern
                ));
            }
        }

        // If there are allowed patterns, the tool must match at least one
        if !self.allowed_patterns.is_empty() {
            for pattern in &self.allowed_patterns {
                if glob_match(pattern, tool_name) {
                    return ConstraintResult::Allowed;
                }
            }
            return ConstraintResult::Denied(format!(
                "Tool '{}' is not in the allowed tools list",
                tool_name
            ));
        }

        // Only denied patterns exist; tool is not denied, so allow
        ConstraintResult::Allowed
    }

    /// Filter a list of tool names, returning only allowed ones.
    pub fn filter_tools(&self, tools: &[String]) -> Vec<String> {
        tools
            .iter()
            .filter(|t| self.check(t) == ConstraintResult::Allowed)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_match_exact() {
        assert!(glob_match("foo", "foo"));
        assert!(!glob_match("foo", "bar"));
    }

    #[test]
    fn test_glob_match_wildcard() {
        assert!(glob_match("*", "anything"));
        assert!(glob_match("mcp:server:*", "mcp:server:tool1"));
        assert!(!glob_match("mcp:server:*", "mcp:other:tool1"));
        assert!(glob_match("*.rs", "main.rs"));
        assert!(!glob_match("*.rs", "main.py"));
    }
}
