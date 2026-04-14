"""Test FTS5 backfill when migrating from Phase 0.5 to Phase 1.

Phase 0.5 session_events table has no event_id/source/cluster_id/metadata_json
columns, and no FTS5 virtual table. Migration adds these. Without backfill,
pre-existing rows would be invisible to `backend.search()` because the FTS5
trigger only fires on INSERT.

This test proves that `init_db()` rebuilds the FTS5 index so old rows remain
searchable after upgrade.
"""

from __future__ import annotations

import os
import tempfile

import aiosqlite

from eaasp_l4_orchestration.db import init_db
from eaasp_l4_orchestration.event_backend_sqlite import SqliteWalBackend


async def test_phase05_to_phase1_migration_preserves_search() -> None:
    """Old events (pre-migration) must be findable via FTS search after init_db."""
    db_path = tempfile.mktemp(suffix=".db")
    try:
        # Simulate Phase 0.5 DB: basic schema, no Phase 1 columns, no FTS5.
        async with aiosqlite.connect(db_path) as db:
            await db.executescript(
                """
                CREATE TABLE sessions (
                    session_id TEXT PRIMARY KEY,
                    intent_id TEXT,
                    skill_id TEXT,
                    runtime_id TEXT,
                    user_id TEXT,
                    status TEXT,
                    payload_json TEXT,
                    created_at INTEGER,
                    closed_at INTEGER
                );
                CREATE TABLE session_events (
                    seq INTEGER PRIMARY KEY AUTOINCREMENT,
                    session_id TEXT NOT NULL,
                    event_type TEXT NOT NULL,
                    payload_json TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    FOREIGN KEY(session_id) REFERENCES sessions(session_id)
                );
                INSERT INTO sessions (session_id, status, payload_json, created_at)
                    VALUES ('sess_legacy', 'active', '{}', 1700000000);
                INSERT INTO session_events
                    (session_id, event_type, payload_json, created_at)
                    VALUES
                    ('sess_legacy', 'PRE_TOOL_USE',
                     '{"tool_name":"scada_read_snapshot"}', 1700000001),
                    ('sess_legacy', 'POST_TOOL_USE',
                     '{"tool_name":"memory_write_anchor"}', 1700000002);
                """
            )
            await db.commit()

        # Phase 1 migration — adds columns, creates FTS5, backfills.
        await init_db(db_path)

        # Verify FTS5 search finds pre-migration events.
        backend = SqliteWalBackend(db_path)
        scada_hits = await backend.search("sess_legacy", "scada")
        memory_hits = await backend.search("sess_legacy", "memory")

        assert len(scada_hits) == 1, (
            f"Pre-migration 'scada' event not found via FTS. Got: {scada_hits}"
        )
        assert scada_hits[0]["event_type"] == "PRE_TOOL_USE"

        assert len(memory_hits) == 1, (
            f"Pre-migration 'memory' event not found via FTS. Got: {memory_hits}"
        )

        # Verify list_events still works (migration didn't break old data).
        all_events = await backend.list_events("sess_legacy")
        assert len(all_events) == 2
    finally:
        for suffix in ("", "-wal", "-shm"):
            p = db_path + suffix
            if os.path.exists(p):
                try:
                    os.unlink(p)
                except OSError:
                    pass


async def test_migration_is_idempotent() -> None:
    """Running init_db twice must not duplicate FTS entries or fail."""
    db_path = tempfile.mktemp(suffix=".db")
    try:
        await init_db(db_path)

        async with aiosqlite.connect(db_path) as db:
            await db.execute(
                "INSERT INTO sessions (session_id, status, payload_json, created_at) "
                "VALUES ('s1', 'active', '{}', 1)"
            )
            await db.execute(
                "INSERT INTO session_events "
                "(session_id, event_type, payload_json, created_at) "
                "VALUES ('s1', 'STOP', '{\"reason\":\"done\"}', 2)"
            )
            await db.commit()

        # Re-run init_db — should not error, should not duplicate FTS rows.
        await init_db(db_path)
        await init_db(db_path)

        backend = SqliteWalBackend(db_path)
        hits = await backend.search("s1", "done")
        assert len(hits) == 1, (
            f"After re-running init_db, expected 1 hit, got {len(hits)}"
        )
    finally:
        for suffix in ("", "-wal", "-shm"):
            p = db_path + suffix
            if os.path.exists(p):
                try:
                    os.unlink(p)
                except OSError:
                    pass
