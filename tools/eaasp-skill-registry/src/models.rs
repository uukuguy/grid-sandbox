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
    pub q: Option<String>,
    pub tags: Option<String>,
    pub status: Option<String>,
    pub limit: Option<usize>,
}
