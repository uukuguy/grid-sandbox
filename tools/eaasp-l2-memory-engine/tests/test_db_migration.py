"""Phase 2 S2.T1 — schema migration idempotency tests.

Covers the embedding columns added to ``memory_files`` plus the helper
pack / unpack utilities. Uses a fresh ``tmp_path`` SQLite file per test so
no cross-test leakage is possible.
"""

from __future__ import annotations

from pathlib import Path

import aiosqlite

from eaasp_l2_memory_engine.db import (
    apply_embedding_migration,
    init_db,
    pack_embedding,
    unpack_embedding,
)

# asyncio_mode="auto" in pyproject handles marking; module-level marker would
# wrongly tag synchronous helpers like test_pack_unpack_embedding_round_trip.


async def _column_names(path: str) -> set[str]:
    """Return the set of column names currently on ``memory_files``."""
    async with aiosqlite.connect(path) as db:
        db.row_factory = aiosqlite.Row
        cur = await db.execute("PRAGMA table_info(memory_files)")
        rows = await cur.fetchall()
    return {row["name"] for row in rows}


async def test_apply_embedding_migration_idempotent(tmp_path: Path) -> None:
    """Running init_db twice must not duplicate columns or error."""
    db_file = str(tmp_path / "idem.db")
    await init_db(db_file)
    first = await _column_names(db_file)
    assert "embedding_model_id" in first
    assert "embedding_dim" in first
    assert "embedding_vec" in first

    # Second call — should be a no-op, not raise "duplicate column" errors.
    await init_db(db_file)
    second = await _column_names(db_file)
    assert first == second

    # Explicit third call directly into apply_embedding_migration on an
    # independent connection (with row_factory) to assert it is re-runnable.
    async with aiosqlite.connect(db_file) as db:
        db.row_factory = aiosqlite.Row
        await apply_embedding_migration(db)
    third = await _column_names(db_file)
    assert first == third


async def test_embedding_columns_nullable(tmp_path: Path) -> None:
    """A row inserted without embedding columns must read back as NULL."""
    db_file = str(tmp_path / "nullable.db")
    await init_db(db_file)

    async with aiosqlite.connect(db_file) as db:
        db.row_factory = aiosqlite.Row
        await db.execute(
            """
            INSERT INTO memory_files (
                memory_id, version, scope, category, content, evidence_refs,
                status, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            ("mem_no_embed", 1, "s", "c", "content", "[]", "agent_suggested", 1, 1),
        )
        await db.commit()

        cur = await db.execute(
            """
            SELECT embedding_model_id, embedding_dim, embedding_vec
            FROM memory_files WHERE memory_id = ?
            """,
            ("mem_no_embed",),
        )
        row = await cur.fetchone()

    assert row is not None
    assert row["embedding_model_id"] is None
    assert row["embedding_dim"] is None
    assert row["embedding_vec"] is None


def test_pack_unpack_embedding_round_trip() -> None:
    """pack → unpack must preserve length and values within float32 precision."""
    import math

    # Use a mix of positive / negative / zero / fractional values to stress
    # the f32 encoding. 1024 dims matches bge-m3:fp16.
    vec = [((i % 200) - 100) * 0.01 for i in range(1024)]
    blob = pack_embedding(vec)
    assert len(blob) == 1024 * 4  # 4 bytes per f32

    recovered = unpack_embedding(blob, 1024)
    assert len(recovered) == 1024
    for original, got in zip(vec, recovered):
        assert math.isclose(original, got, rel_tol=1e-6, abs_tol=1e-6)


async def test_idx_embedding_model_exists(tmp_path: Path) -> None:
    """The ``idx_memory_files_embedding_model`` index must be created."""
    db_file = str(tmp_path / "idx.db")
    await init_db(db_file)

    async with aiosqlite.connect(db_file) as db:
        db.row_factory = aiosqlite.Row
        # PRAGMA index_info returns one row per indexed column.
        cur = await db.execute("PRAGMA index_info(idx_memory_files_embedding_model)")
        rows = await cur.fetchall()

    assert len(rows) == 1
    assert rows[0]["name"] == "embedding_model_id"
