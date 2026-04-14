//! Session summary storage — stores and retrieves per-session summaries
//! for cross-session context injection.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::debug;

/// A session summary record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub summary: String,
    pub event_count: usize,
    pub key_topics: Vec<String>,
    pub memory_count: usize,
    pub created_at: i64,
    pub updated_at: i64,
}

/// SQLite-backed session summary store.
pub struct SessionSummaryStore {
    conn: tokio_rusqlite::Connection,
}

impl SessionSummaryStore {
    pub fn new(conn: tokio_rusqlite::Connection) -> Self {
        Self { conn }
    }

    /// Save or update a session summary.
    pub async fn save(
        &self,
        session_id: &str,
        summary: &str,
        event_count: usize,
        key_topics: &[String],
        memory_count: usize,
    ) -> Result<()> {
        let sid = session_id.to_string();
        let sum = summary.to_string();
        let topics_json = serde_json::to_string(key_topics).unwrap_or_else(|_| "[]".to_string());
        let now = chrono::Utc::now().timestamp();

        self.conn
            .call(move |conn| {
                conn.execute(
                    "INSERT INTO session_summaries (session_id, summary, event_count, key_topics, memory_count, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                     ON CONFLICT(session_id) DO UPDATE SET
                       summary = excluded.summary,
                       event_count = excluded.event_count,
                       key_topics = excluded.key_topics,
                       memory_count = excluded.memory_count,
                       updated_at = excluded.updated_at",
                    rusqlite::params![sid, sum, event_count as i64, topics_json, memory_count as i64, now, now],
                )?;
                Ok(())
            })
            .await?;

        debug!(session_id, "Session summary saved");
        Ok(())
    }

    /// Get the most recent N session summaries, ordered by creation time descending.
    pub async fn recent(&self, limit: usize) -> Result<Vec<SessionSummary>> {
        let result = self
            .conn
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT session_id, summary, event_count, key_topics, memory_count, created_at, updated_at
                     FROM session_summaries
                     ORDER BY created_at DESC
                     LIMIT ?1",
                )?;

                let rows = stmt.query_map(rusqlite::params![limit as i64], |row| {
                    let topics_json: String = row.get::<_, String>(3).unwrap_or_else(|_| "[]".to_string());
                    let key_topics: Vec<String> =
                        serde_json::from_str(&topics_json).unwrap_or_default();
                    Ok(SessionSummary {
                        session_id: row.get(0)?,
                        summary: row.get(1)?,
                        event_count: row.get::<_, i64>(2)? as usize,
                        key_topics,
                        memory_count: row.get::<_, i64>(4)? as usize,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                    })
                })?;

                let mut summaries = Vec::new();
                for row in rows {
                    summaries.push(row?);
                }
                Ok(summaries)
            })
            .await?;

        Ok(result)
    }

    /// Get the latest stored summary for a session.
    ///
    /// Per ADR-V2-018 §D3 the schema is one row per `session_id` (upsert), so
    /// "latest" is always the only row — this is an alias for `get_by_session`
    /// added for naming clarity at the iterative-reuse call site.
    pub async fn get_latest(&self, session_id: &str) -> Result<Option<SessionSummary>> {
        self.get_by_session(session_id).await
    }

    /// Get summary for a specific session.
    pub async fn get_by_session(&self, session_id: &str) -> Result<Option<SessionSummary>> {
        let sid = session_id.to_string();
        let result = self
            .conn
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT session_id, summary, event_count, key_topics, memory_count, created_at, updated_at
                     FROM session_summaries
                     WHERE session_id = ?1",
                )?;

                let entry = stmt
                    .query_row(rusqlite::params![sid], |row| {
                        let topics_json: String = row.get::<_, String>(3).unwrap_or_else(|_| "[]".to_string());
                        let key_topics: Vec<String> =
                            serde_json::from_str(&topics_json).unwrap_or_default();
                        Ok(SessionSummary {
                            session_id: row.get(0)?,
                            summary: row.get(1)?,
                            event_count: row.get::<_, i64>(2)? as usize,
                            key_topics,
                            memory_count: row.get::<_, i64>(4)? as usize,
                            created_at: row.get(5)?,
                            updated_at: row.get(6)?,
                        })
                    })
                    .ok();
                Ok(entry)
            })
            .await?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    async fn setup_store() -> SessionSummaryStore {
        let db = Database::open_in_memory().await.unwrap();
        SessionSummaryStore::new(db.conn().clone())
    }

    #[tokio::test]
    async fn test_save_and_get() {
        let store = setup_store().await;
        store
            .save("session-1", "Discussed auth flow", 2, &["auth".into(), "api".into()], 3)
            .await
            .unwrap();

        let summary = store.get_by_session("session-1").await.unwrap();
        assert!(summary.is_some());
        let s = summary.unwrap();
        assert_eq!(s.summary, "Discussed auth flow");
        assert_eq!(s.event_count, 2);
        assert_eq!(s.key_topics, vec!["auth", "api"]);
        assert_eq!(s.memory_count, 3);
    }

    #[tokio::test]
    async fn test_upsert() {
        let store = setup_store().await;
        store.save("s1", "First", 1, &[], 0).await.unwrap();
        store.save("s1", "Updated", 3, &["new".into()], 2).await.unwrap();

        let s = store.get_by_session("s1").await.unwrap().unwrap();
        assert_eq!(s.summary, "Updated");
        assert_eq!(s.event_count, 3);
    }

    #[tokio::test]
    async fn test_recent() {
        let store = setup_store().await;
        for i in 0..5 {
            store.save(&format!("s{i}"), &format!("Summary {i}"), i, &[], 0).await.unwrap();
        }

        let recent = store.recent(3).await.unwrap();
        assert_eq!(recent.len(), 3);
    }

    #[tokio::test]
    async fn test_get_nonexistent() {
        let store = setup_store().await;
        let result = store.get_by_session("nonexistent").await.unwrap();
        assert!(result.is_none());
    }
}
