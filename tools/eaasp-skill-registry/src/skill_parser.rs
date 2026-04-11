//! V2 skill frontmatter parser.
//!
//! Parses the EAASP v2 skill frontmatter schema, which extends the legacy
//! schema with `runtime_affinity`, `access_scope`, `scoped_hooks`, and
//! `dependencies`. All fields are optional so legacy frontmatter parses
//! cleanly with default values.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct V2Frontmatter {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub runtime_affinity: RuntimeAffinity,
    #[serde(default)]
    pub access_scope: Option<String>,
    #[serde(default)]
    pub scoped_hooks: ScopedHooks,
    #[serde(default)]
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct RuntimeAffinity {
    #[serde(default)]
    pub preferred: Option<String>,
    #[serde(default)]
    pub compatible: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ScopedHooks {
    #[serde(default, rename = "PreToolUse")]
    pub pre_tool_use: Vec<ScopedHook>,
    #[serde(default, rename = "PostToolUse")]
    pub post_tool_use: Vec<ScopedHook>,
    #[serde(default, rename = "Stop")]
    pub stop: Vec<ScopedHook>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScopedHook {
    pub name: String,
    #[serde(flatten)]
    pub body: ScopedHookBody,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ScopedHookBody {
    Command { command: String },
    Prompt { prompt: String },
}

#[derive(Debug, Error)]
pub enum V2ParseError {
    #[error("yaml parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("empty frontmatter")]
    Empty,
}

/// Parse a v2 frontmatter YAML string.
///
/// Returns `V2ParseError::Empty` if the input is blank/whitespace-only, and
/// `V2ParseError::Yaml` for any serde parsing failure. Legacy frontmatter
/// (only `name`/`version`/`author`) parses successfully with default values
/// for the v2-specific fields.
pub fn parse_v2_frontmatter(yaml: &str) -> Result<V2Frontmatter, V2ParseError> {
    if yaml.trim().is_empty() {
        return Err(V2ParseError::Empty);
    }
    let fm: V2Frontmatter = serde_yaml::from_str(yaml)?;
    Ok(fm)
}
