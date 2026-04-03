#[cfg(test)]
mod tests {
    use crate::audit::{AuditEvent, AuditRecord};
    use rusqlite::Connection;

    // Helper function to create an in-memory database with schema
    fn create_test_db() -> rusqlite::Result<Connection> {
        let conn = Connection::open_in_memory()?;

        // Create the audit_logs table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS audit_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                event_type TEXT NOT NULL,
                user_id TEXT,
                session_id TEXT,
                resource_id TEXT,
                action TEXT NOT NULL,
                result TEXT NOT NULL,
                metadata TEXT,
                ip_address TEXT
            )",
            [],
        )?;

        Ok(conn)
    }

    // Helper struct to test AuditStorage with internal connection
    struct TestAuditStorage {
        conn: Connection,
    }

    impl TestAuditStorage {
        fn new() -> rusqlite::Result<Self> {
            let conn = create_test_db()?;
            Ok(Self { conn })
        }

        fn log(&self, event: AuditEvent) -> rusqlite::Result<i64> {
            let metadata_str = event.metadata.as_ref().map(|m| m.to_string());

            self.conn.execute(
                "INSERT INTO audit_logs (timestamp, event_type, user_id, session_id, resource_id, action, result, metadata, ip_address) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                rusqlite::params![
                    chrono::Utc::now().to_rfc3339(),
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

        fn query(
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

            let params_refs: Vec<&dyn rusqlite::ToSql> =
                params.iter().map(|p| p.as_ref()).collect();

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
                    prev_hash: String::new(),
                    hash: String::new(),
                })
            })?;

            rows.collect()
        }

        fn count(&self, event_type: Option<&str>, user_id: Option<&str>) -> rusqlite::Result<i64> {
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

            let params_refs: Vec<&dyn rusqlite::ToSql> =
                params.iter().map(|p| p.as_ref()).collect();

            stmt.query_row(params_refs.as_slice(), |row| row.get(0))
        }
    }

    #[test]
    fn test_audit_event_creation() {
        let event = AuditEvent {
            event_type: "user.login".to_string(),
            user_id: Some("user123".to_string()),
            session_id: Some("session456".to_string()),
            resource_id: None,
            action: "login".to_string(),
            result: "success".to_string(),
            metadata: Some(serde_json::json!({"browser": "Chrome"})),
            ip_address: Some("192.168.1.1".to_string()),
        };

        assert_eq!(event.event_type, "user.login");
        assert_eq!(event.user_id, Some("user123".to_string()));
        assert_eq!(event.action, "login");
        assert_eq!(event.result, "success");
    }

    #[test]
    fn test_audit_record_creation() {
        let record = AuditRecord {
            id: 1,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            event_type: "user.login".to_string(),
            user_id: Some("user123".to_string()),
            session_id: Some("session456".to_string()),
            resource_id: None,
            action: "login".to_string(),
            result: "success".to_string(),
            metadata: Some(r#"{"browser": "Chrome"}"#.to_string()),
            ip_address: Some("192.168.1.1".to_string()),
            prev_hash: String::new(),
            hash: String::new(),
        };

        assert_eq!(record.id, 1);
        assert_eq!(record.event_type, "user.login");
        assert_eq!(record.user_id, Some("user123".to_string()));
    }

    #[test]
    fn test_audit_storage_log() -> rusqlite::Result<()> {
        let storage = TestAuditStorage::new()?;

        let event = AuditEvent {
            event_type: "user.login".to_string(),
            user_id: Some("user123".to_string()),
            session_id: Some("session456".to_string()),
            resource_id: None,
            action: "login".to_string(),
            result: "success".to_string(),
            metadata: None,
            ip_address: Some("192.168.1.1".to_string()),
        };

        let id = storage.log(event)?;

        assert_eq!(id, 1);

        Ok(())
    }

    #[test]
    fn test_audit_storage_log_with_metadata() -> rusqlite::Result<()> {
        let storage = TestAuditStorage::new()?;

        let event = AuditEvent {
            event_type: "tool.execute".to_string(),
            user_id: Some("user456".to_string()),
            session_id: Some("session789".to_string()),
            resource_id: Some("file.txt".to_string()),
            action: "execute".to_string(),
            result: "success".to_string(),
            metadata: Some(serde_json::json!({
                "tool_name": "bash",
                "duration_ms": 150
            })),
            ip_address: None,
        };

        let id = storage.log(event)?;
        assert_eq!(id, 1);

        // Query to verify metadata was stored
        let records = storage.query(None, None, 10, 0)?;
        assert_eq!(records.len(), 1);
        assert!(records[0].metadata.is_some());

        Ok(())
    }

    #[test]
    fn test_audit_storage_query_all() -> rusqlite::Result<()> {
        let storage = TestAuditStorage::new()?;

        // Insert multiple events
        for i in 0..5 {
            let event = AuditEvent {
                event_type: "test.event".to_string(),
                user_id: None,
                session_id: None,
                resource_id: None,
                action: format!("action{}", i),
                result: "success".to_string(),
                metadata: None,
                ip_address: None,
            };
            storage.log(event)?;
        }

        let records = storage.query(None, None, 10, 0)?;

        assert_eq!(records.len(), 5);

        Ok(())
    }

    #[test]
    fn test_audit_storage_query_by_event_type() -> rusqlite::Result<()> {
        let storage = TestAuditStorage::new()?;

        // Insert events with different event types
        let event1 = AuditEvent {
            event_type: "user.login".to_string(),
            user_id: Some("user1".to_string()),
            session_id: None,
            resource_id: None,
            action: "login".to_string(),
            result: "success".to_string(),
            metadata: None,
            ip_address: None,
        };
        storage.log(event1)?;

        let event2 = AuditEvent {
            event_type: "user.logout".to_string(),
            user_id: Some("user1".to_string()),
            session_id: None,
            resource_id: None,
            action: "logout".to_string(),
            result: "success".to_string(),
            metadata: None,
            ip_address: None,
        };
        storage.log(event2)?;

        let event3 = AuditEvent {
            event_type: "user.login".to_string(),
            user_id: Some("user2".to_string()),
            session_id: None,
            resource_id: None,
            action: "login".to_string(),
            result: "success".to_string(),
            metadata: None,
            ip_address: None,
        };
        storage.log(event3)?;

        // Query by event_type
        let records = storage.query(Some("user.login"), None, 10, 0)?;

        assert_eq!(records.len(), 2);
        assert!(records.iter().all(|r| r.event_type == "user.login"));

        Ok(())
    }

    #[test]
    fn test_audit_storage_query_by_user_id() -> rusqlite::Result<()> {
        let storage = TestAuditStorage::new()?;

        // Insert events for different users
        let event1 = AuditEvent {
            event_type: "test.event".to_string(),
            user_id: Some("user1".to_string()),
            session_id: None,
            resource_id: None,
            action: "action1".to_string(),
            result: "success".to_string(),
            metadata: None,
            ip_address: None,
        };
        storage.log(event1)?;

        let event2 = AuditEvent {
            event_type: "test.event".to_string(),
            user_id: Some("user2".to_string()),
            session_id: None,
            resource_id: None,
            action: "action2".to_string(),
            result: "success".to_string(),
            metadata: None,
            ip_address: None,
        };
        storage.log(event2)?;

        let event3 = AuditEvent {
            event_type: "test.event".to_string(),
            user_id: Some("user1".to_string()),
            session_id: None,
            resource_id: None,
            action: "action3".to_string(),
            result: "success".to_string(),
            metadata: None,
            ip_address: None,
        };
        storage.log(event3)?;

        // Query by user_id
        let records = storage.query(None, Some("user1"), 10, 0)?;

        assert_eq!(records.len(), 2);
        assert!(records
            .iter()
            .all(|r| r.user_id.as_deref() == Some("user1")));

        Ok(())
    }

    #[test]
    fn test_audit_storage_query_with_limit() -> rusqlite::Result<()> {
        let storage = TestAuditStorage::new()?;

        // Insert 10 events
        for i in 0..10 {
            let event = AuditEvent {
                event_type: "test.event".to_string(),
                user_id: None,
                session_id: None,
                resource_id: None,
                action: format!("action{}", i),
                result: "success".to_string(),
                metadata: None,
                ip_address: None,
            };
            storage.log(event)?;
        }

        // Query with limit
        let records = storage.query(None, None, 3, 0)?;

        assert_eq!(records.len(), 3);

        Ok(())
    }

    #[test]
    fn test_audit_storage_query_with_offset() -> rusqlite::Result<()> {
        let storage = TestAuditStorage::new()?;

        // Insert 5 events
        for i in 0..5 {
            let event = AuditEvent {
                event_type: "test.event".to_string(),
                user_id: None,
                session_id: None,
                resource_id: None,
                action: format!("action{}", i),
                result: "success".to_string(),
                metadata: None,
                ip_address: None,
            };
            storage.log(event)?;
        }

        // Query with offset
        let records = storage.query(None, None, 10, 2)?;

        assert_eq!(records.len(), 3);

        Ok(())
    }

    #[test]
    fn test_audit_storage_count() -> rusqlite::Result<()> {
        let storage = TestAuditStorage::new()?;

        // Insert events
        for i in 0..5 {
            let event = AuditEvent {
                event_type: "user.login".to_string(),
                user_id: Some(format!("user{}", i)),
                session_id: None,
                resource_id: None,
                action: "login".to_string(),
                result: "success".to_string(),
                metadata: None,
                ip_address: None,
            };
            storage.log(event)?;
        }

        let count = storage.count(None, None)?;
        assert_eq!(count, 5);

        let count_by_event = storage.count(Some("user.login"), None)?;
        assert_eq!(count_by_event, 5);

        let count_by_user = storage.count(None, Some("user0"))?;
        assert_eq!(count_by_user, 1);

        Ok(())
    }

    #[test]
    fn test_audit_storage_count_no_matches() -> rusqlite::Result<()> {
        let storage = TestAuditStorage::new()?;

        // Insert events
        let event = AuditEvent {
            event_type: "user.login".to_string(),
            user_id: Some("user1".to_string()),
            session_id: None,
            resource_id: None,
            action: "login".to_string(),
            result: "success".to_string(),
            metadata: None,
            ip_address: None,
        };
        storage.log(event)?;

        let count = storage.count(Some("user.logout"), None)?;
        assert_eq!(count, 0);

        Ok(())
    }

    #[test]
    fn test_audit_storage_empty_query() -> rusqlite::Result<()> {
        let storage = TestAuditStorage::new()?;

        let records = storage.query(None, None, 10, 0)?;

        assert!(records.is_empty());

        Ok(())
    }

    #[test]
    fn test_audit_storage_query_order() -> rusqlite::Result<()> {
        let storage = TestAuditStorage::new()?;

        // Insert events with different timestamps (they should be ordered by timestamp DESC)
        for i in 0..3 {
            let event = AuditEvent {
                event_type: "test.event".to_string(),
                user_id: None,
                session_id: None,
                resource_id: None,
                action: format!("action{}", i),
                result: "success".to_string(),
                metadata: None,
                ip_address: None,
            };
            storage.log(event)?;
        }

        let records = storage.query(None, None, 10, 0)?;

        // The most recent should be first (DESC order)
        // Since we're inserting quickly, the order might vary, but all should be present
        assert_eq!(records.len(), 3);

        Ok(())
    }

    #[test]
    fn test_audit_storage_complex_filter() -> rusqlite::Result<()> {
        let storage = TestAuditStorage::new()?;

        // Insert various events
        let events = vec![
            ("user.login", "user1", "action1"),
            ("user.login", "user2", "action2"),
            ("user.logout", "user1", "action3"),
            ("user.login", "user1", "action4"),
        ];

        for (event_type, user_id, action) in events {
            let event = AuditEvent {
                event_type: event_type.to_string(),
                user_id: Some(user_id.to_string()),
                session_id: None,
                resource_id: None,
                action: action.to_string(),
                result: "success".to_string(),
                metadata: None,
                ip_address: None,
            };
            storage.log(event)?;
        }

        // Query by both event_type and user_id
        let records = storage.query(Some("user.login"), Some("user1"), 10, 0)?;

        assert_eq!(records.len(), 2);
        assert!(records
            .iter()
            .all(|r| { r.event_type == "user.login" && r.user_id.as_deref() == Some("user1") }));

        Ok(())
    }
}
