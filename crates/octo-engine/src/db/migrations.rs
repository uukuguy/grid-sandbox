use rusqlite::Connection;
use tracing::info;

const CURRENT_VERSION: u32 = 2;

const MIGRATION_V1: &str = "
-- Working Memory blocks persistence
CREATE TABLE IF NOT EXISTS memory_blocks (
    id          TEXT NOT NULL,
    user_id     TEXT NOT NULL,
    sandbox_id  TEXT NOT NULL,
    label       TEXT NOT NULL,
    value       TEXT NOT NULL DEFAULT '',
    priority    INTEGER NOT NULL DEFAULT 128,
    max_age_turns INTEGER,
    last_updated_turn INTEGER NOT NULL DEFAULT 0,
    char_limit  INTEGER NOT NULL DEFAULT 2000,
    is_readonly INTEGER NOT NULL DEFAULT 0,
    updated_at  INTEGER NOT NULL DEFAULT (strftime('%s','now')),
    PRIMARY KEY (id, user_id, sandbox_id)
);

-- Session metadata
CREATE TABLE IF NOT EXISTS sessions (
    session_id  TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL,
    sandbox_id  TEXT NOT NULL,
    created_at  INTEGER NOT NULL DEFAULT (strftime('%s','now'))
);

-- Session messages
CREATE TABLE IF NOT EXISTS session_messages (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id  TEXT NOT NULL,
    role        TEXT NOT NULL,
    content_json TEXT NOT NULL,
    created_at  INTEGER NOT NULL DEFAULT (strftime('%s','now'))
);

CREATE INDEX IF NOT EXISTS idx_session_messages_session_id
    ON session_messages(session_id);

-- Persistent Memory (Layer 2)
CREATE TABLE IF NOT EXISTS memories (
    id          TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL,
    sandbox_id  TEXT NOT NULL DEFAULT '',
    category    TEXT NOT NULL,
    content     TEXT NOT NULL,
    metadata    TEXT NOT NULL DEFAULT '{}',
    embedding   BLOB,
    importance  REAL NOT NULL DEFAULT 0.5,
    access_count INTEGER NOT NULL DEFAULT 0,
    accessed_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
    source_type TEXT NOT NULL DEFAULT 'manual',
    source_ref  TEXT NOT NULL DEFAULT '',
    ttl         INTEGER,
    created_at  INTEGER NOT NULL DEFAULT (strftime('%s','now')),
    updated_at  INTEGER NOT NULL DEFAULT (strftime('%s','now'))
);

CREATE INDEX IF NOT EXISTS idx_memories_user_id ON memories(user_id);
CREATE INDEX IF NOT EXISTS idx_memories_category ON memories(category);
CREATE INDEX IF NOT EXISTS idx_memories_created_at ON memories(created_at);

-- FTS5 virtual table for full-text search
CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
    content,
    category,
    content=memories,
    content_rowid=rowid,
    tokenize='porter unicode61'
);

-- FTS5 sync triggers
CREATE TRIGGER IF NOT EXISTS memories_ai AFTER INSERT ON memories BEGIN
    INSERT INTO memories_fts(rowid, content, category)
    VALUES (NEW.rowid, NEW.content, NEW.category);
END;

CREATE TRIGGER IF NOT EXISTS memories_ad AFTER DELETE ON memories BEGIN
    INSERT INTO memories_fts(memories_fts, rowid, content, category)
    VALUES ('delete', OLD.rowid, OLD.content, OLD.category);
END;

CREATE TRIGGER IF NOT EXISTS memories_au AFTER UPDATE ON memories BEGIN
    INSERT INTO memories_fts(memories_fts, rowid, content, category)
    VALUES ('delete', OLD.rowid, OLD.content, OLD.category);
    INSERT INTO memories_fts(rowid, content, category)
    VALUES (NEW.rowid, NEW.content, NEW.category);
END;
";

const MIGRATION_V2: &str = "
-- Tool execution records
CREATE TABLE IF NOT EXISTS tool_executions (
    id          TEXT PRIMARY KEY,
    session_id  TEXT NOT NULL,
    tool_name   TEXT NOT NULL,
    source      TEXT NOT NULL,
    input       TEXT NOT NULL,
    output      TEXT,
    status      TEXT NOT NULL DEFAULT 'running',
    started_at  INTEGER NOT NULL,
    duration_ms INTEGER,
    error       TEXT,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_tool_executions_session
    ON tool_executions(session_id);
CREATE INDEX IF NOT EXISTS idx_tool_executions_tool
    ON tool_executions(tool_name);
CREATE INDEX IF NOT EXISTS idx_tool_executions_started
    ON tool_executions(started_at DESC);
";

pub fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    let version: u32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

    if version < CURRENT_VERSION {
        info!(from = version, to = CURRENT_VERSION, "Running database migration");

        if version < 1 {
            conn.execute_batch(MIGRATION_V1)?;
            info!("Applied migration v1");
        }

        if version < 2 {
            conn.execute_batch(MIGRATION_V2)?;
            info!("Applied migration v2");
        }

        conn.pragma_update(None, "user_version", CURRENT_VERSION)?;
        info!("Migration to v{CURRENT_VERSION} complete");
    }

    Ok(())
}
