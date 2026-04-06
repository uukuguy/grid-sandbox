//! Mock L3 — simulated L3 governance layer for certifier testing.
//!
//! Provides minimal L3 behavior: policy injection, hook evaluation,
//! telemetry reception. Full implementation deferred to Phase BH.

/// Mock L3 client trait (for future transparent replacement).
pub trait L3Client: Send + Sync {
    /// Get managed hooks JSON for session initialization.
    fn managed_hooks_json(&self) -> String;
}

/// Simple mock that returns empty hooks.
pub struct MockL3 {
    hooks_json: String,
}

impl MockL3 {
    pub fn new() -> Self {
        Self {
            hooks_json: "{}".into(),
        }
    }

    pub fn with_hooks(hooks_json: impl Into<String>) -> Self {
        Self {
            hooks_json: hooks_json.into(),
        }
    }
}

impl Default for MockL3 {
    fn default() -> Self {
        Self::new()
    }
}

impl L3Client for MockL3 {
    fn managed_hooks_json(&self) -> String {
        self.hooks_json.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_l3_default_empty_hooks() {
        let mock = MockL3::new();
        assert_eq!(mock.managed_hooks_json(), "{}");
    }

    #[test]
    fn mock_l3_custom_hooks() {
        let mock = MockL3::with_hooks(r#"{"rules": []}"#);
        assert!(mock.managed_hooks_json().contains("rules"));
    }
}
