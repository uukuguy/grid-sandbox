use anyhow::Result;
use tokio_rusqlite::Connection;
use tracing::info;

use super::migrations;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub async fn open(path: &str) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let conn = Connection::open(path).await?;
        let db = Self { conn };
        db.apply_pragmas().await?;
        db.run_migrations().await?;
        info!("Database opened: {path}");
        Ok(db)
    }

    pub async fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().await?;
        let db = Self { conn };
        db.apply_pragmas().await?;
        db.run_migrations().await?;
        info!("In-memory database opened");
        Ok(db)
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    async fn apply_pragmas(&self) -> Result<()> {
        self.conn
            .call(|conn| {
                conn.execute_batch(
                    "PRAGMA journal_mode = WAL;
                     PRAGMA synchronous = NORMAL;
                     PRAGMA foreign_keys = ON;
                     PRAGMA busy_timeout = 5000;",
                )?;
                Ok(())
            })
            .await?;
        Ok(())
    }

    async fn run_migrations(&self) -> Result<()> {
        self.conn
            .call(|conn| {
                migrations::migrate(conn)?;
                Ok(())
            })
            .await?;
        Ok(())
    }
}
