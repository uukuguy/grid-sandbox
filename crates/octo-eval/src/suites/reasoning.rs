//! Reasoning and planning evaluation suite — tests multi-step reasoning and task decomposition.

use std::path::Path;

use anyhow::Result;

use crate::datasets::loader::{load_jsonl, load_jsonl_as_tasks, JsonlTask};
use crate::task::EvalTask;

/// Reasoning evaluation suite
pub struct ReasoningSuite;

impl ReasoningSuite {
    /// Default dataset path (relative to crate root)
    const DEFAULT_DATASET: &'static str = "datasets/octo_reasoning.jsonl";

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
            "Reasoning dataset not found at {} or {}",
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
    fn test_load_reasoning_suite() {
        let tasks = ReasoningSuite::load().unwrap();
        assert!(
            tasks.len() >= 6,
            "Expected at least 6 reasoning tasks, got {}",
            tasks.len()
        );
    }

    #[test]
    fn test_load_raw() {
        let tasks = ReasoningSuite::load_raw().unwrap();
        assert!(tasks.len() >= 6);
        assert_eq!(tasks[0].category, "reasoning");
    }
}
