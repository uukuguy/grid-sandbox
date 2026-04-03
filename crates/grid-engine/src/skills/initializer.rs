//! Builtin Skills Initializer — seeds embedded skills to `~/.grid/skills/` on startup.
//!
//! All skills under `builtin/skills/` are compiled into the binary via `include_dir!`.
//! On startup, they are extracted to the global skills directory (`~/.grid/skills/`).
//! Existing skill directories are never overwritten — user customizations take priority.

use std::path::Path;

use anyhow::Result;
use include_dir::{include_dir, Dir};
use tracing::{debug, info};

/// All builtin skills embedded at compile time from `builtin/skills/`.
static BUILTIN_SKILLS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/builtin/skills");

/// Sync all builtin skills to the target directory (typically `~/.grid/skills/`).
///
/// For each skill directory in the embedded tree:
/// - If `<target_dir>/<name>/` does NOT exist → create it and extract all files
/// - If it already exists → skip entirely (never overwrite user files)
///
/// Returns the number of skills seeded.
pub fn sync_builtin_skills(target_dir: &Path) -> Result<usize> {
    let mut synced = 0;

    for skill_entry in BUILTIN_SKILLS_DIR.dirs() {
        let name = skill_entry
            .path()
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let skill_dir = target_dir.join(name);

        if skill_dir.exists() {
            debug!(name = name, "Skill directory exists, skipping");
            continue;
        }

        info!(name = name, "Seeding builtin skill");
        extract_dir(skill_entry, &skill_dir)?;
        synced += 1;
    }

    if synced > 0 {
        info!(count = synced, "Builtin skills seeded");
    } else {
        debug!("All builtin skills already present on disk");
    }

    Ok(synced)
}

/// Recursively extract an embedded directory to a filesystem path.
fn extract_dir(dir: &Dir, target: &Path) -> Result<()> {
    std::fs::create_dir_all(target)?;

    // Extract files
    for file in dir.files() {
        let file_name = file
            .path()
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        let file_path = target.join(file_name);
        std::fs::write(&file_path, file.contents())?;

        // Preserve executable permission for scripts
        #[cfg(unix)]
        if file_name.ends_with(".py") || file_name.ends_with(".sh") {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&file_path, std::fs::Permissions::from_mode(0o755))?;
        }
    }

    // Recurse into subdirectories
    for subdir in dir.dirs() {
        let subdir_name = subdir
            .path()
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        extract_dir(subdir, &target.join(subdir_name))?;
    }

    Ok(())
}

/// Get the list of builtin skill names.
pub fn builtin_skill_names() -> Vec<&'static str> {
    BUILTIN_SKILLS_DIR
        .dirs()
        .filter_map(|d| d.path().file_name().and_then(|n| n.to_str()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_skills_count() {
        let names = builtin_skill_names();
        assert!(names.len() >= 10, "Expected at least 10 builtin skills, got {}", names.len());
    }

    #[test]
    fn test_builtin_skill_names_include_key_skills() {
        let names = builtin_skill_names();
        for expected in &["docx", "pdf", "pptx", "xlsx", "filesystem", "web-search"] {
            assert!(names.contains(expected), "Missing builtin skill: {}", expected);
        }
    }

    #[test]
    fn test_sync_to_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let count = sync_builtin_skills(dir.path()).unwrap();
        assert!(count >= 10, "Expected at least 10 skills synced, got {}", count);

        // Verify key skills have SKILL.md
        for name in &["docx", "pdf", "filesystem"] {
            let path = dir.path().join(name).join("SKILL.md");
            assert!(path.exists(), "Missing SKILL.md for: {}", name);
        }
    }

    #[test]
    fn test_sync_preserves_scripts() {
        let dir = tempfile::tempdir().unwrap();
        sync_builtin_skills(dir.path()).unwrap();

        // docx should have scripts/
        let scripts = dir.path().join("docx").join("scripts");
        assert!(scripts.is_dir(), "docx/scripts/ should exist");
    }

    #[test]
    fn test_sync_idempotent() {
        let dir = tempfile::tempdir().unwrap();

        let count1 = sync_builtin_skills(dir.path()).unwrap();
        assert!(count1 >= 10);

        // Second sync — should be no-op
        let count2 = sync_builtin_skills(dir.path()).unwrap();
        assert_eq!(count2, 0);
    }

    #[test]
    fn test_sync_does_not_overwrite_user_files() {
        let dir = tempfile::tempdir().unwrap();
        sync_builtin_skills(dir.path()).unwrap();

        // Modify a file
        let path = dir.path().join("filesystem").join("SKILL.md");
        std::fs::write(&path, "user modified content").unwrap();

        // Second sync — should NOT overwrite
        let count = sync_builtin_skills(dir.path()).unwrap();
        assert_eq!(count, 0);

        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "user modified content");
    }
}
