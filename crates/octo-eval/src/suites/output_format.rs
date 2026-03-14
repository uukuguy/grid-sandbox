//! Output format evaluation suite — tests structured output generation (JSON, YAML, CSV, Markdown).

use std::path::Path;

use anyhow::Result;

use crate::datasets::loader::{load_jsonl, load_jsonl_as_tasks, JsonlTask};
use crate::task::EvalTask;

/// Output format evaluation suite
pub struct OutputFormatSuite;

impl OutputFormatSuite {
    /// Default dataset path (relative to crate root)
    const DEFAULT_DATASET: &'static str = "datasets/octo_output_format.jsonl";

    /// Load tasks from the default dataset
    pub fn load() -> Result<Vec<Box<dyn EvalTask>>> {
        // Try relative to current dir, then relative to crate manifest dir
        let path = Path::new(Self::DEFAULT_DATASET);
        if path.exists() {
            return load_jsonl_as_tasks(path);
        }
        // Try from crate root
        let crate_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(Self::DEFAULT_DATASET);
        if crate_path.exists() {
            return load_jsonl_as_tasks(&crate_path);
        }
        anyhow::bail!(
            "Output format dataset not found at {} or {}",
            path.display(),
            crate_path.display()
        )
    }

    /// Load tasks from a custom path
    pub fn load_from(path: &Path) -> Result<Vec<Box<dyn EvalTask>>> {
        load_jsonl_as_tasks(path)
    }

    /// Load raw JsonlTask structs (useful for inspection)
    pub fn load_raw() -> Result<Vec<JsonlTask>> {
        let crate_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(Self::DEFAULT_DATASET);
        load_jsonl(&crate_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_output_format_suite() {
        let tasks = OutputFormatSuite::load().unwrap();
        assert!(
            tasks.len() >= 6,
            "Expected at least 6 output_format tasks, got {}",
            tasks.len()
        );
    }

    #[test]
    fn test_load_raw() {
        let tasks = OutputFormatSuite::load_raw().unwrap();
        assert!(tasks.len() >= 6);
        assert_eq!(tasks[0].category, "output_format");
    }
}
