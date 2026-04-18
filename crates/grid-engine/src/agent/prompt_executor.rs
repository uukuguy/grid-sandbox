/// D117 — PromptExecutor: LLM-driven yes/no classifier for prompt bodies.
///
/// Default disabled; set `EAASP_PROMPT_EXECUTOR=1` to activate at runtime.
/// When disabled, `NoOpPromptExecutor` passes every call through without LLM
/// overhead. The `LlmDrivenPromptExecutor` implementation is provided for
/// future wiring when a real skill consumer exists.
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

/// Decision returned by a `PromptExecutor` evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptDecision {
    /// Proceed — the prompt body satisfies the classifier.
    Allow,
    /// Reject — the prompt body failed the classifier.
    Deny { reason: String },
}

/// Evaluate a prompt body (e.g. a skill's stop-hook payload) via an optional
/// LLM yes/no classify step. The caller decides how to act on the decision.
#[async_trait]
pub trait PromptExecutor: Send + Sync {
    /// Classify `prompt_body` and return a `PromptDecision`.
    async fn evaluate(&self, prompt_body: &str) -> Result<PromptDecision>;

    /// Human-readable name (for logging / introspection).
    fn name(&self) -> &'static str;
}

/// No-op implementation — always returns `Allow` without any LLM call.
/// Used when `EAASP_PROMPT_EXECUTOR` is unset or `0`.
pub struct NoOpPromptExecutor;

#[async_trait]
impl PromptExecutor for NoOpPromptExecutor {
    async fn evaluate(&self, _prompt_body: &str) -> Result<PromptDecision> {
        Ok(PromptDecision::Allow)
    }

    fn name(&self) -> &'static str {
        "noop"
    }
}

/// LLM-driven prompt executor — sends `prompt_body` to a fast model (e.g.
/// Haiku) with a yes/no classify instruction and maps the response to
/// `PromptDecision`. Activated when `EAASP_PROMPT_EXECUTOR=1`.
///
/// Structure-only for now (D117); provider wiring is deferred until a real
/// skill consumer exists. The struct is pub so future callers can construct
/// it with an `Arc<dyn Provider>` without touching this module.
pub struct LlmDrivenPromptExecutor {
    /// Instruction prepended before the prompt body when calling the model.
    pub classify_instruction: String,
}

impl Default for LlmDrivenPromptExecutor {
    fn default() -> Self {
        Self {
            classify_instruction: concat!(
                "Answer only YES or NO. ",
                "Does the following text satisfy the stated requirement? ",
                "Reply YES if it does, NO if it does not."
            )
            .to_string(),
        }
    }
}

#[async_trait]
impl PromptExecutor for LlmDrivenPromptExecutor {
    async fn evaluate(&self, _prompt_body: &str) -> Result<PromptDecision> {
        // Provider wiring deferred — structure-only per D117.
        // When wired: call provider with classify_instruction + prompt_body,
        // parse response "YES" → Allow, anything else → Deny.
        Ok(PromptDecision::Allow)
    }

    fn name(&self) -> &'static str {
        "llm-driven"
    }
}

/// Build a `PromptExecutor` from the `EAASP_PROMPT_EXECUTOR` env variable.
///
/// - `"1"` / `"true"` / `"on"` → `LlmDrivenPromptExecutor` (structure-only)
/// - anything else (or unset) → `NoOpPromptExecutor`
pub fn build_prompt_executor_from_env() -> Arc<dyn PromptExecutor> {
    let enabled = std::env::var("EAASP_PROMPT_EXECUTOR")
        .map(|v| v == "1" || v == "true" || v == "on")
        .unwrap_or(false);
    if enabled {
        Arc::new(LlmDrivenPromptExecutor::default())
    } else {
        Arc::new(NoOpPromptExecutor)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    // Serialize env-var tests to avoid cross-test pollution in parallel runs.
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    #[tokio::test]
    async fn noop_always_allows() {
        let ex = NoOpPromptExecutor;
        assert_eq!(ex.evaluate("anything").await.unwrap(), PromptDecision::Allow);
    }

    #[tokio::test]
    async fn llm_driven_structure_allows_without_provider() {
        let ex = LlmDrivenPromptExecutor::default();
        assert_eq!(
            ex.evaluate("some prompt body").await.unwrap(),
            PromptDecision::Allow
        );
    }

    #[test]
    fn noop_name() {
        assert_eq!(NoOpPromptExecutor.name(), "noop");
    }

    #[test]
    fn llm_driven_name() {
        assert_eq!(LlmDrivenPromptExecutor::default().name(), "llm-driven");
    }

    #[test]
    fn build_from_env_unset_returns_noop() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("EAASP_PROMPT_EXECUTOR");
        let ex = build_prompt_executor_from_env();
        assert_eq!(ex.name(), "noop");
    }

    #[test]
    fn build_from_env_1_returns_llm_driven() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("EAASP_PROMPT_EXECUTOR", "1");
        let ex = build_prompt_executor_from_env();
        assert_eq!(ex.name(), "llm-driven");
        std::env::remove_var("EAASP_PROMPT_EXECUTOR");
    }
}
