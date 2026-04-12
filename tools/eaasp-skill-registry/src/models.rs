use serde::{Deserialize, Serialize};

/// Skill lifecycle status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillStatus {
    Draft,
    Tested,
    Reviewed,
    Production,
}

impl std::fmt::Display for SkillStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default();
        write!(f, "{}", s)
    }
}

/// Metadata for a skill asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMeta {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub status: SkillStatus,
    pub author: Option<String>,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Full skill content including frontmatter and prose.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillContent {
    pub meta: SkillMeta,
    pub frontmatter_yaml: String,
    pub prose: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parsed_v2: Option<crate::skill_parser::V2Frontmatter>,
    /// Absolute path to the skill version directory on the filesystem.
    /// Used by L4 to resolve `${SKILL_DIR}` in hook commands.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill_dir: Option<String>,
}

/// Version entry for a skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillVersion {
    pub version: String,
    pub status: SkillStatus,
    pub created_at: String,
    pub git_commit: Option<String>,
}

/// Request body for submitting a new skill draft.
#[derive(Debug, Clone, Deserialize)]
pub struct SubmitDraftRequest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: Option<String>,
    /// Source directory of the SKILL.md file (optional). When provided,
    /// subdirectories like `hooks/` and `scripts/` are copied alongside
    /// SKILL.md into the registry storage.
    pub source_dir: Option<String>,
    pub tags: Option<Vec<String>>,
    pub frontmatter_yaml: String,
    pub prose: String,
}

/// Request body for promoting a skill to a new status.
#[derive(Debug, Clone, Deserialize)]
pub struct PromoteRequest {
    pub target_status: SkillStatus,
}

/// Query parameters for skill search.
#[derive(Debug, Clone, Deserialize)]
pub struct SearchQuery {
    #[serde(default)]
    pub q: Option<String>,
    #[serde(default)]
    pub tags: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

/// Generic (id, version) invocation payload used by several MCP-style tools.
#[derive(Debug, Clone, Deserialize)]
pub struct InvokeById {
    pub id: String,
    #[serde(default)]
    pub version: Option<String>,
}

/// Payload for the `skill_read` MCP-style tool.
#[derive(Debug, Clone, Deserialize)]
pub struct InvokeSkillRead {
    pub id: String,
    #[serde(default)]
    pub version: Option<String>,
}

/// Payload for the `skill_search` MCP-style tool.
#[derive(Debug, Clone, Deserialize)]
pub struct InvokeSkillSearch {
    #[serde(default)]
    pub q: Option<String>,
    #[serde(default)]
    pub tags: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

/// Payload for the `skill_promote` MCP-style tool.
#[derive(Debug, Clone, Deserialize)]
pub struct InvokePromote {
    pub id: String,
    pub version: String,
    pub target_status: SkillStatus,
}
