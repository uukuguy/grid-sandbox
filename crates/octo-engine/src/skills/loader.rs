use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use tracing::{debug, warn};

use octo_types::SkillDefinition;

pub struct SkillLoader {
    search_dirs: Vec<PathBuf>,
}

impl SkillLoader {
    /// Create a loader with project-level and user-level skill directories.
    /// Project-level skills override user-level on name conflict.
    pub fn new(project_dir: Option<&Path>, user_dir: Option<&Path>) -> Self {
        let mut search_dirs = Vec::new();
        // Project-level first (higher priority)
        if let Some(dir) = project_dir {
            let skills_dir = dir.join(".octo").join("skills");
            if skills_dir.is_dir() {
                search_dirs.push(skills_dir);
            }
        }
        // User-level second
        if let Some(dir) = user_dir {
            let skills_dir = dir.join(".octo").join("skills");
            if skills_dir.is_dir() {
                search_dirs.push(skills_dir);
            }
        }
        Self { search_dirs }
    }

    /// Scan all directories and parse SKILL.md files.
    pub fn load_all(&self) -> Result<Vec<SkillDefinition>> {
        let mut skills = Vec::new();
        let mut seen_names = std::collections::HashSet::new();

        for dir in &self.search_dirs {
            let entries = match std::fs::read_dir(dir) {
                Ok(e) => e,
                Err(e) => {
                    debug!(dir = %dir.display(), error = %e, "Cannot read skills directory");
                    continue;
                }
            };

            for entry in entries.flatten() {
                let skill_dir = entry.path();
                if !skill_dir.is_dir() {
                    continue;
                }
                let skill_file = skill_dir.join("SKILL.md");
                if !skill_file.exists() {
                    continue;
                }

                match Self::parse_skill(&skill_file) {
                    Ok(skill) => {
                        if seen_names.contains(&skill.name) {
                            debug!(name = %skill.name, "Skill already loaded (project overrides user)");
                            continue;
                        }
                        debug!(name = %skill.name, path = %skill_file.display(), "Loaded skill");
                        seen_names.insert(skill.name.clone());
                        skills.push(skill);
                    }
                    Err(e) => {
                        warn!(path = %skill_file.display(), error = %e, "Failed to parse SKILL.md");
                    }
                }
            }
        }

        debug!(
            "Loaded {} skills from {} directories",
            skills.len(),
            self.search_dirs.len()
        );
        Ok(skills)
    }

    /// Parse a single SKILL.md file.
    pub fn parse_skill(path: &Path) -> Result<SkillDefinition> {
        let content =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;

        let (frontmatter, body) = Self::split_frontmatter(&content)
            .with_context(|| format!("splitting frontmatter in {}", path.display()))?;

        let mut skill: SkillDefinition = serde_yaml::from_str(&frontmatter)
            .with_context(|| format!("parsing YAML frontmatter in {}", path.display()))?;

        // Validate required fields
        if skill.name.is_empty() {
            bail!(
                "SKILL.md missing required field 'name' in {}",
                path.display()
            );
        }
        if skill.description.is_empty() {
            bail!(
                "SKILL.md missing required field 'description' in {}",
                path.display()
            );
        }

        let base_dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();

        // Substitute template variables
        let base_dir_str = base_dir.to_string_lossy();
        let substituted_body = body.replace("${baseDir}", &base_dir_str);

        skill.body = substituted_body;
        skill.base_dir = base_dir;
        skill.source_path = path.to_path_buf();

        Ok(skill)
    }

    /// Split content into YAML frontmatter and Markdown body.
    /// Frontmatter is delimited by `---` at the start and end.
    fn split_frontmatter(content: &str) -> Result<(String, String)> {
        let trimmed = content.trim_start();
        if !trimmed.starts_with("---") {
            bail!("No YAML frontmatter found (must start with ---)");
        }

        // Find closing ---
        let after_first = &trimmed[3..];
        let end_pos = after_first
            .find("\n---")
            .ok_or_else(|| anyhow::anyhow!("No closing --- for frontmatter"))?;

        let frontmatter = after_first[..end_pos].trim().to_string();
        let body_start = end_pos + 4; // skip \n---
        let body = if body_start < after_first.len() {
            after_first[body_start..]
                .trim_start_matches('\n')
                .to_string()
        } else {
            String::new()
        };

        Ok((frontmatter, body))
    }

    /// Get the search directories (for file watching).
    pub fn search_dirs(&self) -> &[PathBuf] {
        &self.search_dirs
    }
}
