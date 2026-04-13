"""Shared SQLite schema and connection helpers for L4 orchestration.

Mirrors S3.T3 / S3.T2 conventions exactly:
- WAL journal mode for concurrent readers.
- ``foreign_keys`` pragma reapplied on every open (per-connection flag).
- Row factory set so callers can use ``row["col"]`` access.
- Writes must be wrapped in ``BEGIN IMMEDIATE`` (reviewer note C1).
"""

from __future__ import annotations

import os

import aiosqlite

SCHEMA = """
PRAGMA journal_mode=WAL;
PRAGMA foreign_keys=ON;

-- Contract 5 — orchestrated sessions produced by the three-way handshake.
CREATE TABLE IF NOT EXISTS sessions (
    session_id   TEXT PRIMARY KEY,
    intent_id    TEXT,
    skill_id     TEXT,
    runtime_id   TEXT,
    user_id      TEXT,
    status       TEXT NOT NULL
        CHECK(status IN ('created','active','closed','failed')),
    payload_json TEXT NOT NULL,
    created_at   INTEGER NOT NULL,
    closed_at    INTEGER
);

CREATE INDEX IF NOT EXISTS idx_sessions_status
    ON sessions(status, created_at DESC);

-- Session event stream — append-only per-session ordered log.
CREATE TABLE IF NOT EXISTS session_events (
    seq          INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id   TEXT NOT NULL,
    event_type   TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    created_at   INTEGER NOT NULL,
    FOREIGN KEY(session_id) REFERENCES sessions(session_id)
);

CREATE INDEX IF NOT EXISTS idx_session_seq
    ON session_events(session_id, seq);
"""


# Phase 1 Event Engine columns — idempotent migration via ALTER TABLE ADD COLUMN.
# SQLite silently fails on duplicate column names via try/except.
_V2_COLUMNS = [
    "ALTER TABLE session_events ADD COLUMN event_id TEXT",
    "ALTER TABLE session_events ADD COLUMN source TEXT DEFAULT ''",
    "ALTER TABLE session_events ADD COLUMN metadata_json TEXT DEFAULT '{}'",
    "ALTER TABLE session_events ADD COLUMN cluster_id TEXT",
]

_V2_FTS = """
CREATE VIRTUAL TABLE IF NOT EXISTS session_events_fts USING fts5(
    event_type, payload_json,
    content='session_events', content_rowid='seq'
);
"""

_V2_FTS_TRIGGER = """
CREATE TRIGGER IF NOT EXISTS session_events_fts_ai
AFTER INSERT ON session_events BEGIN
    INSERT INTO session_events_fts(rowid, event_type, payload_json)
    VALUES (new.seq, new.event_type, new.payload_json);
END;
"""

_V2_INDEX = """
CREATE INDEX IF NOT EXISTS idx_session_events_cluster
    ON session_events(cluster_id) WHERE cluster_id IS NOT NULL;
"""

_V2_EVENT_ID_INDEX = """
CREATE INDEX IF NOT EXISTS idx_session_events_event_id
    ON session_events(event_id) WHERE event_id IS NOT NULL;
"""


async def init_db(path: str) -> None:
    """Create schema if absent. Ensures the parent directory exists."""
    parent = os.path.dirname(os.path.abspath(path))
    if parent:
        os.makedirs(parent, exist_ok=True)
    async with aiosqlite.connect(path) as db:
        await db.executescript(SCHEMA)
        await db.commit()
        # Phase 1 migration — add new columns (idempotent).
        for stmt in _V2_COLUMNS:
            try:
                await db.execute(stmt)
            except Exception:
                pass  # Column already exists
        await db.commit()
        # FTS5 + trigger + indices
        await db.executescript(_V2_FTS)
        await db.executescript(_V2_FTS_TRIGGER)
        await db.executescript(_V2_INDEX)
        await db.executescript(_V2_EVENT_ID_INDEX)
        await db.commit()


async def connect(path: str) -> aiosqlite.Connection:
    """Open a connection with row factory set and pragmas applied."""
    db = await aiosqlite.connect(path)
    db.row_factory = aiosqlite.Row
    # Defense-in-depth: reapply foreign_keys on every connection (per-conn flag).
    await db.execute("PRAGMA foreign_keys=ON")
    # M4 (reviewer): wait up to 5s on SQLITE_BUSY instead of failing immediately —
    # avoids spurious errors when /sessions/create and /sessions/{id}/message
    # race on WAL write locks. Matches the remediation recommended for
    # S3.T2 / S3.T3 as well (tracked as D30).
    await db.execute("PRAGMA busy_timeout=5000")
    return db
