"""Regression tests for the review findings (C1/C2/C3 + M3/M4)."""

from __future__ import annotations

import asyncio

import aiosqlite
import pytest

from eaasp_l2_memory_engine.anchors import AnchorIn, AnchorStore
from eaasp_l2_memory_engine.files import (
    InvalidStatusTransition,
    MemoryFileIn,
    MemoryFileStore,
)
from eaasp_l2_memory_engine.index import MAX_TOP_K, HybridIndex


pytestmark = pytest.mark.asyncio


# --- C1: concurrent version-bump ---------------------------------------------

async def test_concurrent_version_bump_is_serialized(
    file_store: MemoryFileStore,
) -> None:
    """Two concurrent writes to the same memory_id must produce versions 2 and 3."""
    first = await file_store.write(
        MemoryFileIn(scope="s", category="c", content="v1")
    )
    second, third = await asyncio.gather(
        file_store.write(
            MemoryFileIn(memory_id=first.memory_id, scope="s", category="c", content="v2")
        ),
        file_store.write(
            MemoryFileIn(memory_id=first.memory_id, scope="s", category="c", content="v3")
        ),
    )
    versions = sorted([second.version, third.version])
    assert versions == [2, 3]


# --- C2: adversarial FTS5 queries --------------------------------------------

@pytest.mark.parametrize(
    "query",
    [
        'foo^bar',
        '"',
        '""',
        'NOT foo',
        'a:b',
        'foo*bar',
        '()',
        'foo OR bar',
    ],
)
async def test_search_tolerates_adversarial_queries(
    file_store: MemoryFileStore, index: HybridIndex, query: str
) -> None:
    await file_store.write(
        MemoryFileIn(scope="s", category="c", content="foo bar baz")
    )
    # Should not raise even with operator/quote characters in the query.
    hits = await index.search(query)
    assert isinstance(hits, list)


async def test_search_pure_punctuation_returns_empty(
    file_store: MemoryFileStore, index: HybridIndex
) -> None:
    await file_store.write(
        MemoryFileIn(scope="s", category="c", content="content")
    )
    assert await index.search("!!!***") == []


# --- C3: top_k bounds --------------------------------------------------------

async def test_search_top_k_clamped(
    file_store: MemoryFileStore, index: HybridIndex
) -> None:
    await file_store.write(
        MemoryFileIn(scope="s", category="c", content="hello world")
    )
    # A ridiculously large top_k should be clamped to MAX_TOP_K, not blow memory.
    hits = await index.search("hello", top_k=10_000_000)
    assert len(hits) <= MAX_TOP_K


async def test_search_zero_top_k_clamped_to_one(
    file_store: MemoryFileStore, index: HybridIndex
) -> None:
    await file_store.write(
        MemoryFileIn(scope="s", category="c", content="hello world")
    )
    hits = await index.search("hello", top_k=0)
    # Clamped to 1; either 0 or 1 hits is acceptable (we have 1 match).
    assert len(hits) == 1


# --- M3: anchors are DB-enforced append-only --------------------------------

async def test_anchors_no_update_trigger(anchor_store: AnchorStore) -> None:
    out = await anchor_store.write(
        AnchorIn(event_id="e", session_id="s", type="t")
    )
    async with aiosqlite.connect(anchor_store.db_path) as db:
        with pytest.raises(Exception) as exc:
            await db.execute(
                "UPDATE anchors SET type = ? WHERE anchor_id = ?",
                ("mutated", out.anchor_id),
            )
            await db.commit()
        assert "append-only" in str(exc.value)


async def test_anchors_no_delete_trigger(anchor_store: AnchorStore) -> None:
    out = await anchor_store.write(
        AnchorIn(event_id="e", session_id="s", type="t")
    )
    async with aiosqlite.connect(anchor_store.db_path) as db:
        with pytest.raises(Exception) as exc:
            await db.execute(
                "DELETE FROM anchors WHERE anchor_id = ?", (out.anchor_id,)
            )
            await db.commit()
        assert "append-only" in str(exc.value)


# --- M4: status transitions enforced on write() too -------------------------

async def test_write_cannot_reverse_archived_to_agent_suggested(
    file_store: MemoryFileStore,
) -> None:
    first = await file_store.write(
        MemoryFileIn(scope="s", category="c", content="x")
    )
    await file_store.archive(first.memory_id)
    with pytest.raises(InvalidStatusTransition):
        await file_store.write(
            MemoryFileIn(
                memory_id=first.memory_id,
                scope="s",
                category="c",
                content="sneaky",
                status="agent_suggested",
            )
        )


async def test_write_cannot_reverse_confirmed_to_agent_suggested(
    file_store: MemoryFileStore,
) -> None:
    first = await file_store.write(
        MemoryFileIn(scope="s", category="c", content="x")
    )
    await file_store.confirm(first.memory_id)
    with pytest.raises(InvalidStatusTransition):
        await file_store.write(
            MemoryFileIn(
                memory_id=first.memory_id,
                scope="s",
                category="c",
                content="sneaky",
                status="agent_suggested",
            )
        )


async def test_write_same_status_is_allowed(file_store: MemoryFileStore) -> None:
    """Rewriting a memory in the same status should bump version."""
    first = await file_store.write(
        MemoryFileIn(scope="s", category="c", content="a")
    )
    second = await file_store.write(
        MemoryFileIn(
            memory_id=first.memory_id,
            scope="s",
            category="c",
            content="b",
            status="agent_suggested",
        )
    )
    assert second.version == 2
