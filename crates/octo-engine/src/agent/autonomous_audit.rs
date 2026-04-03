//! Autonomous mode audit logging (Phase AQ-T6).
//!
//! Provides structured audit entries for autonomous agent actions.
//! Entries are self-contained and can be serialized/stored independently
//! of the generic AuditRecord system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Autonomous audit event types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event")]
pub enum AutonomousAuditEvent {
    Started { config_summary: String },
    TickCompleted { round: u32, tokens_used: u64, cost_usd: f64 },
    Paused { reason: String },
    Resumed,
    BudgetExhausted { limit: String, value: String },
    UserPresenceChanged { online: bool },
    Completed { total_rounds: u32, total_tokens: u64, total_cost_usd: f64 },
    Failed { error: String },
}

impl AutonomousAuditEvent {
    /// Event type name for categorization.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Started { .. } => "started",
            Self::TickCompleted { .. } => "tick_completed",
            Self::Paused { .. } => "paused",
            Self::Resumed => "resumed",
            Self::BudgetExhausted { .. } => "budget_exhausted",
            Self::UserPresenceChanged { .. } => "user_presence_changed",
            Self::Completed { .. } => "completed",
            Self::Failed { .. } => "failed",
        }
    }
}

/// A single autonomous audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousAuditEntry {
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    pub event: AutonomousAuditEvent,
}

impl AutonomousAuditEntry {
    pub fn new(session_id: &str, event: AutonomousAuditEvent) -> Self {
        Self {
            timestamp: Utc::now(),
            session_id: session_id.to_string(),
            event,
        }
    }

    /// Format as a generic audit event type string.
    pub fn event_type(&self) -> String {
        format!("autonomous.{}", self.event.name())
    }

    /// Convert to JSON for generic storage.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }
}

/// In-memory audit log for autonomous mode sessions.
///
/// Collects all audit entries for a single autonomous session run.
/// Can be queried or serialized after the run completes.
/// Use `flush_to_audit_storage()` (AU-D5) to persist entries to SQLite.
#[derive(Debug, Default)]
pub struct AutonomousAuditLog {
    entries: Vec<AutonomousAuditEntry>,
}

impl AutonomousAuditLog {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    pub fn record(&mut self, session_id: &str, event: AutonomousAuditEvent) {
        self.entries.push(AutonomousAuditEntry::new(session_id, event));
    }

    pub fn entries(&self) -> &[AutonomousAuditEntry] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Export all entries as a JSON array.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(&self.entries).unwrap_or_default()
    }

    /// Persist all entries to the existing AuditStorage (AU-D5).
    ///
    /// Converts each `AutonomousAuditEntry` into a generic `AuditEvent`
    /// and writes it to the `audit_logs` SQLite table with hash chaining.
    /// Returns the number of entries successfully persisted.
    pub fn flush_to_audit_storage(
        &self,
        storage: &crate::audit::AuditStorage,
    ) -> usize {
        let mut persisted = 0;
        for entry in &self.entries {
            let audit_event = crate::audit::AuditEvent {
                event_type: entry.event_type(),
                user_id: None,
                session_id: Some(entry.session_id.clone()),
                resource_id: None,
                action: entry.event.name().to_string(),
                result: "ok".to_string(),
                metadata: Some(entry.to_json()),
                ip_address: None,
            };
            if storage.log(audit_event).is_ok() {
                persisted += 1;
            }
        }
        persisted
    }

    /// Clear all entries after flushing.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_event_names() {
        assert_eq!(AutonomousAuditEvent::Started { config_summary: "test".into() }.name(), "started");
        assert_eq!(AutonomousAuditEvent::TickCompleted { round: 1, tokens_used: 100, cost_usd: 0.01 }.name(), "tick_completed");
        assert_eq!(AutonomousAuditEvent::Paused { reason: "user".into() }.name(), "paused");
        assert_eq!(AutonomousAuditEvent::Resumed.name(), "resumed");
        assert_eq!(AutonomousAuditEvent::BudgetExhausted { limit: "rounds".into(), value: "10".into() }.name(), "budget_exhausted");
        assert_eq!(AutonomousAuditEvent::UserPresenceChanged { online: true }.name(), "user_presence_changed");
        assert_eq!(AutonomousAuditEvent::Completed { total_rounds: 5, total_tokens: 500, total_cost_usd: 0.05 }.name(), "completed");
        assert_eq!(AutonomousAuditEvent::Failed { error: "timeout".into() }.name(), "failed");
    }

    #[test]
    fn test_audit_entry_creation() {
        let entry = AutonomousAuditEntry::new("sess-1", AutonomousAuditEvent::Started {
            config_summary: "max_rounds=10, idle_sleep=30s".into(),
        });
        assert_eq!(entry.session_id, "sess-1");
        assert!(matches!(entry.event, AutonomousAuditEvent::Started { .. }));
        assert_eq!(entry.event_type(), "autonomous.started");
    }

    #[test]
    fn test_audit_entry_json() {
        let entry = AutonomousAuditEntry::new("sess-2", AutonomousAuditEvent::TickCompleted {
            round: 3,
            tokens_used: 1500,
            cost_usd: 0.015,
        });
        let json = entry.to_json();
        assert_eq!(json["session_id"], "sess-2");
        assert!(json["event"]["round"].is_number());
    }

    #[test]
    fn test_audit_event_serialization() {
        let event = AutonomousAuditEvent::Completed {
            total_rounds: 10,
            total_tokens: 5000,
            total_cost_usd: 0.50,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("Completed"));
        assert!(json.contains("5000"));
    }

    #[test]
    fn test_audit_log_collect_and_export() {
        let mut log = AutonomousAuditLog::new();
        assert!(log.is_empty());

        log.record("sess-1", AutonomousAuditEvent::Started { config_summary: "test".into() });
        log.record("sess-1", AutonomousAuditEvent::TickCompleted { round: 1, tokens_used: 100, cost_usd: 0.01 });
        log.record("sess-1", AutonomousAuditEvent::Completed { total_rounds: 1, total_tokens: 100, total_cost_usd: 0.01 });

        assert_eq!(log.len(), 3);
        assert!(!log.is_empty());

        let json = log.to_json();
        assert!(json.is_array());
        assert_eq!(json.as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_user_presence_audit() {
        let entry = AutonomousAuditEntry::new("sess-3", AutonomousAuditEvent::UserPresenceChanged {
            online: false,
        });
        assert_eq!(entry.event_type(), "autonomous.user_presence_changed");
        let json = entry.to_json();
        assert_eq!(json["event"]["online"], false);
    }

    #[test]
    fn test_flush_to_audit_storage() {
        use rusqlite::Connection;

        // Setup in-memory AuditStorage
        let conn = Connection::open_in_memory().unwrap();
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
        .unwrap();
        // Safety: AuditStorage wraps Connection; construct via field access in test
        let storage = crate::audit::AuditStorage::from_conn(conn);

        let mut log = AutonomousAuditLog::new();
        log.record("s1", AutonomousAuditEvent::Started { config_summary: "test".into() });
        log.record("s1", AutonomousAuditEvent::TickCompleted { round: 1, tokens_used: 100, cost_usd: 0.01 });
        log.record("s1", AutonomousAuditEvent::Completed { total_rounds: 1, total_tokens: 100, total_cost_usd: 0.01 });

        let persisted = log.flush_to_audit_storage(&storage);
        assert_eq!(persisted, 3);

        // Verify in AuditStorage
        let records = storage.query(Some("autonomous.started"), None, 10, 0).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].session_id, Some("s1".to_string()));

        // Test clear
        log.clear();
        assert!(log.is_empty());
    }
}
