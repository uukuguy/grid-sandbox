//! Permission rule parsing and matching.
//!
//! Rules follow the `ToolName(pattern)` syntax from Claude Code OSS:
//! - `bash(git *)` — match all git commands
//! - `file_edit(src/**/*.rs)` — match Rust file edits under src/
//! - `file_read` — match all file reads (no parens = match all)
//! - `*` — match all tools

use serde::{Deserialize, Serialize};

use super::permission_types::RuleSource;

/// A single permission rule: tool name + optional glob pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRule {
    /// Tool name ("bash", "file_edit", "*" for all)
    pub tool_name: String,
    /// Glob pattern for matching tool input (None = match all calls to this tool)
    pub pattern: Option<String>,
}

impl PermissionRule {
    /// Parse from string: "bash(git *)" → tool_name="bash", pattern=Some("git *")
    pub fn parse(s: &str) -> anyhow::Result<Self> {
        let s = s.trim();
        if s.is_empty() {
            anyhow::bail!("Empty permission rule");
        }
        if let Some(paren_start) = s.find('(') {
            if s.ends_with(')') {
                let tool_name = s[..paren_start].trim().to_string();
                let pattern = s[paren_start + 1..s.len() - 1].trim().to_string();
                if tool_name.is_empty() {
                    anyhow::bail!("Empty tool name in rule: {s}");
                }
                return Ok(Self {
                    tool_name,
                    pattern: if pattern.is_empty() {
                        None
                    } else {
                        Some(pattern)
                    },
                });
            }
        }
        // No parens: match all calls to this tool
        Ok(Self {
            tool_name: s.to_string(),
            pattern: None,
        })
    }

    /// Check if this rule matches the given tool call.
    pub fn matches(&self, tool_name: &str, input: &serde_json::Value) -> bool {
        // Tool name match (supports "*" wildcard)
        if self.tool_name != "*" && self.tool_name != tool_name {
            return false;
        }
        // No pattern = match all calls to this tool
        let pattern = match &self.pattern {
            None => return true,
            Some(p) => p,
        };
        // Extract the match target from tool input
        let target = match extract_match_target(tool_name, input) {
            Some(t) => t,
            None => return false,
        };
        // Glob-style matching
        glob_match(pattern, &target)
    }

    /// Human-readable description of this rule
    pub fn description(&self) -> String {
        match &self.pattern {
            Some(p) => format!("{}({})", self.tool_name, p),
            None => self.tool_name.clone(),
        }
    }
}

impl std::fmt::Display for PermissionRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

/// Per-tool parameter extraction for matching.
fn extract_match_target(tool_name: &str, input: &serde_json::Value) -> Option<String> {
    match tool_name {
        "bash" => input
            .get("command")
            .and_then(|v| v.as_str())
            .map(String::from),
        "file_read" | "file_write" | "file_edit" => input
            .get("file_path")
            .and_then(|v| v.as_str())
            .map(String::from),
        "grep" | "glob" | "find" => input
            .get("path")
            .and_then(|v| v.as_str())
            .map(String::from),
        "web_fetch" => input
            .get("url")
            .and_then(|v| v.as_str())
            .map(String::from),
        "web_search" => input
            .get("query")
            .and_then(|v| v.as_str())
            .map(String::from),
        _ => {
            // Default: serialize entire input as match target
            Some(input.to_string())
        }
    }
}

/// Simple glob matching supporting `*` (any non-`/` chars) and `**` (any chars including `/`).
///
/// This is a lightweight implementation suitable for permission rules.
/// For file paths, the `glob` crate is used elsewhere; this handles
/// arbitrary string patterns like "git *" or "rm -rf *".
fn glob_match(pattern: &str, text: &str) -> bool {
    // Fast path: exact match
    if pattern == text {
        return true;
    }
    // Fast path: match-all
    if pattern == "*" || pattern == "**" {
        return true;
    }

    let pat: Vec<char> = pattern.chars().collect();
    let txt: Vec<char> = text.chars().collect();
    glob_match_recursive(&pat, 0, &txt, 0)
}

fn glob_match_recursive(pat: &[char], pi: usize, txt: &[char], ti: usize) -> bool {
    let (plen, tlen) = (pat.len(), txt.len());

    if pi == plen {
        return ti == tlen;
    }

    // Check for "**" (matches anything including '/')
    if pi + 1 < plen && pat[pi] == '*' && pat[pi + 1] == '*' {
        // Try matching rest of pattern at every position in text
        let next_pi = pi + 2;
        // Skip optional '/' after **
        let next_pi = if next_pi < plen && pat[next_pi] == '/' {
            next_pi + 1
        } else {
            next_pi
        };
        for i in ti..=tlen {
            if glob_match_recursive(pat, next_pi, txt, i) {
                return true;
            }
        }
        return false;
    }

    // Single '*' — matches any chars except '/'
    if pat[pi] == '*' {
        let next_pi = pi + 1;
        for i in ti..=tlen {
            if i > ti && ti < tlen && txt[i - 1] == '/' {
                // '*' doesn't cross '/' for file path patterns
                // But for command patterns like "git *", we want to match across spaces
                // Use heuristic: if pattern contains '/', treat as path glob
                if pattern_is_path_like(pat) {
                    break;
                }
            }
            if glob_match_recursive(pat, next_pi, txt, i) {
                return true;
            }
        }
        return false;
    }

    // '?' matches any single char
    if pat[pi] == '?' {
        if ti < tlen {
            return glob_match_recursive(pat, pi + 1, txt, ti + 1);
        }
        return false;
    }

    // Literal character match
    if ti < tlen && pat[pi] == txt[ti] {
        return glob_match_recursive(pat, pi + 1, txt, ti + 1);
    }

    false
}

/// Heuristic: does the pattern look like a file path?
fn pattern_is_path_like(pat: &[char]) -> bool {
    pat.iter().any(|c| *c == '/')
}

/// A set of rules from a single source (e.g., project-level, user-level).
#[derive(Debug, Clone)]
pub struct PermissionRuleSet {
    pub source: RuleSource,
    pub allow_rules: Vec<PermissionRule>,
    pub deny_rules: Vec<PermissionRule>,
    pub ask_rules: Vec<PermissionRule>,
}

impl PermissionRuleSet {
    pub fn new(source: RuleSource) -> Self {
        Self {
            source,
            allow_rules: Vec::new(),
            deny_rules: Vec::new(),
            ask_rules: Vec::new(),
        }
    }
}

/// YAML file format for security_rules.yaml
#[derive(Debug, Deserialize)]
pub struct RulesFileFormat {
    pub rules: RulesSection,
}

#[derive(Debug, Deserialize)]
pub struct RulesSection {
    #[serde(default)]
    pub allow: Vec<String>,
    #[serde(default)]
    pub deny: Vec<String>,
    #[serde(default)]
    pub ask: Vec<String>,
}

/// Load rules from a YAML file.
pub fn load_rules_from_yaml(
    path: &std::path::Path,
    source: RuleSource,
) -> anyhow::Result<PermissionRuleSet> {
    let content = std::fs::read_to_string(path)?;
    let file: RulesFileFormat = serde_yaml::from_str(&content)?;

    let mut rule_set = PermissionRuleSet::new(source);

    for s in &file.rules.allow {
        rule_set.allow_rules.push(PermissionRule::parse(s)?);
    }
    for s in &file.rules.deny {
        rule_set.deny_rules.push(PermissionRule::parse(s)?);
    }
    for s in &file.rules.ask {
        rule_set.ask_rules.push(PermissionRule::parse(s)?);
    }

    Ok(rule_set)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_with_pattern() {
        let rule = PermissionRule::parse("bash(git *)").unwrap();
        assert_eq!(rule.tool_name, "bash");
        assert_eq!(rule.pattern.as_deref(), Some("git *"));
    }

    #[test]
    fn test_parse_without_pattern() {
        let rule = PermissionRule::parse("file_read").unwrap();
        assert_eq!(rule.tool_name, "file_read");
        assert!(rule.pattern.is_none());
    }

    #[test]
    fn test_parse_wildcard() {
        let rule = PermissionRule::parse("*").unwrap();
        assert_eq!(rule.tool_name, "*");
        assert!(rule.pattern.is_none());
    }

    #[test]
    fn test_parse_empty_fails() {
        assert!(PermissionRule::parse("").is_err());
    }

    #[test]
    fn test_matches_bash_git() {
        let rule = PermissionRule::parse("bash(git *)").unwrap();
        assert!(rule.matches("bash", &json!({"command": "git status"})));
        assert!(rule.matches("bash", &json!({"command": "git push --force"})));
        assert!(!rule.matches("bash", &json!({"command": "cargo test"})));
        assert!(!rule.matches("file_read", &json!({"file_path": "/etc/passwd"})));
    }

    #[test]
    fn test_matches_file_path_glob() {
        let rule = PermissionRule::parse("file_edit(src/**/*.rs)").unwrap();
        assert!(rule.matches("file_edit", &json!({"file_path": "src/main.rs"})));
        assert!(rule.matches("file_edit", &json!({"file_path": "src/tools/bash.rs"})));
        assert!(!rule.matches("file_edit", &json!({"file_path": "tests/test.rs"})));
    }

    #[test]
    fn test_matches_all_tools() {
        let rule = PermissionRule::parse("*").unwrap();
        assert!(rule.matches("bash", &json!({"command": "ls"})));
        assert!(rule.matches("file_read", &json!({"file_path": "/etc/hosts"})));
    }

    #[test]
    fn test_matches_tool_without_pattern() {
        let rule = PermissionRule::parse("file_read").unwrap();
        assert!(rule.matches("file_read", &json!({"file_path": "/anything"})));
        assert!(!rule.matches("bash", &json!({"command": "ls"})));
    }

    #[test]
    fn test_matches_rm_rf() {
        let rule = PermissionRule::parse("bash(rm -rf *)").unwrap();
        assert!(rule.matches("bash", &json!({"command": "rm -rf /tmp/test"})));
        assert!(rule.matches("bash", &json!({"command": "rm -rf /"})));
        assert!(!rule.matches("bash", &json!({"command": "rm file.txt"})));
    }

    #[test]
    fn test_glob_match_exact() {
        assert!(glob_match("hello", "hello"));
        assert!(!glob_match("hello", "world"));
    }

    #[test]
    fn test_glob_match_star() {
        assert!(glob_match("git *", "git status"));
        assert!(glob_match("git *", "git push --force"));
        assert!(!glob_match("git *", "cargo test"));
    }

    #[test]
    fn test_glob_match_doublestar() {
        assert!(glob_match("src/**/*.rs", "src/main.rs"));
        assert!(glob_match("src/**/*.rs", "src/tools/bash.rs"));
        assert!(!glob_match("src/**/*.rs", "tests/test.rs"));
    }

    #[test]
    fn test_glob_match_question() {
        assert!(glob_match("?.rs", "a.rs"));
        assert!(!glob_match("?.rs", "ab.rs"));
    }

    #[test]
    fn test_description() {
        let rule = PermissionRule::parse("bash(git *)").unwrap();
        assert_eq!(rule.description(), "bash(git *)");

        let rule = PermissionRule::parse("file_read").unwrap();
        assert_eq!(rule.description(), "file_read");
    }
}
