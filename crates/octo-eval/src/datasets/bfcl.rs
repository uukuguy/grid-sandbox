//! BFCL (Berkeley Function Calling Leaderboard) dataset adapter.
//! Supports the "simple" subset format.

use anyhow::Result;
use serde::Deserialize;
use std::path::Path;

use crate::score::{EvalScore, ScoreDetails};
use crate::scorer::{FunctionCallMatchScorer, Scorer};
use crate::task::{AgentOutput, Difficulty, EvalTask, LlmJudgeConfig, TaskMetadata};

/// Raw BFCL entry as read from JSONL
#[derive(Debug, Deserialize)]
struct BfclRawEntry {
    id: String,
    question: Vec<Vec<BfclMessage>>,
    function: Vec<serde_json::Value>,
    ground_truth: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct BfclMessage {
    #[allow(dead_code)]
    role: String,
    content: String,
}

/// Converted BFCL task implementing EvalTask
#[derive(Debug)]
pub struct BfclTask {
    pub id: String,
    pub prompt: String,
    pub functions: Vec<serde_json::Value>,
    pub ground_truth: Vec<String>,
}

impl EvalTask for BfclTask {
    fn id(&self) -> &str {
        &self.id
    }

    fn prompt(&self) -> &str {
        &self.prompt
    }

    fn available_tools(&self) -> Option<Vec<octo_types::tool::ToolSpec>> {
        let tools: Vec<octo_types::tool::ToolSpec> = self
            .functions
            .iter()
            .filter_map(|f| {
                let name = f.get("name")?.as_str()?.to_string();
                let description = f
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("")
                    .to_string();
                let input_schema = f
                    .get("parameters")
                    .cloned()
                    .unwrap_or(serde_json::json!({"type": "object", "properties": {}}));
                Some(octo_types::tool::ToolSpec {
                    name,
                    description,
                    input_schema,
                })
            })
            .collect();
        if tools.is_empty() {
            None
        } else {
            Some(tools)
        }
    }

    fn score(&self, output: &AgentOutput) -> EvalScore {
        if let Some(expected) = self.ground_truth.first() {
            let scorer = FunctionCallMatchScorer::new(expected);
            scorer.score(output)
        } else {
            EvalScore::fail(
                0.0,
                ScoreDetails::Custom {
                    message: "No ground truth".into(),
                },
            )
        }
    }

    fn metadata(&self) -> TaskMetadata {
        TaskMetadata {
            category: "bfcl".into(),
            difficulty: Difficulty::Medium,
            expected_steps: Some(1),
            tags: vec!["function_calling".into()],
        }
    }

    fn tool_allowlist(&self) -> Option<Vec<String>> {
        None
    }

    fn llm_judge_config(&self) -> Option<LlmJudgeConfig> {
        None
    }
}

/// Load BFCL simple format from a JSONL file
pub fn load_bfcl(path: &Path) -> Result<Vec<BfclTask>> {
    let content = std::fs::read_to_string(path)?;
    let mut tasks = Vec::new();
    for (i, line) in content.lines().enumerate() {
        if line.trim().is_empty() || line.trim().starts_with('#') {
            continue;
        }
        let raw: BfclRawEntry = serde_json::from_str(line)
            .map_err(|e| anyhow::anyhow!("BFCL line {}: {}", i + 1, e))?;

        // Extract prompt from last message in first conversation
        let prompt = raw
            .question
            .first()
            .and_then(|msgs| msgs.last())
            .map(|m| m.content.clone())
            .unwrap_or_default();

        tasks.push(BfclTask {
            id: raw.id,
            prompt,
            functions: raw.function,
            ground_truth: raw.ground_truth,
        });
    }
    Ok(tasks)
}

/// Load BFCL tasks as boxed trait objects
pub fn load_bfcl_as_tasks(path: &Path) -> Result<Vec<Box<dyn EvalTask>>> {
    let tasks = load_bfcl(path)?;
    Ok(tasks
        .into_iter()
        .map(|t| Box::new(t) as Box<dyn EvalTask>)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_bfcl_sample() {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let path = manifest_dir.join("datasets/bfcl_simple.jsonl");
        if path.exists() {
            let tasks = load_bfcl(&path).unwrap();
            assert!(!tasks.is_empty());
            assert!(!tasks[0].prompt.is_empty());
            assert!(!tasks[0].ground_truth.is_empty());
        }
    }

    #[test]
    fn test_load_bfcl_parse_all_tasks() {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let path = manifest_dir.join("datasets/bfcl_simple.jsonl");
        if path.exists() {
            let tasks = load_bfcl(&path).unwrap();
            assert_eq!(tasks.len(), 50);
            for task in &tasks {
                assert!(!task.functions.is_empty());
                assert!(!task.ground_truth.is_empty());
                assert!(task.available_tools().is_some());
            }
        }
    }

    #[test]
    fn test_load_bfcl_as_tasks_trait_objects() {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let path = manifest_dir.join("datasets/bfcl_simple.jsonl");
        if path.exists() {
            let tasks = load_bfcl_as_tasks(&path).unwrap();
            assert_eq!(tasks.len(), 50);
            for task in &tasks {
                let meta = task.metadata();
                assert_eq!(meta.category, "bfcl");
                assert_eq!(meta.difficulty, Difficulty::Medium);
            }
        }
    }
}
