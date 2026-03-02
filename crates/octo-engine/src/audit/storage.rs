use chrono::Utc;
use rusqlite::Connection;
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
}

impl AuditStorage {
    pub fn new(db_path: &Path) -> rusqlite::Result<Self> {
        let conn = Connection::open(db_path)?;
        Ok(Self { conn })
    }

    pub fn log(&self, event: AuditEvent) -> rusqlite::Result<i64> {
        let metadata_str = event.metadata.as_ref().map(|m| m.to_string());

        self.conn.execute(
            "INSERT INTO audit_logs (timestamp, event_type, user_id, session_id, resource_id, action, result, metadata, ip_address) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                Utc::now().to_rfc3339(),
                event.event_type,
                event.user_id,
                event.session_id,
                event.resource_id,
                event.action,
                event.result,
                metadata_str,
                event.ip_address
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    pub fn query(
        &self,
        event_type: Option<&str>,
        user_id: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> rusqlite::Result<Vec<AuditRecord>> {
        let mut sql = String::from("SELECT id, timestamp, event_type, user_id, session_id, resource_id, action, result, metadata, ip_address FROM audit_logs WHERE 1=1");

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
