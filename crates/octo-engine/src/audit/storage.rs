use chrono::Utc;
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::path::Path;

pub struct AuditStorage {
    conn: Connection,
}

pub struct AuditEvent {
    pub event_type: String,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub resource_id: Option<String>,
    pub action: String,
    pub result: String,
    pub metadata: Option<serde_json::Value>,
    pub ip_address: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AuditRecord {
    pub id: i64,
    pub timestamp: String,
    pub event_type: String,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub resource_id: Option<String>,
    pub action: String,
    pub result: String,
    pub metadata: Option<String>,
    pub ip_address: Option<String>,
    pub prev_hash: String,
    pub hash: String,
}

/// Result of verifying the audit hash chain integrity.
#[derive(Debug, Clone)]
pub struct ChainVerifyResult {
    /// Whether the entire checked range is valid.
    pub valid: bool,
    /// The id of the first record where the chain is broken, if any.
    pub broken_at: Option<i64>,
    /// Number of records checked.
    pub records_checked: usize,
}

impl AuditStorage {
    pub fn new(db_path: &Path) -> rusqlite::Result<Self> {
        let conn = Connection::open(db_path)?;
        Ok(Self { conn })
    }

    /// Compute SHA-256 hash for a chain link.
    pub fn compute_hash(
        prev_hash: &str,
        timestamp: &str,
        event_type: &str,
        action: &str,
        result: &str,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(prev_hash.as_bytes());
        hasher.update(timestamp.as_bytes());
        hasher.update(event_type.as_bytes());
        hasher.update(action.as_bytes());
        hasher.update(result.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Get the hash of the most recent audit record (empty string if no records).
    fn last_hash(&self) -> rusqlite::Result<String> {
        self.conn
            .query_row(
                "SELECT hash FROM audit_logs ORDER BY id DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(String::new()),
                other => Err(other),
            })
    }

    pub fn log(&self, event: AuditEvent) -> rusqlite::Result<i64> {
        let metadata_str = event.metadata.as_ref().map(|m| m.to_string());
        let prev_hash = self.last_hash()?;
        let timestamp = Utc::now().to_rfc3339();
        let hash = Self::compute_hash(
            &prev_hash,
            &timestamp,
            &event.event_type,
            &event.action,
            &event.result,
        );

        self.conn.execute(
            "INSERT INTO audit_logs (timestamp, event_type, user_id, session_id, resource_id, action, result, metadata, ip_address, prev_hash, hash) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                timestamp,
                event.event_type,
                event.user_id,
                event.session_id,
                event.resource_id,
                event.action,
                event.result,
                metadata_str,
                event.ip_address,
                prev_hash,
                hash
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Verify the hash chain integrity for records in [from_id, to_id].
    pub fn verify_chain(&self, from_id: i64, to_id: i64) -> rusqlite::Result<ChainVerifyResult> {
        let mut stmt = self.conn.prepare(
            "SELECT id, prev_hash, hash, timestamp, event_type, action, result FROM audit_logs WHERE id >= ? AND id <= ? ORDER BY id ASC",
        )?;

        let rows: Vec<(i64, String, String, String, String, String, String)> = stmt
            .query_map(rusqlite::params![from_id, to_id], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        if rows.is_empty() {
            return Ok(ChainVerifyResult {
                valid: true,
                broken_at: None,
                records_checked: 0,
            });
        }

        let mut records_checked = 0;

        for (i, (id, prev_hash, stored_hash, timestamp, event_type, action, result)) in
            rows.iter().enumerate()
        {
            records_checked += 1;

            // Verify this record's hash matches recomputation
            let expected_hash =
                Self::compute_hash(prev_hash, timestamp, event_type, action, result);
            if *stored_hash != expected_hash {
                return Ok(ChainVerifyResult {
                    valid: false,
                    broken_at: Some(*id),
                    records_checked,
                });
            }

            // Verify chain linkage: this record's prev_hash should match previous record's hash
            if i > 0 {
                let (_, _, prev_record_hash, _, _, _, _) = &rows[i - 1];
                if *prev_hash != *prev_record_hash {
                    return Ok(ChainVerifyResult {
                        valid: false,
                        broken_at: Some(*id),
                        records_checked,
                    });
                }
            }
        }

        Ok(ChainVerifyResult {
            valid: true,
            broken_at: None,
            records_checked,
        })
    }

    pub fn query(
        &self,
        event_type: Option<&str>,
        user_id: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> rusqlite::Result<Vec<AuditRecord>> {
        let mut sql = String::from("SELECT id, timestamp, event_type, user_id, session_id, resource_id, action, result, metadata, ip_address, prev_hash, hash FROM audit_logs WHERE 1=1");

        if event_type.is_some() {
            sql.push_str(" AND event_type = ?");
        }
        if user_id.is_some() {
            sql.push_str(" AND user_id = ?");
        }

        sql.push_str(" ORDER BY timestamp DESC LIMIT ? OFFSET ?");

        let mut stmt = self.conn.prepare(&sql)?;

        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(t) = event_type {
            params.push(Box::new(t.to_string()));
        }
        if let Some(u) = user_id {
            params.push(Box::new(u.to_string()));
        }
        params.push(Box::new(limit as i64));
        params.push(Box::new(offset as i64));

        let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let rows = stmt.query_map(params_refs.as_slice(), |row| {
            Ok(AuditRecord {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                event_type: row.get(2)?,
                user_id: row.get(3)?,
                session_id: row.get(4)?,
                resource_id: row.get(5)?,
                action: row.get(6)?,
                result: row.get(7)?,
                metadata: row.get(8)?,
                ip_address: row.get(9)?,
                prev_hash: row.get(10)?,
                hash: row.get(11)?,
            })
        })?;

        rows.collect()
    }

    pub fn count(&self, event_type: Option<&str>, user_id: Option<&str>) -> rusqlite::Result<i64> {
        let mut sql = String::from("SELECT COUNT(*) FROM audit_logs WHERE 1=1");

        if event_type.is_some() {
            sql.push_str(" AND event_type = ?");
        }
        if user_id.is_some() {
            sql.push_str(" AND user_id = ?");
        }

        let mut stmt = self.conn.prepare(&sql)?;

        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(t) = event_type {
            params.push(Box::new(t.to_string()));
        }
        if let Some(u) = user_id {
            params.push(Box::new(u.to_string()));
        }

        let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        stmt.query_row(params_refs.as_slice(), |row| row.get(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_storage() -> AuditStorage {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS audit_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL DEFAULT (datetime('now')),
                event_type TEXT NOT NULL,
                user_id TEXT,
                session_id TEXT,
                resource_id TEXT,
                action TEXT NOT NULL,
                result TEXT NOT NULL,
                metadata TEXT,
                ip_address TEXT,
                prev_hash TEXT NOT NULL DEFAULT '',
                hash TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_audit_hash ON audit_logs(hash);",
        )
        .expect("create table");
        AuditStorage { conn }
    }

    fn make_event(event_type: &str, action: &str, result: &str) -> AuditEvent {
        AuditEvent {
            event_type: event_type.to_string(),
            user_id: Some("user1".to_string()),
            session_id: None,
            resource_id: None,
            action: action.to_string(),
            result: result.to_string(),
            metadata: None,
            ip_address: None,
        }
    }

    #[test]
    fn test_chained_insert() {
        let storage = setup_storage();

        let id1 = storage
            .log(make_event("auth", "login", "success"))
            .expect("log 1");
        let id2 = storage
            .log(make_event("auth", "logout", "success"))
            .expect("log 2");
        let id3 = storage
            .log(make_event("tool", "execute", "ok"))
            .expect("log 3");

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);

        // Query all records and verify chain connectivity
        let records = storage.query(None, None, 10, 0).expect("query");
        assert_eq!(records.len(), 3);

        // Records are returned DESC, so reverse for chain order
        let mut sorted = records.clone();
        sorted.sort_by_key(|r| r.id);

        // First record's prev_hash should be empty
        assert_eq!(sorted[0].prev_hash, "");
        // Second record's prev_hash should equal first record's hash
        assert_eq!(sorted[1].prev_hash, sorted[0].hash);
        // Third record's prev_hash should equal second record's hash
        assert_eq!(sorted[2].prev_hash, sorted[1].hash);

        // All hashes should be non-empty
        for r in &sorted {
            assert!(!r.hash.is_empty(), "hash should not be empty for id {}", r.id);
        }
    }

    #[test]
    fn test_verify_chain_valid() {
        let storage = setup_storage();

        storage.log(make_event("auth", "login", "success")).expect("log 1");
        storage.log(make_event("auth", "logout", "success")).expect("log 2");
        storage.log(make_event("tool", "execute", "ok")).expect("log 3");

        let result = storage.verify_chain(1, 3).expect("verify");
        assert!(result.valid);
        assert!(result.broken_at.is_none());
        assert_eq!(result.records_checked, 3);
    }

    #[test]
    fn test_verify_chain_tampered() {
        let storage = setup_storage();

        storage.log(make_event("auth", "login", "success")).expect("log 1");
        storage.log(make_event("auth", "logout", "success")).expect("log 2");
        storage.log(make_event("tool", "execute", "ok")).expect("log 3");

        // Tamper with the middle record's action field
        storage
            .conn
            .execute(
                "UPDATE audit_logs SET action = 'tampered_action' WHERE id = 2",
                [],
            )
            .expect("tamper");

        let result = storage.verify_chain(1, 3).expect("verify");
        assert!(!result.valid);
        assert_eq!(result.broken_at, Some(2));
    }

    #[test]
    fn test_empty_chain() {
        let storage = setup_storage();

        let result = storage.verify_chain(1, 100).expect("verify");
        assert!(result.valid);
        assert!(result.broken_at.is_none());
        assert_eq!(result.records_checked, 0);
    }
}
