//! Per-round memory extraction — incrementally captures memories after each agent loop round.
//!
//! Reuses the existing [`RuleBasedExtractor`] from `auto_extractor` module but operates
//! on a sliding window of recent messages rather than the full session history.
//! This prevents memory loss when long sessions are interrupted before the
//! session-end hook fires.

use grid_types::{ChatMessage, MemoryCategory, MemoryEntry, MemorySource};
use tracing::{debug, warn};

use super::auto_extractor::{AutoMemoryCategory, ExtractedMemory, MemoryExtractor, RuleBasedExtractor};
use super::store_traits::MemoryStore;

/// Configuration for per-round memory extraction.
#[derive(Debug, Clone)]
pub struct RoundMemoryConfig {
    /// Enable per-round extraction.
    pub enabled: bool,
    /// Minimum confidence threshold for storing extracted memories.
    pub min_confidence: f64,
    /// Maximum memories to extract per round.
    pub max_per_round: usize,
    /// Extract every N rounds (1 = every round, 3 = every 3rd round).
    pub extract_interval: u32,
}

impl Default for RoundMemoryConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Opt-in to avoid extra processing by default
            min_confidence: 0.6,
            max_per_round: 5,
            extract_interval: 1,
        }
    }
}

/// Per-round memory extractor.
///
/// Maintains a cursor position (index into messages array) to track which
/// messages have already been processed. Each call to [`extract_round`]
/// only processes new messages since the last call.
pub struct RoundMemoryExtractor {
    extractor: Box<dyn MemoryExtractor>,
    config: RoundMemoryConfig,
    /// Index of the next unprocessed message in the conversation history.
    last_processed_idx: usize,
}

impl RoundMemoryExtractor {
    pub fn new(config: RoundMemoryConfig) -> Self {
        Self {
            extractor: Box::new(RuleBasedExtractor::new()),
            config,
            last_processed_idx: 0,
        }
    }

    /// Extract memories from new messages since last round.
    ///
    /// Returns the number of memories stored.
    pub async fn extract_round(
        &mut self,
        messages: &[ChatMessage],
        store: &dyn MemoryStore,
        user_id: &str,
        round: u32,
    ) -> usize {
        if !self.config.enabled {
            return 0;
        }

        // Check interval
        if self.config.extract_interval > 1 && round % self.config.extract_interval != 0 {
            return 0;
        }

        // Only process new messages
        if messages.len() <= self.last_processed_idx {
            return 0;
        }

        let new_messages = &messages[self.last_processed_idx..];
        self.last_processed_idx = messages.len();

        if new_messages.is_empty() {
            return 0;
        }

        // Run rule-based extraction on new messages only
        let extracted = self.extractor.extract(new_messages).await;

        // Filter by confidence and limit
        let filtered: Vec<&ExtractedMemory> = extracted
            .iter()
            .filter(|m| m.confidence >= self.config.min_confidence)
            .take(self.config.max_per_round)
            .collect();

        if filtered.is_empty() {
            return 0;
        }

        let mut stored = 0;
        for mem in &filtered {
            let mut entry = MemoryEntry::new(user_id, map_category(&mem.category), &mem.value);
            entry.source_type = MemorySource::Extracted;
            entry.importance = mem.confidence as f32;

            match store.store(entry).await {
                Ok(_) => stored += 1,
                Err(e) => {
                    warn!("Failed to store round memory: {e}");
                }
            }
        }

        if stored > 0 {
            debug!(round, stored, "Round memory extraction complete");
        }

        stored
    }

    /// Reset the cursor (e.g., after context compaction).
    pub fn reset_cursor(&mut self) {
        self.last_processed_idx = 0;
    }
}

/// Map from auto-extractor category to the standard MemoryCategory.
fn map_category(cat: &AutoMemoryCategory) -> MemoryCategory {
    match cat {
        AutoMemoryCategory::ProjectStructure => MemoryCategory::Profile,
        AutoMemoryCategory::UserPreference => MemoryCategory::Preferences,
        AutoMemoryCategory::CommandPattern => MemoryCategory::Tools,
        AutoMemoryCategory::TechnicalDecision => MemoryCategory::Patterns,
        AutoMemoryCategory::ContextualFact => MemoryCategory::Debug,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use async_trait::async_trait;
    use grid_types::{MemoryFilter, MemoryId, MemoryResult, SearchOptions};
    use std::sync::Mutex;

    /// Mock MemoryStore for testing.
    struct MockStore {
        stored: Mutex<Vec<MemoryEntry>>,
    }

    impl MockStore {
        fn new() -> Self {
            Self {
                stored: Mutex::new(Vec::new()),
            }
        }
        fn count(&self) -> usize {
            self.stored.lock().unwrap().len()
        }
    }

    #[async_trait]
    impl MemoryStore for MockStore {
        async fn store(&self, entry: MemoryEntry) -> Result<MemoryId> {
            self.stored.lock().unwrap().push(entry);
            Ok(MemoryId::new())
        }
        async fn search(&self, _query: &str, _opts: SearchOptions) -> Result<Vec<MemoryResult>> {
            Ok(vec![])
        }
        async fn get(&self, _id: &MemoryId) -> Result<Option<MemoryEntry>> {
            Ok(None)
        }
        async fn update(&self, _id: &MemoryId, _content: &str) -> Result<()> {
            Ok(())
        }
        async fn delete(&self, _id: &MemoryId) -> Result<()> {
            Ok(())
        }
        async fn delete_by_filter(&self, _filter: MemoryFilter) -> Result<usize> {
            Ok(0)
        }
        async fn list(&self, _filter: MemoryFilter) -> Result<Vec<MemoryEntry>> {
            Ok(vec![])
        }
        async fn batch_store(&self, entries: Vec<MemoryEntry>) -> Result<Vec<MemoryId>> {
            let mut guard = self.stored.lock().unwrap();
            let ids: Vec<MemoryId> = entries
                .into_iter()
                .map(|e| {
                    guard.push(e);
                    MemoryId::new()
                })
                .collect();
            Ok(ids)
        }
    }

    #[tokio::test]
    async fn test_disabled_extraction() {
        let config = RoundMemoryConfig::default(); // enabled: false
        let mut extractor = RoundMemoryExtractor::new(config);
        let store = MockStore::new();
        let messages = vec![ChatMessage::user("Check src/main.rs")];
        let stored = extractor
            .extract_round(&messages, &store, "user1", 1)
            .await;
        assert_eq!(stored, 0);
        assert_eq!(store.count(), 0);
    }

    #[tokio::test]
    async fn test_enabled_extraction_stores_memories() {
        let config = RoundMemoryConfig {
            enabled: true,
            min_confidence: 0.5,
            max_per_round: 10,
            extract_interval: 1,
        };
        let mut extractor = RoundMemoryExtractor::new(config);
        let store = MockStore::new();

        // Messages with recognizable file paths
        let messages = vec![
            ChatMessage::user("Please check src/main.rs and crates/octo-engine/Cargo.toml"),
            ChatMessage::assistant("I found the issue in web/src/App.tsx"),
        ];
        let stored = extractor
            .extract_round(&messages, &store, "user1", 1)
            .await;
        assert!(stored > 0, "Should store at least one memory");
        assert_eq!(store.count(), stored);
    }

    #[tokio::test]
    async fn test_cursor_only_processes_new_messages() {
        let config = RoundMemoryConfig {
            enabled: true,
            min_confidence: 0.5,
            max_per_round: 10,
            extract_interval: 1,
        };
        let mut extractor = RoundMemoryExtractor::new(config);
        let store = MockStore::new();

        let mut messages = vec![ChatMessage::user("Check src/main.rs")];

        // First round
        let first = extractor
            .extract_round(&messages, &store, "user1", 1)
            .await;

        // Second round with same messages — should extract nothing new
        let second = extractor
            .extract_round(&messages, &store, "user1", 2)
            .await;
        assert_eq!(second, 0, "No new messages, should extract nothing");

        // Add new message and extract again
        messages.push(ChatMessage::user("Also look at web/src/App.tsx"));
        let third = extractor
            .extract_round(&messages, &store, "user1", 3)
            .await;
        assert_eq!(store.count(), first + third);
    }

    #[tokio::test]
    async fn test_interval_skipping() {
        let config = RoundMemoryConfig {
            enabled: true,
            extract_interval: 3,
            min_confidence: 0.5,
            max_per_round: 10,
        };
        let mut extractor = RoundMemoryExtractor::new(config);
        let store = MockStore::new();

        let messages = vec![ChatMessage::user("Check src/main.rs")];

        // Round 1: skip (1 % 3 != 0)
        let r1 = extractor
            .extract_round(&messages, &store, "user1", 1)
            .await;
        assert_eq!(r1, 0);

        // Round 2: skip (2 % 3 != 0)
        let r2 = extractor
            .extract_round(&messages, &store, "user1", 2)
            .await;
        assert_eq!(r2, 0);

        // Round 3: extract (3 % 3 == 0)
        let r3 = extractor
            .extract_round(&messages, &store, "user1", 3)
            .await;
        assert!(r3 > 0, "Should extract on interval match");
    }

    #[tokio::test]
    async fn test_max_per_round_limit() {
        let config = RoundMemoryConfig {
            enabled: true,
            min_confidence: 0.0, // Accept everything
            max_per_round: 2,
            extract_interval: 1,
        };
        let mut extractor = RoundMemoryExtractor::new(config);
        let store = MockStore::new();

        // Many file paths to trigger lots of extractions
        let messages = vec![ChatMessage::user(
            "src/a.rs src/b.rs src/c.rs src/d.rs src/e.rs",
        )];
        let stored = extractor
            .extract_round(&messages, &store, "user1", 1)
            .await;
        assert!(stored <= 2, "Should respect max_per_round limit");
    }

    #[tokio::test]
    async fn test_reset_cursor() {
        let config = RoundMemoryConfig {
            enabled: true,
            min_confidence: 0.5,
            max_per_round: 10,
            extract_interval: 1,
        };
        let mut extractor = RoundMemoryExtractor::new(config);
        let store = MockStore::new();

        let messages = vec![ChatMessage::user("Check src/main.rs")];

        let first = extractor
            .extract_round(&messages, &store, "user1", 1)
            .await;
        assert!(first > 0);

        // Reset and re-extract same messages
        extractor.reset_cursor();
        let after_reset = extractor
            .extract_round(&messages, &store, "user1", 2)
            .await;
        assert!(after_reset > 0, "After reset, should re-process messages");
    }

    #[test]
    fn test_category_mapping() {
        assert_eq!(
            map_category(&AutoMemoryCategory::ProjectStructure),
            MemoryCategory::Profile
        );
        assert_eq!(
            map_category(&AutoMemoryCategory::UserPreference),
            MemoryCategory::Preferences
        );
        assert_eq!(
            map_category(&AutoMemoryCategory::CommandPattern),
            MemoryCategory::Tools
        );
        assert_eq!(
            map_category(&AutoMemoryCategory::TechnicalDecision),
            MemoryCategory::Patterns
        );
        assert_eq!(
            map_category(&AutoMemoryCategory::ContextualFact),
            MemoryCategory::Debug
        );
    }

    #[test]
    fn test_round_memory_config_defaults() {
        let config = RoundMemoryConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.extract_interval, 1);
        assert_eq!(config.max_per_round, 5);
        assert!((config.min_confidence - 0.6).abs() < f64::EPSILON);
    }
}
