use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::id::SandboxId;

/// Tool risk level (aligned with MCP Tool Annotations spec)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    /// Read-only operations (no side effects)
    ReadOnly,
    /// Low-risk modifications (default)
    LowRisk,
    /// High-risk modifications (file writes, config changes)
    HighRisk,
    /// Destructive operations (shell execution, deletions)
    Destructive,
}

/// Tool approval requirement
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalRequirement {
    /// Never requires approval (default)
    Never,
    /// Can be auto-approved based on policy rules
    AutoApprovable,
    /// Always requires explicit human approval
    Always,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolSource {
    BuiltIn,
    Mcp(String),   // MCP server name
    Skill(String), // Skill name
    Plugin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub output: String,
    pub is_error: bool,
}

impl ToolResult {
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            output: output.into(),
            is_error: false,
        }
    }

    pub fn error(output: impl Into<String>) -> Self {
        Self {
            output: output.into(),
            is_error: true,
        }
    }
}

/// Tool output artifact (file, image, structured data, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub name: String,
    pub content_type: String,
    pub data: String,
}

/// Structured tool output with artifacts, metadata, and truncation info.
///
/// This is an enhanced version of `ToolResult`. Existing code continues to use
/// `ToolResult`; new code can adopt `ToolOutput` and convert via `From`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    pub content: String,
    pub is_error: bool,
    pub artifacts: Vec<Artifact>,
    pub metadata: Option<serde_json::Value>,
    pub truncated: bool,
    pub original_size: Option<usize>,
    pub duration_ms: u64,
}

impl ToolOutput {
    /// Create a successful output with the given content.
    pub fn success(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            is_error: false,
            artifacts: Vec::new(),
            metadata: None,
            truncated: false,
            original_size: None,
            duration_ms: 0,
        }
    }

    /// Create an error output with the given content.
    pub fn error(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            is_error: true,
            artifacts: Vec::new(),
            metadata: None,
            truncated: false,
            original_size: None,
            duration_ms: 0,
        }
    }

    /// Attach an artifact to this output.
    pub fn with_artifact(mut self, artifact: Artifact) -> Self {
        self.artifacts.push(artifact);
        self
    }

    /// Attach JSON metadata to this output.
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Record the execution duration in milliseconds.
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }

    /// Mark this output as truncated, recording the original size in bytes.
    pub fn mark_truncated(mut self, original_size: usize) -> Self {
        self.truncated = true;
        self.original_size = Some(original_size);
        self
    }
}

impl From<ToolOutput> for ToolResult {
    fn from(output: ToolOutput) -> Self {
        ToolResult {
            output: output.content,
            is_error: output.is_error,
        }
    }
}

/// Trait for validating file paths against security policies.
pub trait PathValidator: Send + Sync + std::fmt::Debug {
    fn check_path(&self, path: &Path) -> Result<(), String>;
}

#[derive(Debug, Clone)]
pub struct ToolContext {
    pub sandbox_id: SandboxId,
    pub working_dir: PathBuf,
    pub path_validator: Option<Arc<dyn PathValidator>>,
}
