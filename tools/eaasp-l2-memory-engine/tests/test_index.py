"""Layer 3 — Hybrid index (FTS5 + time-decay) tests."""

from __future__ import annotations

import pytest

from eaasp_l2_memory_engine.files import MemoryFileIn, MemoryFileStore
from eaasp_l2_memory_engine.index import HybridIndex


pytestmark = pytest.mark.asyncio


async def test_keyword_search_hit(
    file_store: MemoryFileStore, index: HybridIndex
) -> None:
    await file_store.write(
        MemoryFileIn(
            scope="user:alice",
            category="threshold",
            content="salary floor 50000 for engineers",
        )
    )
    await file_store.write(
        MemoryFileIn(
            scope="user:alice",
            category="preference",
            content="prefers remote work over office",
        )
    )

    hits = await index.search("salary")
    assert len(hits) == 1
    assert "salary" in hits[0].memory.content


async def test_search_scope_filter(
    file_store: MemoryFileStore, index: HybridIndex
) -> None:
    await file_store.write(
        MemoryFileIn(scope="alice", category="c", content="python rocks")
    )
    await file_store.write(
        MemoryFileIn(scope="bob", category="c", content="python rocks")
    )

    alice_hits = await index.search("python", scope="alice")
    assert len(alice_hits) == 1
    assert alice_hits[0].memory.scope == "alice"


async def test_search_returns_latest_version_only(
    file_store: MemoryFileStore, index: HybridIndex
) -> None:
    first = await file_store.write(
        MemoryFileIn(scope="s", category="c", content="alpha beta gamma")
    )
    await file_store.write(
        MemoryFileIn(
            memory_id=first.memory_id,
            scope="s",
            category="c",
            content="alpha delta epsilon",
        )
    )

    hits = await index.search("alpha")
    assert len(hits) == 1
    assert hits[0].memory.version == 2
    assert "delta" in hits[0].memory.content


async def test_search_empty_query_returns_empty(
    file_store: MemoryFileStore, index: HybridIndex
) -> None:
    await file_store.write(
        MemoryFileIn(scope="s", category="c", content="anything")
    )
    hits = await index.search("   ")
    assert hits == []


async def test_time_decay_weights_recent_higher(
    file_store: MemoryFileStore, index: HybridIndex
) -> None:
    # both match the same token, so fts_score is comparable; time decay decides order
    old = await file_store.write(
        MemoryFileIn(scope="s", category="c", content="widget widget")
    )
    # Simulate an older memory by backdating updated_at directly
    import aiosqlite

    async with aiosqlite.connect(file_store.db_path) as db:
        old_ts = 0  # epoch
        await db.execute(
            "UPDATE memory_files SET updated_at = ? WHERE memory_id = ?",
            (old_ts, old.memory_id),
        )
        await db.commit()

    await file_store.write(
        MemoryFileIn(scope="s", category="c", content="widget widget")
    )

    hits = await index.search("widget", top_k=5)
    assert len(hits) == 2
    # Newer entry should rank first due to time_decay
    assert hits[0].memory.memory_id != old.memory_id
    assert hits[0].time_decay > hits[1].time_decay
