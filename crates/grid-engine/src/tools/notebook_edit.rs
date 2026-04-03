//! NotebookEditTool — edit Jupyter notebook (.ipynb) cells.
//!
//! Supports inserting, replacing, and deleting cells in .ipynb JSON files.
//! Phase AS: CC-OSS gap closure.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{bail, Result};
use async_trait::async_trait;
use grid_types::{ApprovalRequirement, RiskLevel, ToolContext, ToolOutput, ToolSource};
use serde_json::{json, Value};

use super::path_safety::reject_symlink;
use super::traits::Tool;

/// Split source text into notebook-format lines (each line except last gets trailing \n).
fn split_source_lines(source: &str) -> Vec<Value> {
    let lines: Vec<&str> = source.lines().collect();
    let total = lines.len();
    lines
        .into_iter()
        .enumerate()
        .map(|(i, line)| {
            if i < total.saturating_sub(1) {
                Value::String(format!("{}\n", line))
            } else {
                Value::String(line.to_string())
            }
        })
        .collect()
}

pub struct NotebookEditTool;

impl NotebookEditTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for NotebookEditTool {
    fn name(&self) -> &str {
        "notebook_edit"
    }

    fn description(&self) -> &str {
        "Edit a Jupyter notebook (.ipynb) cell. Supports three actions:\n\
         - insert: Insert a new cell at a position\n\
         - replace: Replace the source of an existing cell\n\
         - delete: Delete a cell at a position\n\
         \n\
         The notebook must be a valid .ipynb JSON file."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute path to the .ipynb notebook file"
                },
                "action": {
                    "type": "string",
                    "enum": ["insert", "replace", "delete"],
                    "description": "The edit action to perform"
                },
                "cell_index": {
                    "type": "integer",
                    "description": "0-based index of the cell to edit (for replace/delete) or insert before"
                },
                "cell_type": {
                    "type": "string",
                    "enum": ["code", "markdown", "raw"],
                    "description": "Cell type (required for insert, optional for replace)"
                },
                "source": {
                    "type": "string",
                    "description": "New cell source content (required for insert and replace)"
                }
            },
            "required": ["path", "action", "cell_index"]
        })
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<ToolOutput> {
        let path_str = params
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: path"))?;

        let path = PathBuf::from(path_str);

        // Security: validate path against policy
        if let Some(ref validator) = ctx.path_validator {
            if let Err(e) = validator.check_path(&path) {
                return Ok(ToolOutput::error(format!("Path validation failed: {e}")));
            }
        }
        // Symlink defense: reject symbolic links
        if let Some(output) = reject_symlink(&path) {
            return Ok(output);
        }

        let action = params
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: action"))?;

        let cell_index = params
            .get("cell_index")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: cell_index"))?
            as usize;

        // Read notebook
        let content = tokio::fs::read_to_string(&path).await.map_err(|e| {
            anyhow::anyhow!("Failed to read notebook {}: {}", path.display(), e)
        })?;

        let mut notebook: Value = serde_json::from_str(&content).map_err(|e| {
            anyhow::anyhow!("Invalid notebook JSON in {}: {}", path.display(), e)
        })?;

        // Validate notebook structure
        let cells = notebook
            .get_mut("cells")
            .and_then(|v| v.as_array_mut())
            .ok_or_else(|| anyhow::anyhow!("Notebook missing 'cells' array"))?;

        let message = match action {
            "insert" => {
                let source = params
                    .get("source")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("'source' is required for insert action"))?;

                let cell_type = params
                    .get("cell_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("code");

                if cell_index > cells.len() {
                    bail!(
                        "cell_index {} is out of range (notebook has {} cells)",
                        cell_index,
                        cells.len()
                    );
                }

                let source_lines = split_source_lines(source);

                let mut new_cell = json!({
                    "cell_type": cell_type,
                    "metadata": {},
                    "source": source_lines,
                });
                if cell_type == "code" {
                    new_cell["outputs"] = json!([]);
                    new_cell["execution_count"] = json!(null);
                }

                cells.insert(cell_index, new_cell);

                format!(
                    "Inserted {} cell at index {} in {}",
                    cell_type, cell_index, path.display()
                )
            }

            "replace" => {
                let source = params
                    .get("source")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("'source' is required for replace action"))?;

                if cell_index >= cells.len() {
                    bail!(
                        "cell_index {} is out of range (notebook has {} cells)",
                        cell_index,
                        cells.len()
                    );
                }

                if let Some(new_type) = params.get("cell_type").and_then(|v| v.as_str()) {
                    cells[cell_index]["cell_type"] = Value::String(new_type.to_string());
                }

                cells[cell_index]["source"] = Value::Array(split_source_lines(source));

                // Clear outputs for code cells
                if cells[cell_index].get("cell_type").and_then(|v| v.as_str()) == Some("code") {
                    cells[cell_index]["outputs"] = json!([]);
                    cells[cell_index]["execution_count"] = json!(null);
                }

                format!("Replaced cell {} in {}", cell_index, path.display())
            }

            "delete" => {
                if cell_index >= cells.len() {
                    bail!(
                        "cell_index {} is out of range (notebook has {} cells)",
                        cell_index,
                        cells.len()
                    );
                }

                cells.remove(cell_index);

                format!(
                    "Deleted cell {} from {} ({} cells remaining)",
                    cell_index,
                    path.display(),
                    cells.len()
                )
            }

            other => bail!("Unknown action '{}'. Use insert, replace, or delete.", other),
        };

        // Write back
        let output = serde_json::to_string_pretty(&notebook)?;
        tokio::fs::write(&path, output).await.map_err(|e| {
            anyhow::anyhow!("Failed to write notebook {}: {}", path.display(), e)
        })?;

        Ok(ToolOutput::success(message))
    }

    fn source(&self) -> ToolSource {
        ToolSource::BuiltIn
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::HighRisk
    }

    fn approval(&self) -> ApprovalRequirement {
        ApprovalRequirement::AutoApprovable
    }

    fn execution_timeout(&self) -> Duration {
        Duration::from_secs(30)
    }

    fn category(&self) -> &str {
        "file"
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn is_concurrency_safe(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_ctx() -> ToolContext {
        ToolContext {
            sandbox_id: grid_types::SandboxId::default(),
            user_id: grid_types::UserId::from_string(grid_types::id::DEFAULT_USER_ID),
            working_dir: PathBuf::from("/tmp"),
            path_validator: None,
        }
    }

    #[test]
    fn test_tool_metadata() {
        let tool = NotebookEditTool::new();
        assert_eq!(tool.name(), "notebook_edit");
        assert!(!tool.is_read_only());
        assert_eq!(tool.category(), "file");
        assert!(!tool.is_concurrency_safe());
    }

    #[tokio::test]
    async fn test_missing_path() {
        let tool = NotebookEditTool::new();
        let result = tool
            .execute(
                json!({"action": "insert", "cell_index": 0}),
                &test_ctx(),
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_insert_replace_delete() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.ipynb");

        // Create minimal notebook
        let notebook = json!({
            "cells": [
                {
                    "cell_type": "code",
                    "metadata": {},
                    "source": ["print('hello')"],
                    "outputs": [],
                    "execution_count": null
                }
            ],
            "metadata": {
                "kernelspec": {
                    "display_name": "Python 3",
                    "language": "python",
                    "name": "python3"
                }
            },
            "nbformat": 4,
            "nbformat_minor": 5
        });
        tokio::fs::write(&path, serde_json::to_string_pretty(&notebook).unwrap())
            .await
            .unwrap();

        let tool = NotebookEditTool::new();
        let ctx = test_ctx();

        // Insert a markdown cell at index 0
        let result = tool
            .execute(
                json!({
                    "path": path.to_str().unwrap(),
                    "action": "insert",
                    "cell_index": 0,
                    "cell_type": "markdown",
                    "source": "# Title"
                }),
                &ctx,
            )
            .await
            .unwrap();
        assert!(result.content.contains("Inserted markdown cell"));

        // Verify
        let nb: Value =
            serde_json::from_str(&tokio::fs::read_to_string(&path).await.unwrap()).unwrap();
        assert_eq!(nb["cells"].as_array().unwrap().len(), 2);
        assert_eq!(nb["cells"][0]["cell_type"], "markdown");

        // Replace cell 1
        let result = tool
            .execute(
                json!({
                    "path": path.to_str().unwrap(),
                    "action": "replace",
                    "cell_index": 1,
                    "source": "print('world')"
                }),
                &ctx,
            )
            .await
            .unwrap();
        assert!(result.content.contains("Replaced cell 1"));

        // Delete cell 0
        let result = tool
            .execute(
                json!({
                    "path": path.to_str().unwrap(),
                    "action": "delete",
                    "cell_index": 0
                }),
                &ctx,
            )
            .await
            .unwrap();
        assert!(result.content.contains("Deleted cell 0"));
        assert!(result.content.contains("1 cells remaining"));
    }
}
