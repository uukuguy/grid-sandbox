//! FTS5 Full-text search for knowledge graph

use anyhow::Result;
use rusqlite::{params, Connection};
use std::sync::Arc;

pub struct FtsStore {
    conn: Arc<Connection>,
}

impl FtsStore {
    pub fn new(conn: Arc<Connection>) -> Self {
        Self { conn }
    }

    /// Initialize FTS5 virtual table
    pub fn init(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE VIRTUAL TABLE IF NOT EXISTS kg_fts USING fts5(
                entity_id,
                name,
                entity_type,
                properties,
                content='',
                tokenize='porter unicode61'
            );
            "#,
        )?;
        Ok(())
    }

    /// Index entity
    pub fn index_entity(
        &self,
        entity_id: &str,
        name: &str,
        entity_type: &str,
        properties: &serde_json::Value,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO kg_fts (entity_id, name, entity_type, properties) VALUES (?1, ?2, ?3, ?4)",
            params![
                entity_id,
                name,
                entity_type,
                serde_json::to_string(properties)?
            ],
        )?;
        Ok(())
    }

    /// Remove from index
    pub fn remove_entity(&self, entity_id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM kg_fts WHERE entity_id = ?1",
            params![entity_id],
        )?;
        Ok(())
    }

    /// Search entities
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT entity_id FROM kg_fts WHERE kg_fts MATCH ?1 LIMIT ?2",
        )?;

        let ids = stmt
            .query_map(params![query, limit as i64], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;

        Ok(ids)
    }

    /// Rebuild index from entities
    pub fn rebuild(
        &self,
        entities: &[(String, String, String, serde_json::Value)],
    ) -> Result<()> {
        self.conn.execute("DELETE FROM kg_fts", [])?;

        for (id, name, etype, props) in entities {
            self.index_entity(id, name, etype, props)?;
        }

        Ok(())
    }
}
