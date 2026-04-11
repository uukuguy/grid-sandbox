"""Shared SQLite schema and connection helpers."""

from __future__ import annotations

import aiosqlite

SCHEMA = """
PRAGMA journal_mode=WAL;

CREATE TABLE IF NOT EXISTS anchors (
    anchor_id      TEXT PRIMARY KEY,
    event_id       TEXT NOT NULL,
    session_id     TEXT NOT NULL,
    type           TEXT NOT NULL,
    data_ref       TEXT,
    snapshot_hash  TEXT,
    source_system  TEXT,
    tool_version   TEXT,
    model_version  TEXT,
    rule_version   TEXT,
    created_at     INTEGER NOT NULL,
    metadata       TEXT
);

CREATE INDEX IF NOT EXISTS idx_anchors_event_id   ON anchors(event_id);
CREATE INDEX IF NOT EXISTS idx_anchors_session_id ON anchors(session_id);

-- M3: DB-enforced append-only invariant on anchors.
CREATE TRIGGER IF NOT EXISTS anchors_no_update
BEFORE UPDATE ON anchors
BEGIN SELECT RAISE(ABORT, 'anchors are append-only'); END;

CREATE TRIGGER IF NOT EXISTS anchors_no_delete
BEFORE DELETE ON anchors
BEGIN SELECT RAISE(ABORT, 'anchors are append-only'); END;

CREATE TABLE IF NOT EXISTS memory_files (
    memory_id      TEXT NOT NULL,
    version        INTEGER NOT NULL,
    scope          TEXT NOT NULL,
    category       TEXT NOT NULL,
    content        TEXT NOT NULL,
    evidence_refs  TEXT,
    status         TEXT NOT NULL CHECK(status IN ('agent_suggested','confirmed','archived')),
    created_at     INTEGER NOT NULL,
    updated_at     INTEGER NOT NULL,
    PRIMARY KEY (memory_id, version)
);

CREATE INDEX IF NOT EXISTS idx_memory_files_scope    ON memory_files(scope);
CREATE INDEX IF NOT EXISTS idx_memory_files_category ON memory_files(category);
CREATE INDEX IF NOT EXISTS idx_memory_files_status   ON memory_files(status);

CREATE VIRTUAL TABLE IF NOT EXISTS memory_fts USING fts5(
    memory_id UNINDEXED,
    version   UNINDEXED,
    content_text,
    category,
    scope,
    tokenize = 'unicode61'
);
"""


async def init_db(path: str) -> None:
    """Create schema if absent."""
    async with aiosqlite.connect(path) as db:
        await db.executescript(SCHEMA)
        await db.commit()


async def connect(path: str) -> aiosqlite.Connection:
    """Open a connection with row factory set."""
    db = await aiosqlite.connect(path)
    db.row_factory = aiosqlite.Row
    return db
