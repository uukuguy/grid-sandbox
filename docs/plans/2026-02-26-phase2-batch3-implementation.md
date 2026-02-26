# Phase 2 Batch 3 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add Skill Loader, MCP Client, Tool Execution recording, REST APIs, and minimal Debug UI to the octo-sandbox agent platform.

**Architecture:** Three independent feature chains (Skill → MCP → Debug) converge in a final integration task. Backend is Rust (Axum + SQLite + rmcp), frontend is React 19 + Jotai + Tailwind CSS 4. All new code follows existing patterns in the codebase.

**Tech Stack:** Rust (serde_yaml, notify, rmcp), TypeScript/React 19, Jotai, Tailwind CSS 4, SQLite

**Design Doc:** `docs/plans/2026-02-26-phase2-batch3-design.md`

---

### Task 1: Workspace dependencies + ToolSource enhancement

**Files:**
- Modify: `Cargo.toml` (workspace deps)
- Modify: `crates/octo-types/Cargo.toml`
- Modify: `crates/octo-engine/Cargo.toml`
- Modify: `crates/octo-types/src/tool.rs`

**Step 1: Add workspace dependencies**

In `Cargo.toml`, add to `[workspace.dependencies]`:

```toml
# YAML parsing
serde_yaml = "0.9"
# File system watcher
notify = "7"
notify-debouncer-mini = "0.5"
# MCP Protocol SDK
rmcp = { version = "0.16", features = ["client", "transport-child-process"] }
```

In `crates/octo-types/Cargo.toml`, add:

```toml
serde_yaml = { workspace = true }
```

In `crates/octo-engine/Cargo.toml`, add:

```toml
serde_yaml = { workspace = true }
notify = { workspace = true }
notify-debouncer-mini = { workspace = true }
rmcp = { workspace = true }
```

**Step 2: Enhance ToolSource enum**

In `crates/octo-types/src/tool.rs`, change `ToolSource` to carry server/skill names:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolSource {
    BuiltIn,
    Mcp(String),    // MCP server name
    Skill(String),  // Skill name
    Plugin,
}
```

**Step 3: Build to verify**

Run: `cargo check --workspace`
Expected: Compilation succeeds. Existing `source()` impls on all tools return `ToolSource::BuiltIn` so they still compile. Dead code warnings are fine.

**Step 4: Commit**

```bash
git add Cargo.toml crates/octo-types/Cargo.toml crates/octo-engine/Cargo.toml crates/octo-types/src/tool.rs
git commit -m "feat(deps): add serde_yaml, notify, rmcp workspace deps + ToolSource(String)"
```

---

### Task 2: SkillDefinition type + SKILL.md parser

**Files:**
- Create: `crates/octo-types/src/skill.rs`
- Modify: `crates/octo-types/src/lib.rs`
- Create: `crates/octo-engine/src/skills/mod.rs`
- Create: `crates/octo-engine/src/skills/loader.rs`
- Modify: `crates/octo-engine/src/lib.rs`

**Step 1: Create SkillDefinition type**

Create `crates/octo-types/src/skill.rs`:

```rust
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Skill definition parsed from a SKILL.md file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default, rename = "user-invocable")]
    pub user_invocable: bool,
    #[serde(default, rename = "allowed-tools")]
    pub allowed_tools: Option<Vec<String>>,
    /// Markdown body (template variables already substituted).
    #[serde(skip)]
    pub body: String,
    /// Directory containing SKILL.md.
    #[serde(skip)]
    pub base_dir: PathBuf,
    /// Full path to the SKILL.md file.
    #[serde(skip)]
    pub source_path: PathBuf,
}
```

**Step 2: Register in octo-types/lib.rs**

Add to `crates/octo-types/src/lib.rs`:

```rust
pub mod skill;
```

And in the re-exports:

```rust
pub use skill::*;
```

**Step 3: Create SkillLoader**

Create `crates/octo-engine/src/skills/mod.rs`:

```rust
pub mod loader;
```

Create `crates/octo-engine/src/skills/loader.rs`:

```rust
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
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

        debug!("Loaded {} skills from {} directories", skills.len(), self.search_dirs.len());
        Ok(skills)
    }

    /// Parse a single SKILL.md file.
    pub fn parse_skill(path: &Path) -> Result<SkillDefinition> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("reading {}", path.display()))?;

        let (frontmatter, body) = Self::split_frontmatter(&content)
            .with_context(|| format!("splitting frontmatter in {}", path.display()))?;

        let mut skill: SkillDefinition = serde_yaml::from_str(&frontmatter)
            .with_context(|| format!("parsing YAML frontmatter in {}", path.display()))?;

        // Validate required fields
        if skill.name.is_empty() {
            bail!("SKILL.md missing required field 'name' in {}", path.display());
        }
        if skill.description.is_empty() {
            bail!("SKILL.md missing required field 'description' in {}", path.display());
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
            after_first[body_start..].trim_start_matches('\n').to_string()
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
```

**Step 4: Register skills module in octo-engine**

Add to `crates/octo-engine/src/lib.rs`:

```rust
pub mod skills;
```

**Step 5: Build to verify**

Run: `cargo check --workspace`
Expected: Compilation succeeds.

**Step 6: Commit**

```bash
git add crates/octo-types/src/skill.rs crates/octo-types/src/lib.rs crates/octo-engine/src/skills/ crates/octo-engine/src/lib.rs
git commit -m "feat(skills): SkillDefinition type + SKILL.md parser with frontmatter splitting"
```

---

### Task 3: SkillRegistry + SkillTool + SystemPromptBuilder integration

**Files:**
- Create: `crates/octo-engine/src/skills/registry.rs`
- Create: `crates/octo-engine/src/skills/tool.rs`
- Modify: `crates/octo-engine/src/skills/mod.rs`
- Modify: `crates/octo-engine/src/context/builder.rs`
- Modify: `crates/octo-engine/src/lib.rs`

**Step 1: Create SkillRegistry**

Create `crates/octo-engine/src/skills/registry.rs`:

```rust
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use anyhow::Result;
use tracing::{debug, info, warn};

use octo_types::SkillDefinition;

use super::loader::SkillLoader;

/// Thread-safe registry of loaded Skills.
pub struct SkillRegistry {
    skills: Arc<RwLock<HashMap<String, SkillDefinition>>>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            skills: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Load all skills from the given SkillLoader.
    pub fn load_from(&self, loader: &SkillLoader) -> Result<()> {
        let loaded = loader.load_all()?;
        let mut skills = self.skills.write().unwrap();
        skills.clear();
        for skill in loaded {
            skills.insert(skill.name.clone(), skill);
        }
        info!("SkillRegistry loaded {} skills", skills.len());
        Ok(())
    }

    /// Reload all skills (for hot-reload).
    pub fn reload(&self, loader: &SkillLoader) -> Result<()> {
        let loaded = loader.load_all()?;
        let mut skills = self.skills.write().unwrap();
        let old_count = skills.len();
        skills.clear();
        for skill in loaded {
            skills.insert(skill.name.clone(), skill);
        }
        info!("SkillRegistry reloaded: {} → {} skills", old_count, skills.len());
        Ok(())
    }

    /// Generate system prompt section listing available skills.
    pub fn prompt_section(&self) -> String {
        let skills = self.skills.read().unwrap();
        if skills.is_empty() {
            return String::new();
        }

        let mut section = String::from("<available_skills>\n");
        let mut sorted: Vec<_> = skills.values().collect();
        sorted.sort_by_key(|s| &s.name);

        for skill in sorted {
            let version = skill
                .version
                .as_deref()
                .map(|v| format!(" (v{v})"))
                .unwrap_or_default();
            section.push_str(&format!("## {}{}\n", skill.name, version));
            section.push_str(&skill.description);
            if !skill.description.ends_with('\n') {
                section.push('\n');
            }
            if skill.user_invocable {
                section.push_str(&format!("Use: /{}\n", skill.name));
            }
            section.push('\n');
        }
        section.push_str("</available_skills>");
        section
    }

    /// Get all user-invocable skills (for registering as tools).
    pub fn invocable_skills(&self) -> Vec<SkillDefinition> {
        let skills = self.skills.read().unwrap();
        skills
            .values()
            .filter(|s| s.user_invocable)
            .cloned()
            .collect()
    }

    /// Get a skill by name.
    pub fn get(&self, name: &str) -> Option<SkillDefinition> {
        let skills = self.skills.read().unwrap();
        skills.get(name).cloned()
    }

    /// Number of loaded skills.
    pub fn len(&self) -> usize {
        self.skills.read().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get inner Arc for sharing across threads.
    pub fn inner(&self) -> Arc<RwLock<HashMap<String, SkillDefinition>>> {
        self.skills.clone()
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 2: Create SkillTool**

Create `crates/octo-engine/src/skills/tool.rs`:

```rust
use anyhow::Result;
use async_trait::async_trait;

use octo_types::{SkillDefinition, ToolContext, ToolResult, ToolSource, ToolSpec};

use crate::tools::Tool;

/// Wraps a user-invocable Skill as a callable Tool.
pub struct SkillTool {
    skill: SkillDefinition,
}

impl SkillTool {
    pub fn new(skill: SkillDefinition) -> Self {
        Self { skill }
    }
}

#[async_trait]
impl Tool for SkillTool {
    fn name(&self) -> &str {
        &self.skill.name
    }

    fn description(&self) -> &str {
        &self.skill.description
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "args": {
                    "type": "string",
                    "description": "Optional arguments for the skill"
                }
            },
            "required": []
        })
    }

    fn source(&self) -> ToolSource {
        ToolSource::Skill(self.skill.name.clone())
    }

    async fn execute(
        &self,
        _params: serde_json::Value,
        _ctx: &ToolContext,
    ) -> Result<ToolResult> {
        // Return the skill's instructions as the tool result.
        // The Agent will follow these instructions.
        Ok(ToolResult::success(&self.skill.body))
    }
}
```

**Step 3: Update skills/mod.rs**

```rust
pub mod loader;
pub mod registry;
pub mod tool;

pub use loader::SkillLoader;
pub use registry::SkillRegistry;
pub use tool::SkillTool;
```

**Step 4: Integrate with SystemPromptBuilder**

In `crates/octo-engine/src/context/builder.rs`, add a method to `SystemPromptBuilder`:

```rust
    /// Add skills section to system prompt.
    pub fn with_skills(mut self, skills_section: String) -> Self {
        if !skills_section.is_empty() {
            self.extra_parts.push(skills_section);
        }
        self
    }
```

**Step 5: Update re-exports in lib.rs**

In `crates/octo-engine/src/lib.rs`, add to the existing re-exports:

```rust
pub use skills::{SkillLoader, SkillRegistry, SkillTool};
```

**Step 6: Build to verify**

Run: `cargo check --workspace`
Expected: Compilation succeeds.

**Step 7: Commit**

```bash
git add crates/octo-engine/src/skills/ crates/octo-engine/src/context/builder.rs crates/octo-engine/src/lib.rs
git commit -m "feat(skills): SkillRegistry + SkillTool + SystemPromptBuilder integration"
```

---

### Task 4: Skill hot-reload with notify

**Files:**
- Modify: `crates/octo-engine/src/skills/registry.rs`
- Modify: `crates/octo-engine/src/skills/mod.rs`

**Step 1: Add hot-reload to SkillRegistry**

Add the following to `crates/octo-engine/src/skills/registry.rs` (new `start_watching` method):

```rust
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
use std::time::Duration;

impl SkillRegistry {
    /// Start watching skill directories for changes.
    /// Reloads all skills when any SKILL.md file changes.
    pub fn start_watching(&self, loader: SkillLoader) -> Result<()> {
        let dirs = loader.search_dirs().to_vec();
        if dirs.is_empty() {
            debug!("No skill directories to watch");
            return Ok(());
        }

        let skills = self.skills.clone();

        std::thread::spawn(move || {
            let (tx, rx) = std::sync::mpsc::channel();
            let mut debouncer = match new_debouncer(Duration::from_millis(300), tx) {
                Ok(d) => d,
                Err(e) => {
                    warn!("Failed to create file watcher: {e}");
                    return;
                }
            };

            for dir in &dirs {
                if let Err(e) = debouncer
                    .watcher()
                    .watch(dir, notify::RecursiveMode::Recursive)
                {
                    warn!(dir = %dir.display(), error = %e, "Failed to watch directory");
                }
            }

            info!("Skill hot-reload watcher started for {} directories", dirs.len());

            for events in rx {
                match events {
                    Ok(events) => {
                        let has_skill_change = events.iter().any(|e| {
                            e.kind == DebouncedEventKind::Any
                                && e.path
                                    .file_name()
                                    .map(|f| f == "SKILL.md")
                                    .unwrap_or(false)
                        });

                        if has_skill_change {
                            info!("SKILL.md changed, reloading skills");
                            match loader.load_all() {
                                Ok(loaded) => {
                                    let mut map = skills.write().unwrap();
                                    map.clear();
                                    for skill in loaded {
                                        map.insert(skill.name.clone(), skill);
                                    }
                                    info!("Skills reloaded: {} skills", map.len());
                                }
                                Err(e) => {
                                    warn!("Failed to reload skills: {e}");
                                }
                            }
                        }
                    }
                    Err(errs) => {
                        for e in errs {
                            warn!("Watch error: {e}");
                        }
                    }
                }
            }
        });

        Ok(())
    }
}
```

Add required imports at top of file:

```rust
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
use std::time::Duration;
```

**Step 2: Build to verify**

Run: `cargo check --workspace`
Expected: Compilation succeeds.

**Step 3: Commit**

```bash
git add crates/octo-engine/src/skills/
git commit -m "feat(skills): hot-reload with notify watcher (300ms debounce)"
```

---

### Task 5: McpClient trait + StdioMcpClient (rmcp wrapper)

**Files:**
- Create: `crates/octo-engine/src/mcp/mod.rs`
- Create: `crates/octo-engine/src/mcp/traits.rs`
- Create: `crates/octo-engine/src/mcp/stdio.rs`
- Modify: `crates/octo-engine/src/lib.rs`

**Step 1: Create MCP module entry**

Create `crates/octo-engine/src/mcp/mod.rs`:

```rust
pub mod traits;
pub mod stdio;
pub mod bridge;
pub mod manager;

pub use traits::{McpClient, McpServerConfig, McpToolInfo};
pub use stdio::StdioMcpClient;
pub use bridge::McpToolBridge;
pub use manager::McpManager;
```

**Step 2: Create McpClient trait + types**

Create `crates/octo-engine/src/mcp/traits.rs`:

```rust
use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Info about a tool provided by an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolInfo {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}

/// Configuration for an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// Abstraction over MCP protocol client.
#[async_trait]
pub trait McpClient: Send + Sync {
    /// Server name.
    fn name(&self) -> &str;

    /// Connect to the MCP server (spawn process + handshake).
    async fn connect(&mut self) -> Result<()>;

    /// List tools provided by the server.
    async fn list_tools(&self) -> Result<Vec<McpToolInfo>>;

    /// Call a tool on the server.
    async fn call_tool(
        &self,
        name: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value>;

    /// Check if connected.
    fn is_connected(&self) -> bool;

    /// Graceful shutdown.
    async fn shutdown(&mut self) -> Result<()>;
}
```

**Step 3: Create StdioMcpClient**

Create `crates/octo-engine/src/mcp/stdio.rs`:

```rust
use anyhow::{Result, Context, bail};
use async_trait::async_trait;
use tracing::{debug, info, warn};

use rmcp::model::CallToolRequestParams;
use rmcp::service::RunningService;
use rmcp::transport::{ConfigureCommandExt, TokioChildProcess};
use rmcp::{RoleClient, ServiceExt};

use super::traits::{McpClient, McpServerConfig, McpToolInfo};

pub struct StdioMcpClient {
    config: McpServerConfig,
    service: Option<RunningService<RoleClient, ()>>,
}

impl StdioMcpClient {
    pub fn new(config: McpServerConfig) -> Self {
        Self {
            config,
            service: None,
        }
    }
}

#[async_trait]
impl McpClient for StdioMcpClient {
    fn name(&self) -> &str {
        &self.config.name
    }

    async fn connect(&mut self) -> Result<()> {
        let config = &self.config;
        info!(
            name = %config.name,
            command = %config.command,
            "Connecting to MCP server"
        );

        let mut cmd = tokio::process::Command::new(&config.command);
        let env = config.env.clone();
        let args = config.args.clone();

        let transport = TokioChildProcess::new(cmd.configure(move |c| {
            for arg in &args {
                c.arg(arg);
            }
            for (k, v) in &env {
                c.env(k, v);
            }
        }))
        .context("Failed to spawn MCP server process")?;

        let service = ().serve(transport).await
            .context("Failed to initialize MCP connection")?;

        let peer_info = service.peer_info();
        info!(
            name = %config.name,
            server = ?peer_info,
            "MCP server connected"
        );

        self.service = Some(service);
        Ok(())
    }

    async fn list_tools(&self) -> Result<Vec<McpToolInfo>> {
        let service = self
            .service
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("MCP client not connected"))?;

        let tools = service.list_all_tools().await
            .context("Failed to list MCP tools")?;

        let result: Vec<McpToolInfo> = tools
            .into_iter()
            .map(|t| McpToolInfo {
                name: t.name.to_string(),
                description: t.description.map(|d| d.to_string()),
                input_schema: serde_json::to_value(&t.input_schema)
                    .unwrap_or(serde_json::json!({"type": "object"})),
            })
            .collect();

        debug!(count = result.len(), "Listed MCP tools");
        Ok(result)
    }

    async fn call_tool(
        &self,
        name: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let service = self
            .service
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("MCP client not connected"))?;

        let arguments = if args.is_object() {
            Some(
                args.as_object()
                    .unwrap()
                    .iter()
                    .map(|(k, v)| (k.clone().into(), v.clone()))
                    .collect(),
            )
        } else {
            None
        };

        let result = service
            .call_tool(CallToolRequestParams {
                meta: None,
                name: name.into(),
                arguments,
                task: None,
            })
            .await
            .with_context(|| format!("Failed to call MCP tool '{name}'"))?;

        // Convert result content to JSON
        let content_strs: Vec<String> = result
            .content
            .into_iter()
            .filter_map(|c| {
                match c {
                    rmcp::model::Content::Text(text) => Some(text.text.to_string()),
                    _ => None,
                }
            })
            .collect();

        Ok(serde_json::json!({
            "content": content_strs.join("\n"),
            "isError": result.is_error.unwrap_or(false),
        }))
    }

    fn is_connected(&self) -> bool {
        self.service.is_some()
    }

    async fn shutdown(&mut self) -> Result<()> {
        if let Some(service) = self.service.take() {
            info!(name = %self.config.name, "Shutting down MCP server");
            service.cancel().await
                .context("Failed to cancel MCP service")?;
        }
        Ok(())
    }
}
```

**Step 4: Register mcp module in lib.rs**

Add to `crates/octo-engine/src/lib.rs`:

```rust
pub mod mcp;
```

And to re-exports:

```rust
pub use mcp::{McpClient, McpManager, McpServerConfig, McpToolBridge, McpToolInfo, StdioMcpClient};
```

Note: `bridge.rs` and `manager.rs` don't exist yet. Create empty placeholder files to satisfy the module declaration:

Create `crates/octo-engine/src/mcp/bridge.rs`:
```rust
// McpToolBridge — implemented in Task 6
```

Create `crates/octo-engine/src/mcp/manager.rs`:
```rust
// McpManager — implemented in Task 6
```

**Step 5: Build to verify**

Run: `cargo check --workspace`
Expected: Compilation succeeds (bridge/manager are empty but declared).

**Step 6: Commit**

```bash
git add crates/octo-engine/src/mcp/ crates/octo-engine/src/lib.rs
git commit -m "feat(mcp): McpClient trait + StdioMcpClient (rmcp wrapper)"
```

---

### Task 6: McpToolBridge + McpManager

**Files:**
- Modify: `crates/octo-engine/src/mcp/bridge.rs`
- Modify: `crates/octo-engine/src/mcp/manager.rs`

**Step 1: Implement McpToolBridge**

Replace `crates/octo-engine/src/mcp/bridge.rs`:

```rust
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::RwLock;

use octo_types::{ToolContext, ToolResult, ToolSource, ToolSpec};

use crate::tools::Tool;
use super::traits::{McpClient, McpToolInfo};

/// Bridges an MCP server tool into the local ToolRegistry.
pub struct McpToolBridge {
    client: Arc<RwLock<Box<dyn McpClient>>>,
    server_name: String,
    tool_info: McpToolInfo,
}

impl McpToolBridge {
    pub fn new(
        client: Arc<RwLock<Box<dyn McpClient>>>,
        server_name: String,
        tool_info: McpToolInfo,
    ) -> Self {
        Self {
            client,
            server_name,
            tool_info,
        }
    }
}

#[async_trait]
impl Tool for McpToolBridge {
    fn name(&self) -> &str {
        &self.tool_info.name
    }

    fn description(&self) -> &str {
        self.tool_info
            .description
            .as_deref()
            .unwrap_or("")
    }

    fn parameters(&self) -> serde_json::Value {
        self.tool_info.input_schema.clone()
    }

    fn source(&self) -> ToolSource {
        ToolSource::Mcp(self.server_name.clone())
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: &ToolContext,
    ) -> Result<ToolResult> {
        let client = self.client.read().await;
        match client.call_tool(&self.tool_info.name, params).await {
            Ok(result) => {
                let is_error = result
                    .get("isError")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let content = result
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if is_error {
                    Ok(ToolResult::error(content))
                } else {
                    Ok(ToolResult::success(content))
                }
            }
            Err(e) => Ok(ToolResult::error(format!("MCP tool error: {e}"))),
        }
    }
}
```

**Step 2: Implement McpManager**

Replace `crates/octo-engine/src/mcp/manager.rs`:

```rust
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::tools::ToolRegistry;

use super::bridge::McpToolBridge;
use super::stdio::StdioMcpClient;
use super::traits::{McpClient, McpServerConfig, McpToolInfo};

/// MCP config file format (.octo/mcp.json).
#[derive(Debug, serde::Deserialize)]
struct McpConfigFile {
    servers: HashMap<String, McpServerEntry>,
}

#[derive(Debug, serde::Deserialize)]
struct McpServerEntry {
    command: String,
    args: Vec<String>,
    #[serde(default)]
    env: HashMap<String, String>,
}

/// Manages multiple MCP server connections.
pub struct McpManager {
    clients: HashMap<String, Arc<RwLock<Box<dyn McpClient>>>>,
    tool_infos: HashMap<String, Vec<McpToolInfo>>,
}

impl McpManager {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
            tool_infos: HashMap::new(),
        }
    }

    /// Load MCP server configs from a JSON file.
    pub fn load_config(config_path: &Path) -> Result<Vec<McpServerConfig>> {
        let content = std::fs::read_to_string(config_path)
            .with_context(|| format!("reading {}", config_path.display()))?;
        let config: McpConfigFile = serde_json::from_str(&content)
            .with_context(|| format!("parsing {}", config_path.display()))?;

        Ok(config
            .servers
            .into_iter()
            .map(|(name, entry)| McpServerConfig {
                name,
                command: entry.command,
                args: entry.args,
                env: entry.env,
            })
            .collect())
    }

    /// Add and connect a new MCP server.
    pub async fn add_server(&mut self, config: McpServerConfig) -> Result<Vec<McpToolInfo>> {
        let name = config.name.clone();
        let mut client = StdioMcpClient::new(config);
        client.connect().await?;

        let tools = client.list_tools().await?;
        info!(
            server = %name,
            tool_count = tools.len(),
            "MCP server connected with tools"
        );

        let client: Arc<RwLock<Box<dyn McpClient>>> =
            Arc::new(RwLock::new(Box::new(client)));
        self.clients.insert(name.clone(), client);
        self.tool_infos.insert(name, tools.clone());
        Ok(tools)
    }

    /// Remove and shutdown an MCP server.
    pub async fn remove_server(&mut self, name: &str) -> Result<()> {
        if let Some(client) = self.clients.remove(name) {
            let mut client = client.write().await;
            client.shutdown().await?;
        }
        self.tool_infos.remove(name);
        info!(server = %name, "MCP server removed");
        Ok(())
    }

    /// Bridge all MCP tools into a ToolRegistry.
    pub fn bridge_tools(&self, registry: &mut ToolRegistry) {
        for (server_name, tools) in &self.tool_infos {
            let client = self.clients.get(server_name).unwrap().clone();
            for tool_info in tools {
                let bridge = McpToolBridge::new(
                    client.clone(),
                    server_name.clone(),
                    tool_info.clone(),
                );
                registry.register(bridge);
                debug!(
                    server = %server_name,
                    tool = %tool_info.name,
                    "Bridged MCP tool"
                );
            }
        }
    }

    /// Shutdown all MCP servers.
    pub async fn shutdown_all(&mut self) -> Result<()> {
        let names: Vec<String> = self.clients.keys().cloned().collect();
        for name in names {
            if let Some(client) = self.clients.remove(&name) {
                let mut c = client.write().await;
                if let Err(e) = c.shutdown().await {
                    warn!(server = %name, error = %e, "Error shutting down MCP server");
                }
            }
        }
        self.tool_infos.clear();
        Ok(())
    }

    /// Get number of connected servers.
    pub fn server_count(&self) -> usize {
        self.clients.len()
    }
}

impl Default for McpManager {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 3: Build to verify**

Run: `cargo check --workspace`
Expected: Compilation succeeds.

**Step 4: Commit**

```bash
git add crates/octo-engine/src/mcp/
git commit -m "feat(mcp): McpToolBridge + McpManager (multi-server, config file)"
```

---

### Task 7: ToolExecution types + SQLite schema

**Files:**
- Create: `crates/octo-types/src/execution.rs`
- Modify: `crates/octo-types/src/lib.rs`
- Modify: `crates/octo-engine/src/db/migrations.rs`

**Step 1: Create ToolExecution types**

Create `crates/octo-types/src/execution.rs`:

```rust
use serde::{Deserialize, Serialize};

use crate::ToolSource;

/// Status of a tool execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Running,
    Success,
    Failed,
    Timeout,
}

/// Record of a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecution {
    pub id: String,
    pub session_id: String,
    pub tool_name: String,
    pub source: ToolSource,
    pub input: serde_json::Value,
    pub output: Option<serde_json::Value>,
    pub status: ExecutionStatus,
    pub started_at: i64,
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
}

/// Snapshot of the token budget state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBudgetSnapshot {
    pub total: usize,
    pub system_prompt: usize,
    pub dynamic_context: usize,
    pub history: usize,
    pub free: usize,
    pub usage_percent: f32,
    pub degradation_level: u8,
}
```

**Step 2: Register in lib.rs**

In `crates/octo-types/src/lib.rs`, add:

```rust
pub mod execution;
```

And:

```rust
pub use execution::*;
```

**Step 3: Add SQLite migration**

In `crates/octo-engine/src/db/migrations.rs`, bump `CURRENT_VERSION` to 2 and add:

```rust
const CURRENT_VERSION: u32 = 2;
```

Add the new migration SQL:

```rust
const MIGRATION_V2: &str = "
-- Tool execution records
CREATE TABLE IF NOT EXISTS tool_executions (
    id          TEXT PRIMARY KEY,
    session_id  TEXT NOT NULL,
    tool_name   TEXT NOT NULL,
    source      TEXT NOT NULL,
    input       TEXT NOT NULL,
    output      TEXT,
    status      TEXT NOT NULL DEFAULT 'running',
    started_at  INTEGER NOT NULL,
    duration_ms INTEGER,
    error       TEXT,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_tool_executions_session
    ON tool_executions(session_id);
CREATE INDEX IF NOT EXISTS idx_tool_executions_tool
    ON tool_executions(tool_name);
CREATE INDEX IF NOT EXISTS idx_tool_executions_started
    ON tool_executions(started_at DESC);
";
```

Update the `migrate` function:

```rust
pub fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    let version: u32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

    if version < 1 {
        info!(from = version, to = 1, "Running database migration v1");
        conn.execute_batch(MIGRATION_V1)?;
    }
    if version < 2 {
        info!(from = version.max(1), to = 2, "Running database migration v2");
        conn.execute_batch(MIGRATION_V2)?;
    }
    if version < CURRENT_VERSION {
        conn.pragma_update(None, "user_version", CURRENT_VERSION)?;
        info!("Migration to v{CURRENT_VERSION} complete");
    }

    Ok(())
}
```

**Step 4: Build to verify**

Run: `cargo check --workspace`
Expected: Compilation succeeds.

**Step 5: Commit**

```bash
git add crates/octo-types/src/execution.rs crates/octo-types/src/lib.rs crates/octo-engine/src/db/migrations.rs
git commit -m "feat(types+db): ToolExecution types + SQLite migration v2 (tool_executions table)"
```

---

### Task 8: ToolExecutionRecorder + AgentLoop integration

**Files:**
- Create: `crates/octo-engine/src/tools/recorder.rs`
- Modify: `crates/octo-engine/src/tools/mod.rs`
- Modify: `crates/octo-engine/src/agent/loop_.rs`
- Modify: `crates/octo-engine/src/lib.rs`

**Step 1: Create ToolExecutionRecorder**

Create `crates/octo-engine/src/tools/recorder.rs`:

```rust
use anyhow::Result;
use tracing::debug;

use octo_types::{ExecutionStatus, ToolExecution, ToolSource};

use crate::db::Database;

pub struct ToolExecutionRecorder {
    db: Database,
}

impl ToolExecutionRecorder {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Record tool execution start. Returns the execution ID.
    pub async fn record_start(
        &self,
        session_id: &str,
        tool_name: &str,
        source: &ToolSource,
        input: &serde_json::Value,
    ) -> Result<String> {
        let id = ulid::Ulid::new().to_string();
        let source_str = serde_json::to_string(source)?;
        let input_str = serde_json::to_string(input)?;
        let started_at = chrono::Utc::now().timestamp_millis();

        let id_clone = id.clone();
        let session_id = session_id.to_string();
        let tool_name = tool_name.to_string();

        self.db
            .conn()
            .call(move |conn| {
                conn.execute(
                    "INSERT INTO tool_executions (id, session_id, tool_name, source, input, status, started_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, 'running', ?6)",
                    rusqlite::params![id_clone, session_id, tool_name, source_str, input_str, started_at],
                )?;
                Ok(())
            })
            .await?;

        debug!(id = %id, tool = %tool_name, "Recorded tool execution start");
        Ok(id)
    }

    /// Record successful tool execution completion.
    pub async fn record_complete(
        &self,
        id: &str,
        output: &serde_json::Value,
        duration_ms: u64,
    ) -> Result<()> {
        let output_str = serde_json::to_string(output)?;
        let id = id.to_string();

        self.db
            .conn()
            .call(move |conn| {
                conn.execute(
                    "UPDATE tool_executions SET output = ?1, status = 'success', duration_ms = ?2 WHERE id = ?3",
                    rusqlite::params![output_str, duration_ms as i64, id],
                )?;
                Ok(())
            })
            .await?;

        Ok(())
    }

    /// Record failed tool execution.
    pub async fn record_failed(
        &self,
        id: &str,
        error: &str,
        duration_ms: u64,
    ) -> Result<()> {
        let id = id.to_string();
        let error = error.to_string();

        self.db
            .conn()
            .call(move |conn| {
                conn.execute(
                    "UPDATE tool_executions SET error = ?1, status = 'failed', duration_ms = ?2 WHERE id = ?3",
                    rusqlite::params![error, duration_ms as i64, id],
                )?;
                Ok(())
            })
            .await?;

        Ok(())
    }

    /// List executions for a session.
    pub async fn list_by_session(
        &self,
        session_id: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<ToolExecution>> {
        let session_id = session_id.to_string();
        self.db
            .conn()
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, session_id, tool_name, source, input, output, status, started_at, duration_ms, error
                     FROM tool_executions WHERE session_id = ?1
                     ORDER BY started_at DESC LIMIT ?2 OFFSET ?3",
                )?;
                let rows = stmt
                    .query_map(rusqlite::params![session_id, limit as i64, offset as i64], |row| {
                        Ok(Self::row_to_execution(row))
                    })?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                Ok(rows)
            })
            .await
            .map_err(Into::into)
    }

    /// Get a single execution by ID.
    pub async fn get(&self, id: &str) -> Result<Option<ToolExecution>> {
        let id = id.to_string();
        self.db
            .conn()
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, session_id, tool_name, source, input, output, status, started_at, duration_ms, error
                     FROM tool_executions WHERE id = ?1",
                )?;
                let result = stmt
                    .query_row(rusqlite::params![id], |row| Ok(Self::row_to_execution(row)))
                    .ok();
                Ok(result)
            })
            .await
            .map_err(Into::into)
    }

    fn row_to_execution(row: &rusqlite::Row<'_>) -> ToolExecution {
        let source_str: String = row.get(3).unwrap_or_default();
        let source: ToolSource =
            serde_json::from_str(&source_str).unwrap_or(ToolSource::BuiltIn);
        let input_str: String = row.get(4).unwrap_or_default();
        let output_str: Option<String> = row.get(5).unwrap_or(None);
        let status_str: String = row.get(6).unwrap_or_default();

        ToolExecution {
            id: row.get(0).unwrap_or_default(),
            session_id: row.get(1).unwrap_or_default(),
            tool_name: row.get(2).unwrap_or_default(),
            source,
            input: serde_json::from_str(&input_str).unwrap_or_default(),
            output: output_str.and_then(|s| serde_json::from_str(&s).ok()),
            status: match status_str.as_str() {
                "running" => ExecutionStatus::Running,
                "success" => ExecutionStatus::Success,
                "failed" => ExecutionStatus::Failed,
                "timeout" => ExecutionStatus::Timeout,
                _ => ExecutionStatus::Failed,
            },
            started_at: row.get(7).unwrap_or(0),
            duration_ms: row.get::<_, Option<i64>>(8).unwrap_or(None).map(|v| v as u64),
            error: row.get(9).unwrap_or(None),
        }
    }
}
```

**Step 2: Register in tools/mod.rs**

Add to `crates/octo-engine/src/tools/mod.rs`:

```rust
pub mod recorder;
```

**Step 3: Integrate into AgentLoop**

In `crates/octo-engine/src/agent/loop_.rs`:

1. Add field to `AgentLoop` struct:
```rust
    recorder: Option<Arc<crate::tools::recorder::ToolExecutionRecorder>>,
```

2. Initialize in `new()`:
```rust
            recorder: None,
```

3. Add builder method:
```rust
    pub fn with_recorder(mut self, recorder: Arc<crate::tools::recorder::ToolExecutionRecorder>) -> Self {
        self.recorder = Some(recorder);
        self
    }
```

4. In the tool execution loop (around line 335-368), wrap tool execution with recording:

Before the `let result = ...` line, add:
```rust
                let exec_id = if let Some(ref recorder) = self.recorder {
                    let source = self.tools.get(&tu.name)
                        .map(|t| t.source())
                        .unwrap_or(ToolSource::BuiltIn);
                    recorder.record_start(
                        session_id.as_str(),
                        &tu.name,
                        &source,
                        &input,
                    ).await.ok()
                } else {
                    None
                };

                let exec_start = std::time::Instant::now();
```

After the `let result = ...` block and before `let _ = tx.send(AgentEvent::ToolResult`, add:
```rust
                let exec_duration = exec_start.elapsed().as_millis() as u64;
                if let (Some(ref recorder), Some(ref eid)) = (&self.recorder, &exec_id) {
                    if result.is_error {
                        let _ = recorder.record_failed(eid, &result.output, exec_duration).await;
                    } else {
                        let output_val = serde_json::Value::String(result.output.clone());
                        let _ = recorder.record_complete(eid, &output_val, exec_duration).await;
                    }
                }
```

5. Add imports at top of file:
```rust
use octo_types::ToolSource;
use std::sync::Arc;
```

**Step 4: Update re-exports**

In `crates/octo-engine/src/lib.rs`, add:
```rust
pub use tools::recorder::ToolExecutionRecorder;
```

**Step 5: Build to verify**

Run: `cargo check --workspace`
Expected: Compilation succeeds.

**Step 6: Commit**

```bash
git add crates/octo-engine/src/tools/recorder.rs crates/octo-engine/src/tools/mod.rs crates/octo-engine/src/agent/loop_.rs crates/octo-engine/src/lib.rs
git commit -m "feat(tools): ToolExecutionRecorder + AgentLoop integration (SQLite recording)"
```

---

### Task 9: REST API endpoints

**Files:**
- Create: `crates/octo-server/src/api/mod.rs`
- Create: `crates/octo-server/src/api/sessions.rs`
- Create: `crates/octo-server/src/api/executions.rs`
- Create: `crates/octo-server/src/api/tools.rs`
- Create: `crates/octo-server/src/api/memories.rs`
- Create: `crates/octo-server/src/api/budget.rs`
- Modify: `crates/octo-server/src/router.rs`
- Modify: `crates/octo-server/src/state.rs`
- Modify: `crates/octo-server/src/main.rs`

**Step 1: Create API module**

Create `crates/octo-server/src/api/mod.rs`:

```rust
pub mod budget;
pub mod executions;
pub mod memories;
pub mod sessions;
pub mod tools;

use std::sync::Arc;

use axum::{routing::get, Router};
use crate::state::AppState;

/// Pagination query params.
#[derive(Debug, serde::Deserialize)]
pub struct PaginationParams {
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

fn default_limit() -> usize {
    50
}

impl PaginationParams {
    pub fn clamped(&self) -> (usize, usize) {
        (self.limit.min(200).max(1), self.offset)
    }
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/sessions", get(sessions::list_sessions))
        .route("/sessions/{id}", get(sessions::get_session))
        .route("/sessions/{id}/executions", get(executions::list_session_executions))
        .route("/executions/{id}", get(executions::get_execution))
        .route("/tools", get(tools::list_tools))
        .route("/memories", get(memories::search_memories))
        .route("/memories/working", get(memories::get_working_memory))
        .route("/budget", get(budget::get_budget))
}
```

**Step 2: Create sessions endpoint**

Create `crates/octo-server/src/api/sessions.rs`:

```rust
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Serialize;

use crate::state::AppState;
use super::PaginationParams;

#[derive(Serialize)]
pub struct SessionSummary {
    pub id: String,
    pub created_at: i64,
    pub message_count: usize,
}

pub async fn list_sessions(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationParams>,
) -> Json<Vec<SessionSummary>> {
    let (limit, offset) = params.clamped();
    let sessions = state.sessions.list_sessions(limit, offset).await;
    Json(sessions.into_iter().map(|s| SessionSummary {
        id: s.session_id.as_str().to_string(),
        created_at: 0, // TODO: add created_at to SessionData
        message_count: 0,
    }).collect())
}

pub async fn get_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let session_id = octo_types::SessionId::from_string(&id);
    let messages = state.sessions.get_messages(&session_id).await;
    Json(serde_json::json!({
        "id": id,
        "messages": messages.unwrap_or_default(),
    }))
}
```

**Step 3: Create executions endpoint**

Create `crates/octo-server/src/api/executions.rs`:

```rust
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::Json;

use octo_types::ToolExecution;

use crate::state::AppState;
use super::PaginationParams;

pub async fn list_session_executions(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    Query(params): Query<PaginationParams>,
) -> Json<Vec<ToolExecution>> {
    let (limit, offset) = params.clamped();
    match &state.recorder {
        Some(recorder) => {
            let execs = recorder.list_by_session(&session_id, limit, offset).await.unwrap_or_default();
            Json(execs)
        }
        None => Json(vec![]),
    }
}

pub async fn get_execution(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    match &state.recorder {
        Some(recorder) => {
            let exec = recorder.get(&id).await.ok().flatten();
            Json(serde_json::to_value(exec).unwrap_or_default())
        }
        None => Json(serde_json::json!(null)),
    }
}
```

**Step 4: Create tools endpoint**

Create `crates/octo-server/src/api/tools.rs`:

```rust
use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub source: octo_types::ToolSource,
}

pub async fn list_tools(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<ToolInfo>> {
    let specs = state.tools.specs();
    let tools: Vec<ToolInfo> = state.tools.names().into_iter().zip(specs).map(|(name, spec)| {
        let source = state.tools.get(&name)
            .map(|t| t.source())
            .unwrap_or(octo_types::ToolSource::BuiltIn);
        ToolInfo {
            name: spec.name,
            description: spec.description,
            source,
        }
    }).collect();
    Json(tools)
}
```

**Step 5: Create memories endpoint**

Create `crates/octo-server/src/api/memories.rs`:

```rust
use std::sync::Arc;

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;

use octo_types::{SandboxId, UserId};

use crate::state::AppState;

#[derive(Deserialize)]
pub struct MemorySearchParams {
    pub q: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize { 20 }

pub async fn search_memories(
    State(state): State<Arc<AppState>>,
    Query(params): Query<MemorySearchParams>,
) -> Json<serde_json::Value> {
    let query = params.q.unwrap_or_default();
    if query.is_empty() {
        return Json(serde_json::json!([]));
    }

    let options = octo_types::SearchOptions {
        query,
        user_id: "default".to_string(),
        limit: params.limit.min(100),
        category: None,
        min_importance: None,
    };

    match state.memory_store.search(options, None).await {
        Ok(entries) => Json(serde_json::to_value(entries).unwrap_or_default()),
        Err(_) => Json(serde_json::json!([])),
    }
}

pub async fn get_working_memory(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let user_id = UserId::from_string("default");
    let sandbox_id = SandboxId::new();
    match state.memory.get_blocks(&user_id, &sandbox_id).await {
        Ok(blocks) => Json(serde_json::to_value(blocks).unwrap_or_default()),
        Err(_) => Json(serde_json::json!([])),
    }
}
```

**Step 6: Create budget endpoint**

Create `crates/octo-server/src/api/budget.rs`:

```rust
use std::sync::Arc;

use axum::extract::State;
use axum::Json;

use octo_types::TokenBudgetSnapshot;
use crate::state::AppState;

pub async fn get_budget(
    State(_state): State<Arc<AppState>>,
) -> Json<TokenBudgetSnapshot> {
    // Return a placeholder budget snapshot.
    // Real-time budget is sent via WebSocket; REST is for polling.
    Json(TokenBudgetSnapshot {
        total: 200_000,
        system_prompt: 0,
        dynamic_context: 0,
        history: 0,
        free: 200_000,
        usage_percent: 0.0,
        degradation_level: 0,
    })
}
```

**Step 7: Update AppState**

In `crates/octo-server/src/state.rs`, add the recorder field:

```rust
use std::sync::Arc;

use octo_engine::{MemoryStore, Provider, SessionStore, SkillRegistry, ToolExecutionRecorder, ToolRegistry, WorkingMemory};

pub struct AppState {
    pub provider: Arc<dyn Provider>,
    pub tools: Arc<ToolRegistry>,
    pub memory: Arc<dyn WorkingMemory>,
    pub sessions: Arc<dyn SessionStore>,
    pub memory_store: Arc<dyn MemoryStore>,
    pub model: Option<String>,
    pub recorder: Option<Arc<ToolExecutionRecorder>>,
    pub skill_registry: Arc<SkillRegistry>,
}

impl AppState {
    pub fn new(
        provider: Arc<dyn Provider>,
        tools: Arc<ToolRegistry>,
        memory: Arc<dyn WorkingMemory>,
        sessions: Arc<dyn SessionStore>,
        memory_store: Arc<dyn MemoryStore>,
        model: Option<String>,
        recorder: Option<Arc<ToolExecutionRecorder>>,
        skill_registry: Arc<SkillRegistry>,
    ) -> Self {
        Self {
            provider, tools, memory, sessions, memory_store, model, recorder, skill_registry,
        }
    }
}
```

**Step 8: Update router.rs**

In `crates/octo-server/src/router.rs`, add the API routes:

```rust
mod api;  // Add at top of file or in main.rs mod declarations
```

Actually, since `api` is under `crates/octo-server/src/`, declare it in the server's module structure. Add to the top of `crates/octo-server/src/main.rs`:

```rust
mod api;
```

Update `crates/octo-server/src/router.rs`:

```rust
use std::sync::Arc;

use axum::{routing::get, Router};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::api;
use crate::state::AppState;
use crate::ws::ws_handler;

async fn health() -> &'static str {
    "ok"
}

pub fn build_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/api/health", get(health))
        .nest("/api", api::routes())
        .route("/ws", get(ws_handler))
        .with_state(state)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}
```

**Step 9: Update main.rs with new initialization**

In `crates/octo-server/src/main.rs`, add recorder + skill initialization. After the `tools` setup and before `AppState::new`, add:

```rust
    // Tool execution recorder
    let recorder = Arc::new(octo_engine::ToolExecutionRecorder::new(
        Database::open(&db_path).await?,
    ));

    // Skill system
    let project_dir = std::env::current_dir().ok();
    let user_dir = dirs::home_dir();
    let skill_loader = octo_engine::SkillLoader::new(
        project_dir.as_deref(),
        user_dir.as_deref(),
    );
    let skill_registry = Arc::new(octo_engine::SkillRegistry::new());
    if let Err(e) = skill_registry.load_from(&skill_loader) {
        tracing::warn!("Failed to load skills: {e}");
    }
    // Register user-invocable skills as tools
    for skill in skill_registry.invocable_skills() {
        tools.register(octo_engine::SkillTool::new(skill));
    }
    // Start hot-reload watcher
    if let Err(e) = skill_registry.start_watching(skill_loader) {
        tracing::warn!("Failed to start skill watcher: {e}");
    }
```

Update `AppState::new` call to include new fields:

```rust
    let state = Arc::new(AppState::new(
        provider,
        tools,
        memory,
        sessions,
        memory_store,
        model,
        Some(recorder.clone()),
        skill_registry,
    ));
```

Also add `use crate::api;` near the module declarations and `mod api;` at the top.

Note: Add `dirs = "5"` to octo-server's Cargo.toml for `dirs::home_dir()`, or use `std::env::var("HOME")` as fallback. Let's use `std::env::var("HOME")`:

```rust
    let user_dir = std::env::var("HOME").ok().map(std::path::PathBuf::from);
```

**Step 10: Update ws.rs to pass recorder to AgentLoop**

In `crates/octo-server/src/ws.rs`, where `AgentLoop::new` is called, add:

```rust
                let mut agent_loop = octo_engine::AgentLoop::new(provider, tools, memory)
                    .with_memory_store(state.memory_store.clone());
                if let Some(ref recorder) = state.recorder {
                    agent_loop = agent_loop.with_recorder(recorder.clone());
                }
```

**Step 11: Build to verify**

Run: `cargo check --workspace`
Expected: Compilation succeeds.

**Step 12: Commit**

```bash
git add crates/octo-server/src/api/ crates/octo-server/src/router.rs crates/octo-server/src/state.rs crates/octo-server/src/main.rs crates/octo-server/src/ws.rs
git commit -m "feat(server): REST API endpoints (sessions/executions/tools/memories/budget) + AppState integration"
```

---

### Task 10: WebSocket new events (tool_execution + token_budget_update)

**Files:**
- Modify: `crates/octo-engine/src/agent/loop_.rs`
- Modify: `crates/octo-server/src/ws.rs`

**Step 1: Add new AgentEvent variants**

In `crates/octo-engine/src/agent/loop_.rs`, add to `AgentEvent` enum:

```rust
    ToolExecution {
        execution: octo_types::ToolExecution,
    },
    TokenBudgetUpdate {
        budget: octo_types::TokenBudgetSnapshot,
    },
```

**Step 2: Emit ToolExecution events from AgentLoop**

After the recorder `record_complete`/`record_failed` calls in the tool loop, emit:

```rust
                if let Some(ref eid) = exec_id {
                    let exec = octo_types::ToolExecution {
                        id: eid.clone(),
                        session_id: session_id.as_str().to_string(),
                        tool_name: tu.name.clone(),
                        source: self.tools.get(&tu.name)
                            .map(|t| t.source())
                            .unwrap_or(ToolSource::BuiltIn),
                        input: input.clone(),
                        output: Some(serde_json::Value::String(result.output.clone())),
                        status: if result.is_error {
                            octo_types::ExecutionStatus::Failed
                        } else {
                            octo_types::ExecutionStatus::Success
                        },
                        started_at: chrono::Utc::now().timestamp_millis(),
                        duration_ms: Some(exec_duration),
                        error: if result.is_error { Some(result.output.clone()) } else { None },
                    };
                    let _ = tx.send(AgentEvent::ToolExecution { execution: exec });
                }
```

**Step 3: Emit TokenBudgetUpdate after each round**

After `self.budget.update_actual_usage(...)` in the MessageStop handler, add:

```rust
                        let budget_snapshot = octo_types::TokenBudgetSnapshot {
                            total: 200_000,
                            system_prompt: (system_prompt.len() / 4),
                            dynamic_context: 0,
                            history: (msg_chars_estimate / 4),
                            free: 200_000_usize.saturating_sub(system_prompt.len() / 4 + msg_chars_estimate / 4),
                            usage_percent: self.budget.current_usage_percent(),
                            degradation_level: level as u8,
                        };
                        let _ = tx.send(AgentEvent::TokenBudgetUpdate { budget: budget_snapshot });
```

Where `msg_chars_estimate` is the rough chars count of messages (you can use `messages.iter().map(|m| m.content.iter().map(...).sum()).sum()`). A simpler approach: compute from the budget manager:

```rust
                        let snapshot = self.budget.snapshot(&system_prompt, messages, &tool_specs);
                        let _ = tx.send(AgentEvent::TokenBudgetUpdate { budget: snapshot });
```

Add a `snapshot` method to `ContextBudgetManager`. In `crates/octo-engine/src/context/budget.rs`:

```rust
    pub fn snapshot(
        &self,
        system_prompt: &str,
        messages: &[octo_types::ChatMessage],
        tools: &[octo_types::ToolSpec],
    ) -> octo_types::TokenBudgetSnapshot {
        let sys_tokens = system_prompt.len() / 4;
        let history_tokens = crate::context::builder::estimate_messages_tokens(messages, tools) as usize;
        let total = self.context_window;
        let used = sys_tokens + history_tokens;
        let free = total.saturating_sub(used);
        let usage_pct = if total > 0 { (used as f32 / total as f32) * 100.0 } else { 0.0 };
        let level = self.compute_degradation_level(system_prompt, messages, tools);

        octo_types::TokenBudgetSnapshot {
            total,
            system_prompt: sys_tokens,
            dynamic_context: 0,
            history: history_tokens,
            free,
            usage_percent: usage_pct,
            degradation_level: level as u8,
        }
    }
```

**Step 4: Handle new events in ws.rs**

In `crates/octo-server/src/ws.rs`, add match arms for the new events:

```rust
                                AgentEvent::ToolExecution { execution } => {
                                    ServerMessage::ToolExecutionEvent {
                                        session_id: sid_str.clone(),
                                        execution,
                                    }
                                }
                                AgentEvent::TokenBudgetUpdate { budget } => {
                                    ServerMessage::TokenBudgetUpdate {
                                        session_id: sid_str.clone(),
                                        budget,
                                    }
                                }
```

Add `ServerMessage` variants:

```rust
    #[serde(rename = "tool_execution")]
    ToolExecutionEvent {
        session_id: String,
        execution: octo_types::ToolExecution,
    },

    #[serde(rename = "token_budget_update")]
    TokenBudgetUpdate {
        session_id: String,
        budget: octo_types::TokenBudgetSnapshot,
    },
```

**Step 5: Build to verify**

Run: `cargo check --workspace`
Expected: Compilation succeeds.

**Step 6: Commit**

```bash
git add crates/octo-engine/src/agent/loop_.rs crates/octo-engine/src/context/budget.rs crates/octo-server/src/ws.rs
git commit -m "feat(ws): tool_execution + token_budget_update WebSocket events"
```

---

### Task 11: Frontend - debug atoms + WebSocket event handling

**Files:**
- Create: `web/src/atoms/debug.ts`
- Modify: `web/src/ws/types.ts`
- Modify: `web/src/ws/events.ts`

**Step 1: Create debug atoms**

Create `web/src/atoms/debug.ts`:

```typescript
import { atom } from "jotai";

export interface ToolExecutionRecord {
  id: string;
  session_id: string;
  tool_name: string;
  source: string;
  input: unknown;
  output: unknown | null;
  status: "running" | "success" | "failed" | "timeout";
  started_at: number;
  duration_ms: number | null;
  error: string | null;
}

export interface TokenBudget {
  total: number;
  system_prompt: number;
  dynamic_context: number;
  history: number;
  free: number;
  usage_percent: number;
  degradation_level: number;
}

export const executionRecordsAtom = atom<ToolExecutionRecord[]>([]);
export const tokenBudgetAtom = atom<TokenBudget | null>(null);
export const selectedExecutionIdAtom = atom<string | null>(null);
```

**Step 2: Add new ServerMessage types**

In `web/src/ws/types.ts`, add:

```typescript
  | {
      type: "tool_execution";
      session_id: string;
      execution: {
        id: string;
        session_id: string;
        tool_name: string;
        source: string;
        input: unknown;
        output: unknown | null;
        status: "running" | "success" | "failed" | "timeout";
        started_at: number;
        duration_ms: number | null;
        error: string | null;
      };
    }
  | {
      type: "token_budget_update";
      session_id: string;
      budget: {
        total: number;
        system_prompt: number;
        dynamic_context: number;
        history: number;
        free: number;
        usage_percent: number;
        degradation_level: number;
      };
    }
```

**Step 3: Handle new events**

In `web/src/ws/events.ts`, add imports and cases:

```typescript
import {
  executionRecordsAtom,
  tokenBudgetAtom,
} from "../atoms/debug";
```

Add cases:

```typescript
    case "tool_execution":
      set(executionRecordsAtom, (prev) => {
        const idx = prev.findIndex((e) => e.id === msg.execution.id);
        if (idx >= 0) {
          const next = [...prev];
          next[idx] = msg.execution;
          return next;
        }
        return [...prev, msg.execution];
      });
      break;

    case "token_budget_update":
      set(tokenBudgetAtom, msg.budget);
      break;
```

**Step 4: Verify TypeScript compiles**

Run: `cd web && npx tsc --noEmit`
Expected: 0 errors.

**Step 5: Commit**

```bash
git add web/src/atoms/debug.ts web/src/ws/types.ts web/src/ws/events.ts
git commit -m "feat(web): debug atoms + WebSocket event handling (tool_execution, token_budget)"
```

---

### Task 12: Frontend - 3 Tab layout + Tools page + Debug page

**Files:**
- Modify: `web/src/atoms/ui.ts`
- Modify: `web/src/components/layout/TabBar.tsx`
- Modify: `web/src/App.tsx`
- Create: `web/src/pages/Tools.tsx`
- Create: `web/src/pages/Debug.tsx`
- Create: `web/src/components/tools/ExecutionList.tsx`
- Create: `web/src/components/tools/ExecutionDetail.tsx`
- Create: `web/src/components/debug/TokenBudgetBar.tsx`

**Step 1: Update tab atoms**

In `web/src/atoms/ui.ts`:

```typescript
import { atom } from "jotai";

export type TabId = "chat" | "tools" | "debug";
export const activeTabAtom = atom<TabId>("chat");
export const sidebarOpenAtom = atom(false);
```

**Step 2: Update TabBar**

Replace `web/src/components/layout/TabBar.tsx`:

```typescript
import { useAtom } from "jotai";
import { cn } from "@/lib/utils";
import { activeTabAtom, type TabId } from "@/atoms/ui";

const tabs: { id: TabId; label: string }[] = [
  { id: "chat", label: "Chat" },
  { id: "tools", label: "Tools" },
  { id: "debug", label: "Debug" },
];

export function TabBar() {
  const [activeTab, setActiveTab] = useAtom(activeTabAtom);

  return (
    <div className="flex h-10 items-center gap-1 border-b border-border bg-card px-4">
      {tabs.map((tab) => (
        <button
          key={tab.id}
          onClick={() => setActiveTab(tab.id)}
          className={cn(
            "rounded-md px-3 py-1 text-sm font-medium transition-colors",
            activeTab === tab.id
              ? "bg-secondary text-foreground"
              : "text-muted-foreground hover:text-foreground hover:bg-secondary/50",
          )}
        >
          {tab.label}
        </button>
      ))}
    </div>
  );
}
```

**Step 3: Create ExecutionList component**

Create `web/src/components/tools/ExecutionList.tsx`:

```typescript
import { useAtom } from "jotai";
import { executionRecordsAtom, selectedExecutionIdAtom } from "@/atoms/debug";
import { ExecutionDetail } from "./ExecutionDetail";

export function ExecutionList() {
  const [executions] = useAtom(executionRecordsAtom);
  const [selectedId, setSelectedId] = useAtom(selectedExecutionIdAtom);

  if (executions.length === 0) {
    return (
      <div className="flex flex-1 items-center justify-center text-muted-foreground text-sm">
        No tool executions yet. Start a conversation to see tool calls here.
      </div>
    );
  }

  return (
    <div className="flex flex-col overflow-auto">
      <table className="w-full text-sm">
        <thead className="sticky top-0 bg-card border-b border-border">
          <tr className="text-left text-muted-foreground">
            <th className="px-3 py-2 font-medium">Tool</th>
            <th className="px-3 py-2 font-medium">Source</th>
            <th className="px-3 py-2 font-medium">Status</th>
            <th className="px-3 py-2 font-medium">Duration</th>
            <th className="px-3 py-2 font-medium">Time</th>
          </tr>
        </thead>
        <tbody>
          {executions.map((exec) => (
            <tr
              key={exec.id}
              onClick={() =>
                setSelectedId(selectedId === exec.id ? null : exec.id)
              }
              className="border-b border-border/50 cursor-pointer hover:bg-secondary/30"
            >
              <td className="px-3 py-2 font-mono">{exec.tool_name}</td>
              <td className="px-3 py-2 text-muted-foreground">
                {typeof exec.source === "string" ? exec.source : "built_in"}
              </td>
              <td className="px-3 py-2">
                <StatusBadge status={exec.status} />
              </td>
              <td className="px-3 py-2 text-muted-foreground">
                {exec.duration_ms != null
                  ? `${(exec.duration_ms / 1000).toFixed(1)}s`
                  : "—"}
              </td>
              <td className="px-3 py-2 text-muted-foreground">
                {new Date(exec.started_at).toLocaleTimeString()}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {selectedId && (
        <ExecutionDetail
          execution={executions.find((e) => e.id === selectedId) ?? null}
          onClose={() => setSelectedId(null)}
        />
      )}
    </div>
  );
}

function StatusBadge({ status }: { status: string }) {
  const styles: Record<string, string> = {
    running: "text-yellow-500",
    success: "text-green-500",
    failed: "text-red-500",
    timeout: "text-orange-500",
  };
  const icons: Record<string, string> = {
    running: "...",
    success: "ok",
    failed: "err",
    timeout: "t/o",
  };
  return (
    <span className={`font-mono text-xs ${styles[status] ?? ""}`}>
      {icons[status] ?? status}
    </span>
  );
}
```

**Step 4: Create ExecutionDetail component**

Create `web/src/components/tools/ExecutionDetail.tsx`:

```typescript
import type { ToolExecutionRecord } from "@/atoms/debug";

interface Props {
  execution: ToolExecutionRecord | null;
  onClose: () => void;
}

export function ExecutionDetail({ execution, onClose }: Props) {
  if (!execution) return null;

  return (
    <div className="border-t border-border bg-card/50 p-4">
      <div className="flex items-center justify-between mb-3">
        <h3 className="font-mono text-sm font-medium">
          {execution.tool_name}
        </h3>
        <button
          onClick={onClose}
          className="text-muted-foreground hover:text-foreground text-xs"
        >
          close
        </button>
      </div>

      <div className="space-y-3">
        <details open>
          <summary className="text-xs text-muted-foreground cursor-pointer">
            Input
          </summary>
          <pre className="mt-1 rounded bg-secondary/50 p-2 text-xs overflow-auto max-h-40">
            {JSON.stringify(execution.input, null, 2)}
          </pre>
        </details>

        {execution.output != null && (
          <details open>
            <summary className="text-xs text-muted-foreground cursor-pointer">
              Output
            </summary>
            <pre className="mt-1 rounded bg-secondary/50 p-2 text-xs overflow-auto max-h-40">
              {typeof execution.output === "string"
                ? execution.output
                : JSON.stringify(execution.output, null, 2)}
            </pre>
          </details>
        )}

        {execution.error && (
          <div className="rounded bg-red-500/10 p-2 text-xs text-red-400">
            {execution.error}
          </div>
        )}
      </div>
    </div>
  );
}
```

**Step 5: Create TokenBudgetBar component**

Create `web/src/components/debug/TokenBudgetBar.tsx`:

```typescript
import { useAtom } from "jotai";
import { tokenBudgetAtom } from "@/atoms/debug";

const LEVEL_LABELS = ["L0 None", "L1 Soft Trim", "L2 Hard Clear", "L3 Compact"];

function usageColor(pct: number): string {
  if (pct < 60) return "bg-green-500";
  if (pct < 80) return "bg-yellow-500";
  if (pct < 90) return "bg-orange-500";
  return "bg-red-500";
}

function formatTokens(n: number): string {
  if (n >= 1000) return `${(n / 1000).toFixed(1)}K`;
  return `${n}`;
}

export function TokenBudgetBar() {
  const [budget] = useAtom(tokenBudgetAtom);

  if (!budget) {
    return (
      <div className="flex flex-1 items-center justify-center text-muted-foreground text-sm">
        No token budget data yet. Start a conversation to see context usage.
      </div>
    );
  }

  const total = budget.total || 1;
  const sysPct = (budget.system_prompt / total) * 100;
  const dynPct = (budget.dynamic_context / total) * 100;
  const histPct = (budget.history / total) * 100;

  return (
    <div className="p-4 space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-medium">
          Context Window Usage ({budget.usage_percent.toFixed(0)}%)
        </h3>
        <span className="text-xs font-mono text-muted-foreground">
          {LEVEL_LABELS[budget.degradation_level] ?? `L${budget.degradation_level}`}
        </span>
      </div>

      <div className="h-6 flex rounded overflow-hidden bg-secondary/30">
        {sysPct > 0 && (
          <div
            className="bg-blue-500 flex items-center justify-center text-[10px] text-white"
            style={{ width: `${sysPct}%` }}
            title={`System: ${formatTokens(budget.system_prompt)}`}
          >
            {sysPct > 5 && "Sys"}
          </div>
        )}
        {dynPct > 0 && (
          <div
            className="bg-purple-500 flex items-center justify-center text-[10px] text-white"
            style={{ width: `${dynPct}%` }}
            title={`Dynamic: ${formatTokens(budget.dynamic_context)}`}
          >
            {dynPct > 5 && "Dyn"}
          </div>
        )}
        {histPct > 0 && (
          <div
            className={`${usageColor(budget.usage_percent)} flex items-center justify-center text-[10px] text-white`}
            style={{ width: `${histPct}%` }}
            title={`History: ${formatTokens(budget.history)}`}
          >
            {histPct > 5 && "Hist"}
          </div>
        )}
      </div>

      <div className="grid grid-cols-2 gap-2 text-xs text-muted-foreground">
        <div>System Prompt: {formatTokens(budget.system_prompt)} tokens</div>
        <div>Dynamic Context: {formatTokens(budget.dynamic_context)} tokens</div>
        <div>Conversation: {formatTokens(budget.history)} tokens</div>
        <div>Free: {formatTokens(budget.free)} tokens</div>
      </div>
    </div>
  );
}
```

**Step 6: Create Tools page**

Create `web/src/pages/Tools.tsx`:

```typescript
import { ExecutionList } from "@/components/tools/ExecutionList";

export default function Tools() {
  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      <div className="px-4 py-2 border-b border-border">
        <h2 className="text-sm font-medium">Tool Executions</h2>
      </div>
      <ExecutionList />
    </div>
  );
}
```

**Step 7: Create Debug page**

Create `web/src/pages/Debug.tsx`:

```typescript
import { TokenBudgetBar } from "@/components/debug/TokenBudgetBar";

export default function Debug() {
  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      <div className="px-4 py-2 border-b border-border">
        <h2 className="text-sm font-medium">Debug Dashboard</h2>
      </div>
      <TokenBudgetBar />
    </div>
  );
}
```

**Step 8: Update App.tsx with tab routing**

Replace `web/src/App.tsx`:

```typescript
import { useAtom } from "jotai";
import { AppLayout } from "./components/layout/AppLayout";
import { activeTabAtom } from "./atoms/ui";
import Chat from "./pages/Chat";
import Tools from "./pages/Tools";
import Debug from "./pages/Debug";

export default function App() {
  const [activeTab] = useAtom(activeTabAtom);

  return (
    <AppLayout>
      {activeTab === "chat" && <Chat />}
      {activeTab === "tools" && <Tools />}
      {activeTab === "debug" && <Debug />}
    </AppLayout>
  );
}
```

**Step 9: Verify TypeScript compiles**

Run: `cd web && npx tsc --noEmit`
Expected: 0 errors.

**Step 10: Verify Vite builds**

Run: `cd web && npx vite build`
Expected: Build succeeds.

**Step 11: Commit**

```bash
git add web/src/atoms/ui.ts web/src/components/layout/TabBar.tsx web/src/App.tsx web/src/pages/Tools.tsx web/src/pages/Debug.tsx web/src/components/tools/ web/src/components/debug/
git commit -m "feat(web): 3-tab layout (Chat|Tools|Debug) + ExecutionList + TokenBudgetBar"
```

---

### Task 13: Full integration + build verification

**Files:**
- All files from Tasks 1-12
- Possibly small fixups

**Step 1: Full Rust build**

Run: `cargo build`
Expected: Compilation succeeds (warnings are OK).

**Step 2: Full TypeScript check**

Run: `cd web && npx tsc --noEmit`
Expected: 0 errors.

**Step 3: Full Vite build**

Run: `cd web && npx vite build`
Expected: Build succeeds.

**Step 4: Fix any compilation issues**

If any errors, fix them and rebuild.

**Step 5: Final commit**

```bash
git add -A
git commit -m "feat: Phase 2 Batch 3 complete - Skill Loader, MCP Client, Tool Execution, REST API, Debug UI"
```

---

## Task Summary

| Task | Description | New Files | Modified Files |
|------|-------------|-----------|---------------|
| 1 | Workspace deps + ToolSource | 0 | 4 |
| 2 | SkillDefinition + SKILL.md parser | 3 | 2 |
| 3 | SkillRegistry + SkillTool + builder | 2 | 3 |
| 4 | Skill hot-reload (notify) | 0 | 1 |
| 5 | McpClient trait + StdioMcpClient | 5 | 1 |
| 6 | McpToolBridge + McpManager | 0 (replace) | 2 |
| 7 | ToolExecution types + SQLite schema | 1 | 2 |
| 8 | ToolExecutionRecorder + AgentLoop | 1 | 3 |
| 9 | REST API endpoints + AppState | 7 | 4 |
| 10 | WebSocket new events | 0 | 3 |
| 11 | Frontend atoms + WS events | 1 | 2 |
| 12 | 3-tab UI + Tools + Debug pages | 6 | 3 |
| 13 | Full integration + build verification | 0 | varies |

**Total: ~26 new files, ~30 modified files, 13 tasks, 13 commits**
