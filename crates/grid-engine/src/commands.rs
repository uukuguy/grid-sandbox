//! Custom slash command loader.
//!
//! Loads `.md` files from `~/.grid/commands/` (global) and `$PWD/.grid/commands/`
//! (project). Each `.md` file becomes a `/filename` slash command whose content
//! is used as a prompt template. `$ARGUMENTS` in the template is replaced with
//! user-provided arguments.
//!
//! Subdirectories create namespaced commands: `commands/foo/bar.md` → `/foo:bar`.
//!
//! Builtin commands are compiled into the binary via `include_dir!` from
//! `builtin/commands/`. They are synced to `~/.grid/commands/` on startup
//! (never overwriting existing files) and loaded with lowest priority.
//!
//! Priority order: project commands > global commands > builtin commands.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use include_dir::{include_dir, Dir};
use tracing::{debug, info, warn};

/// All builtin commands embedded at compile time from `builtin/commands/`.
static BUILTIN_COMMANDS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/builtin/commands");

/// A loaded custom command.
#[derive(Debug, Clone)]
pub struct CustomCommand {
    /// Command name as shown in autocomplete (e.g. "deploy", "review:pr").
    pub name: String,
    /// Short description (first non-empty line of the file, or filename).
    pub description: String,
    /// Full prompt template content.
    pub template: String,
    /// Whether the template contains `$ARGUMENTS`.
    pub has_arguments: bool,
    /// Source file path (for diagnostics).
    pub source_path: PathBuf,
    /// Whether this is a project-level command (vs global).
    pub is_project: bool,
}

impl CustomCommand {
    /// Expand the template, replacing `$ARGUMENTS` with the given args string.
    pub fn expand(&self, arguments: &str) -> String {
        self.template.replace("$ARGUMENTS", arguments)
    }
}

/// Load custom commands from the given directories.
///
/// Directories are processed in order; earlier entries take priority
/// (project before global). Returns commands keyed by name.
pub fn load_commands(dirs: &[PathBuf]) -> Vec<CustomCommand> {
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut commands = Vec::new();

    for (dir_idx, dir) in dirs.iter().enumerate() {
        if !dir.is_dir() {
            continue;
        }
        let is_project = dir_idx == 0;
        load_from_dir(dir, dir, is_project, &mut seen, &mut commands);
    }

    debug!(count = commands.len(), "Loaded custom commands");
    commands
}

/// Sync all builtin commands to the target directory (typically `~/.grid/commands/`).
///
/// For each `.md` file in the embedded tree:
/// - If `<target_dir>/<name>.md` does NOT exist → write it
/// - If it already exists → skip (never overwrite user customizations)
///
/// Returns the number of commands seeded.
pub fn sync_builtin_commands(target_dir: &Path) -> Result<usize> {
    std::fs::create_dir_all(target_dir)?;
    let mut synced = 0;

    for file in BUILTIN_COMMANDS_DIR.files() {
        let file_name = match file.path().file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };
        let target_path = target_dir.join(file_name);
        if target_path.exists() {
            debug!(name = file_name, "Builtin command exists, skipping");
            continue;
        }

        info!(name = file_name, "Seeding builtin command");
        std::fs::write(&target_path, file.contents())?;
        synced += 1;
    }

    if synced > 0 {
        info!(count = synced, "Builtin commands seeded");
    } else {
        debug!("All builtin commands already present on disk");
    }

    Ok(synced)
}

/// Get the list of builtin command names (without `.md` extension).
pub fn builtin_command_names() -> Vec<&'static str> {
    BUILTIN_COMMANDS_DIR
        .files()
        .filter_map(|f| {
            let name = f.path().file_stem().and_then(|n| n.to_str())?;
            // Only include .md files
            if f.path().extension().and_then(|e| e.to_str()) == Some("md") {
                Some(name)
            } else {
                None
            }
        })
        .collect()
}

/// Recursively load `.md` files from a directory tree.
fn load_from_dir(
    base: &Path,
    dir: &Path,
    is_project: bool,
    seen: &mut HashMap<String, usize>,
    commands: &mut Vec<CustomCommand>,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            debug!(dir = %dir.display(), error = %e, "Cannot read commands directory");
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            load_from_dir(base, &path, is_project, seen, commands);
            continue;
        }

        // Only process .md files
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }

        let name = derive_command_name(base, &path);
        if name.is_empty() {
            continue;
        }

        // Project commands (earlier dirs) take priority
        if seen.contains_key(&name) {
            debug!(name, path = %path.display(), "Skipping duplicate command");
            continue;
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => {
                let description = extract_description(&content, &name);
                let has_arguments = content.contains("$ARGUMENTS");
                let idx = commands.len();
                seen.insert(name.clone(), idx);
                commands.push(CustomCommand {
                    name,
                    description,
                    template: content,
                    has_arguments,
                    source_path: path,
                    is_project,
                });
            }
            Err(e) => {
                warn!(path = %path.display(), error = %e, "Failed to read command file");
            }
        }
    }
}

/// Derive a command name from the file path relative to the base directory.
///
/// `commands/deploy.md` → `"deploy"`
/// `commands/review/pr.md` → `"review:pr"`
fn derive_command_name(base: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(base).unwrap_or(path);
    let stem = relative.with_extension("");
    let parts: Vec<&str> = stem
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();
    parts.join(":")
}

/// Extract a short description from the command file content.
///
/// Uses the first non-empty, non-heading line. Falls back to the command name.
fn extract_description(content: &str, fallback_name: &str) -> String {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("---") {
            continue;
        }
        // Truncate long descriptions
        if trimmed.len() > 80 {
            return format!("{}...", &trimmed[..77]);
        }
        return trimmed.to_string();
    }
    format!("Custom command: {}", fallback_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_derive_command_name_simple() {
        let base = Path::new("/home/user/.grid/commands");
        let path = Path::new("/home/user/.grid/commands/deploy.md");
        assert_eq!(derive_command_name(base, path), "deploy");
    }

    #[test]
    fn test_derive_command_name_nested() {
        let base = Path::new("/home/user/.grid/commands");
        let path = Path::new("/home/user/.grid/commands/review/pr.md");
        assert_eq!(derive_command_name(base, path), "review:pr");
    }

    #[test]
    fn test_extract_description_first_line() {
        let content = "# Title\n\nDeploy the application to production.\n\nMore details...";
        assert_eq!(
            extract_description(content, "deploy"),
            "Deploy the application to production."
        );
    }

    #[test]
    fn test_extract_description_fallback() {
        let content = "# Just a heading\n\n";
        assert_eq!(
            extract_description(content, "deploy"),
            "Custom command: deploy"
        );
    }

    #[test]
    fn test_custom_command_expand() {
        let cmd = CustomCommand {
            name: "test".into(),
            description: "A test".into(),
            template: "Do $ARGUMENTS for me".into(),
            has_arguments: true,
            source_path: PathBuf::from("test.md"),
            is_project: false,
        };
        assert_eq!(cmd.expand("something cool"), "Do something cool for me");
    }

    #[test]
    fn test_custom_command_expand_no_placeholder() {
        let cmd = CustomCommand {
            name: "test".into(),
            description: "A test".into(),
            template: "Always do this".into(),
            has_arguments: false,
            source_path: PathBuf::from("test.md"),
            is_project: false,
        };
        assert_eq!(cmd.expand("ignored"), "Always do this");
    }

    #[test]
    fn test_load_commands_from_dirs() {
        let tmp = tempdir().unwrap();

        // Create project commands dir
        let project_dir = tmp.path().join("project");
        std::fs::create_dir_all(&project_dir).unwrap();
        std::fs::write(
            project_dir.join("deploy.md"),
            "Deploy the app.\n\nDeploy $ARGUMENTS to production.",
        )
        .unwrap();

        // Create global commands dir
        let global_dir = tmp.path().join("global");
        std::fs::create_dir_all(&global_dir).unwrap();
        std::fs::write(
            global_dir.join("deploy.md"),
            "Global deploy (should be shadowed).",
        )
        .unwrap();
        std::fs::write(global_dir.join("greet.md"), "Say hello to $ARGUMENTS.").unwrap();

        let commands = load_commands(&[project_dir, global_dir]);
        assert_eq!(commands.len(), 2); // deploy (project) + greet (global)

        let deploy = commands.iter().find(|c| c.name == "deploy").unwrap();
        assert!(deploy.is_project);
        assert!(deploy.has_arguments);
        assert!(deploy.template.contains("production"));

        let greet = commands.iter().find(|c| c.name == "greet").unwrap();
        assert!(!greet.is_project);
        assert!(greet.has_arguments);
    }

    #[test]
    fn test_load_commands_nested() {
        let tmp = tempdir().unwrap();
        let dir = tmp.path().join("commands");
        let sub = dir.join("review");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("pr.md"), "Review this PR.").unwrap();

        let commands = load_commands(&[dir]);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].name, "review:pr");
    }

    #[test]
    fn test_load_commands_empty_dir() {
        let tmp = tempdir().unwrap();
        let commands = load_commands(&[tmp.path().to_path_buf()]);
        assert!(commands.is_empty());
    }

    #[test]
    fn test_load_commands_nonexistent_dir() {
        let commands = load_commands(&[PathBuf::from("/nonexistent/path")]);
        assert!(commands.is_empty());
    }

    #[test]
    fn test_builtin_command_names() {
        let names = builtin_command_names();
        assert!(names.len() >= 10, "Expected at least 10 builtin commands, got {}", names.len());
        for expected in &["review", "test", "fix", "refactor", "doc", "commit", "security", "plan", "audit", "bootstrap"] {
            assert!(names.contains(expected), "Missing builtin command: {}", expected);
        }
    }

    #[test]
    fn test_sync_builtin_commands_to_empty_dir() {
        let dir = tempdir().unwrap();
        let count = sync_builtin_commands(dir.path()).unwrap();
        assert!(count >= 10, "Expected at least 10 commands synced, got {}", count);

        // Verify key commands exist
        for name in &["review.md", "security.md", "test.md"] {
            let path = dir.path().join(name);
            assert!(path.exists(), "Missing builtin command: {}", name);
        }
    }

    #[test]
    fn test_sync_builtin_commands_idempotent() {
        let dir = tempdir().unwrap();
        let count1 = sync_builtin_commands(dir.path()).unwrap();
        assert!(count1 >= 10);

        let count2 = sync_builtin_commands(dir.path()).unwrap();
        assert_eq!(count2, 0, "Second sync should be no-op");
    }

    #[test]
    fn test_sync_builtin_commands_preserves_user_files() {
        let dir = tempdir().unwrap();
        sync_builtin_commands(dir.path()).unwrap();

        // User modifies a command
        let path = dir.path().join("review.md");
        std::fs::write(&path, "my custom review prompt").unwrap();

        // Re-sync should NOT overwrite
        sync_builtin_commands(dir.path()).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "my custom review prompt");
    }

    #[test]
    fn test_builtin_commands_loaded_via_load_commands() {
        let dir = tempdir().unwrap();
        sync_builtin_commands(dir.path()).unwrap();

        let commands = load_commands(&[dir.path().to_path_buf()]);
        assert!(commands.len() >= 10);

        let review = commands.iter().find(|c| c.name == "review").unwrap();
        assert!(review.has_arguments);
        assert!(review.template.contains("$ARGUMENTS"));
    }

    #[test]
    fn test_user_commands_override_builtin() {
        let dir = tempdir().unwrap();

        // Sync builtins
        let global_dir = dir.path().join("global");
        sync_builtin_commands(&global_dir).unwrap();

        // Create project command with same name
        let project_dir = dir.path().join("project");
        std::fs::create_dir_all(&project_dir).unwrap();
        std::fs::write(project_dir.join("review.md"), "My custom review: $ARGUMENTS").unwrap();

        let commands = load_commands(&[project_dir, global_dir]);

        // Only one "review" command — the project one
        let reviews: Vec<_> = commands.iter().filter(|c| c.name == "review").collect();
        assert_eq!(reviews.len(), 1);
        assert!(reviews[0].is_project);
        assert!(reviews[0].template.contains("My custom review"));
    }

    #[test]
    fn test_ignores_non_md_files() {
        let tmp = tempdir().unwrap();
        let dir = tmp.path().join("commands");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("readme.txt"), "Not a command").unwrap();
        std::fs::write(dir.join("deploy.md"), "Deploy it.").unwrap();

        let commands = load_commands(&[dir]);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].name, "deploy");
    }
}
