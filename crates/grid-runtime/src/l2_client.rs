//! L2 Skill Registry HTTP client.
//!
//! Fetches skill content from the L2 Skill Registry REST API
//! during session initialization.

use anyhow::Result;
use serde::Deserialize;
use tracing::info;

/// Minimal L2 Skill content for L1 consumption.
#[derive(Debug, Clone, Deserialize)]
pub struct L2SkillContent {
    pub meta: L2SkillMeta,
    pub frontmatter_yaml: String,
    pub prose: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct L2SkillMeta {
    pub id: String,
    pub name: String,
    pub version: String,
}

/// HTTP client for L2 Skill Registry.
pub struct L2SkillClient {
    base_url: String,
    http: reqwest::Client,
}

impl L2SkillClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            http: reqwest::Client::new(),
        }
    }

    /// Fetch skill content by ID from L2 Skill Registry.
    pub async fn fetch_skill(&self, skill_id: &str) -> Result<L2SkillContent> {
        let url = format!("{}/skills/{}/content", self.base_url, skill_id);
        info!(skill_id = %skill_id, url = %url, "Fetching skill from L2");
        let resp = self.http.get(&url).send().await?;
        if !resp.status().is_success() {
            anyhow::bail!(
                "L2 Skill Registry returned {}: {}",
                resp.status(),
                skill_id
            );
        }
        let content: L2SkillContent = resp.json().await?;
        Ok(content)
    }

    /// Fetch multiple skills, returning results per skill.
    pub async fn fetch_skills(
        &self,
        skill_ids: &[String],
    ) -> Vec<(String, Result<L2SkillContent>)> {
        let mut results = Vec::new();
        for id in skill_ids {
            let result = self.fetch_skill(id).await;
            results.push((id.clone(), result));
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l2_skill_content_deserialize() {
        let json = serde_json::json!({
            "meta": {"id": "org/order-mgmt", "name": "Order Management", "version": "1.0.0"},
            "frontmatter_yaml": "name: order-mgmt\n",
            "prose": "# Order Management\nProcess orders..."
        });
        let json = json.to_string();
        let content: L2SkillContent = serde_json::from_str(&json).unwrap();
        assert_eq!(content.meta.id, "org/order-mgmt");
        assert_eq!(content.meta.name, "Order Management");
        assert!(content.prose.contains("Process orders"));
    }

    #[test]
    fn l2_client_base_url_trim() {
        let client = L2SkillClient::new("http://localhost:8081/");
        assert_eq!(client.base_url, "http://localhost:8081");
    }

    #[test]
    fn l2_client_base_url_no_trailing_slash() {
        let client = L2SkillClient::new("http://localhost:8081");
        assert_eq!(client.base_url, "http://localhost:8081");
    }

    #[test]
    fn l2_skill_meta_deserialize() {
        let json = r#"{"id": "acme/billing", "name": "Billing Skill", "version": "2.1.0"}"#;
        let meta: L2SkillMeta = serde_json::from_str(json).unwrap();
        assert_eq!(meta.id, "acme/billing");
        assert_eq!(meta.version, "2.1.0");
    }
}
