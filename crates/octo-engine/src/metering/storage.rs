//! Persistent metering storage backed by SQLite.

use crate::db::Database;
use crate::metering::pricing::ModelPricing;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct MeteringRecord {
    pub session_id: String,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub duration_ms: u64,
    pub is_error: bool,
}

#[derive(Debug, Clone, Default)]
pub struct MeteringSummary {
    pub model: String,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_requests: u64,
    pub total_errors: u64,
    pub total_duration_ms: u64,
}

pub struct MeteringStorage {
    db: Database,
}

impl MeteringStorage {
    pub fn new(db: Database) -> Self { Self { db } }

    pub async fn record(&self, rec: MeteringRecord) -> Result<()> {
        let is_error_int: i32 = if rec.is_error { 1 } else { 0 };
        self.db.conn().call(move |conn| {
            conn.execute(
                "INSERT INTO metering_records (session_id, model, input_tokens, output_tokens, duration_ms, is_error) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![rec.session_id, rec.model, rec.input_tokens as i64, rec.output_tokens as i64, rec.duration_ms as i64, is_error_int],
            )?;
            Ok(())
        }).await?;
        Ok(())
    }

    pub async fn summary_by_session(&self, session_id: String) -> Result<Vec<MeteringSummary>> {
        let rows = self.db.conn().call(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT model, COALESCE(SUM(input_tokens),0), COALESCE(SUM(output_tokens),0), COUNT(*), COALESCE(SUM(is_error),0), COALESCE(SUM(duration_ms),0) FROM metering_records WHERE session_id = ?1 GROUP BY model")?;
            let results = stmt.query_map(rusqlite::params![session_id], |row| {
                Ok(MeteringSummary {
                    model: row.get(0)?, total_input_tokens: row.get::<_, i64>(1)? as u64,
                    total_output_tokens: row.get::<_, i64>(2)? as u64, total_requests: row.get::<_, i64>(3)? as u64,
                    total_errors: row.get::<_, i64>(4)? as u64, total_duration_ms: row.get::<_, i64>(5)? as u64,
                })
            })?.collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(results)
        }).await?;
        Ok(rows)
    }

    pub async fn summary_by_model(&self) -> Result<Vec<MeteringSummary>> {
        let rows = self.db.conn().call(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT model, COALESCE(SUM(input_tokens),0), COALESCE(SUM(output_tokens),0), COUNT(*), COALESCE(SUM(is_error),0), COALESCE(SUM(duration_ms),0) FROM metering_records GROUP BY model")?;
            let results = stmt.query_map([], |row| {
                Ok(MeteringSummary {
                    model: row.get(0)?, total_input_tokens: row.get::<_, i64>(1)? as u64,
                    total_output_tokens: row.get::<_, i64>(2)? as u64, total_requests: row.get::<_, i64>(3)? as u64,
                    total_errors: row.get::<_, i64>(4)? as u64, total_duration_ms: row.get::<_, i64>(5)? as u64,
                })
            })?.collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(results)
        }).await?;
        Ok(rows)
    }

    pub async fn summary_global(&self, since: Option<String>) -> Result<MeteringSummary> {
        let row = self.db.conn().call(move |conn| {
            let (sql, params): (String, Vec<Box<dyn rusqlite::ToSql + Send>>) = if let Some(ref s) = since {
                ("SELECT 'all', COALESCE(SUM(input_tokens),0), COALESCE(SUM(output_tokens),0), COUNT(*), COALESCE(SUM(is_error),0), COALESCE(SUM(duration_ms),0) FROM metering_records WHERE created_at >= ?1".into(),
                 vec![Box::new(s.clone()) as Box<dyn rusqlite::ToSql + Send>])
            } else {
                ("SELECT 'all', COALESCE(SUM(input_tokens),0), COALESCE(SUM(output_tokens),0), COUNT(*), COALESCE(SUM(is_error),0), COALESCE(SUM(duration_ms),0) FROM metering_records".into(), vec![])
            };
            let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref() as &dyn rusqlite::ToSql).collect();
            let result = conn.query_row(&sql, params_refs.as_slice(), |row| {
                Ok(MeteringSummary {
                    model: row.get(0)?, total_input_tokens: row.get::<_, i64>(1)? as u64,
                    total_output_tokens: row.get::<_, i64>(2)? as u64, total_requests: row.get::<_, i64>(3)? as u64,
                    total_errors: row.get::<_, i64>(4)? as u64, total_duration_ms: row.get::<_, i64>(5)? as u64,
                })
            })?;
            Ok(result)
        }).await?;
        Ok(row)
    }

    pub fn estimate_cost(summary: &MeteringSummary) -> f64 {
        let pricing = ModelPricing::lookup(&summary.model);
        pricing.estimate_cost(summary.total_input_tokens, summary.total_output_tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_storage() -> MeteringStorage {
        let db = Database::open_in_memory().await.expect("open in-memory db");
        MeteringStorage::new(db)
    }

    #[tokio::test]
    async fn test_record_and_summary_by_model() {
        let storage = setup_storage().await;
        storage.record(MeteringRecord { session_id: "s1".into(), model: "claude-sonnet-4".into(), input_tokens: 1000, output_tokens: 500, duration_ms: 200, is_error: false }).await.expect("record 1");
        storage.record(MeteringRecord { session_id: "s1".into(), model: "claude-sonnet-4".into(), input_tokens: 2000, output_tokens: 800, duration_ms: 300, is_error: false }).await.expect("record 2");
        storage.record(MeteringRecord { session_id: "s2".into(), model: "gpt-4o".into(), input_tokens: 500, output_tokens: 200, duration_ms: 150, is_error: true }).await.expect("record 3");
        let summaries = storage.summary_by_model().await.expect("summary_by_model");
        assert_eq!(summaries.len(), 2);
        let sonnet = summaries.iter().find(|s| s.model == "claude-sonnet-4").expect("sonnet");
        assert_eq!(sonnet.total_input_tokens, 3000);
        assert_eq!(sonnet.total_output_tokens, 1300);
        assert_eq!(sonnet.total_requests, 2);
        assert_eq!(sonnet.total_errors, 0);
        assert_eq!(sonnet.total_duration_ms, 500);
        let gpt = summaries.iter().find(|s| s.model == "gpt-4o").expect("gpt");
        assert_eq!(gpt.total_requests, 1);
        assert_eq!(gpt.total_errors, 1);
    }

    #[tokio::test]
    async fn test_summary_by_session() {
        let storage = setup_storage().await;
        storage.record(MeteringRecord { session_id: "session-a".into(), model: "claude-sonnet-4".into(), input_tokens: 1000, output_tokens: 400, duration_ms: 100, is_error: false }).await.expect("record");
        storage.record(MeteringRecord { session_id: "session-b".into(), model: "claude-sonnet-4".into(), input_tokens: 2000, output_tokens: 800, duration_ms: 200, is_error: false }).await.expect("record");
        let summaries = storage.summary_by_session("session-a".into()).await.expect("summary");
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].total_input_tokens, 1000);
        assert_eq!(summaries[0].total_output_tokens, 400);
    }
}
