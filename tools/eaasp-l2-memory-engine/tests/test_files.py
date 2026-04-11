"""Layer 2 — Versioned memory file store + status state machine tests."""

from __future__ import annotations

import pytest

from eaasp_l2_memory_engine.files import (
    InvalidStatusTransition,
    MemoryFileIn,
    MemoryFileStore,
)


pytestmark = pytest.mark.asyncio


async def test_write_and_read_memory(file_store: MemoryFileStore) -> None:
    out = await file_store.write(
        MemoryFileIn(
            scope="user:alice/skill:hr-onboard",
            category="threshold",
            content="salary_floor=50000",
            evidence_refs=["anc_1", "anc_2"],
        )
    )
    assert out.memory_id.startswith("mem_")
    assert out.version == 1
    assert out.status == "agent_suggested"

    latest = await file_store.read_latest(out.memory_id)
    assert latest is not None
    assert latest.version == 1
    assert latest.content == "salary_floor=50000"


async def test_version_bump_on_rewrite(file_store: MemoryFileStore) -> None:
    first = await file_store.write(
        MemoryFileIn(scope="s1", category="c1", content="v1")
    )
    second = await file_store.write(
        MemoryFileIn(memory_id=first.memory_id, scope="s1", category="c1", content="v2")
    )
    assert second.memory_id == first.memory_id
    assert second.version == 2

    latest = await file_store.read_latest(first.memory_id)
    assert latest is not None
    assert latest.version == 2
    assert latest.content == "v2"


async def test_status_transition_confirm_then_archive(
    file_store: MemoryFileStore,
) -> None:
    first = await file_store.write(
        MemoryFileIn(scope="s", category="c", content="x")
    )
    confirmed = await file_store.confirm(first.memory_id)
    assert confirmed.status == "confirmed"
    assert confirmed.version == 2

    archived = await file_store.archive(first.memory_id)
    assert archived.status == "archived"
    assert archived.version == 3


async def test_invalid_status_transition(file_store: MemoryFileStore) -> None:
    first = await file_store.write(
        MemoryFileIn(scope="s", category="c", content="x")
    )
    await file_store.archive(first.memory_id)
    with pytest.raises(InvalidStatusTransition):
        await file_store.confirm(first.memory_id)
    with pytest.raises(InvalidStatusTransition):
        await file_store.archive(first.memory_id)


async def test_archive_unknown_raises(file_store: MemoryFileStore) -> None:
    with pytest.raises(KeyError):
        await file_store.archive("mem_missing")


async def test_list_filters_by_scope_and_category(file_store: MemoryFileStore) -> None:
    await file_store.write(MemoryFileIn(scope="alice", category="cat1", content="a"))
    await file_store.write(MemoryFileIn(scope="alice", category="cat2", content="b"))
    await file_store.write(MemoryFileIn(scope="bob", category="cat1", content="c"))

    alice_cat1 = await file_store.list(scope="alice", category="cat1")
    assert len(alice_cat1) == 1
    assert alice_cat1[0].content == "a"

    all_alice = await file_store.list(scope="alice")
    assert len(all_alice) == 2

    all_cat1 = await file_store.list(category="cat1")
    assert len(all_cat1) == 2


async def test_list_returns_latest_version_only(file_store: MemoryFileStore) -> None:
    first = await file_store.write(
        MemoryFileIn(scope="s", category="c", content="v1")
    )
    await file_store.write(
        MemoryFileIn(memory_id=first.memory_id, scope="s", category="c", content="v2")
    )
    rows = await file_store.list(scope="s")
    assert len(rows) == 1
    assert rows[0].version == 2
    assert rows[0].content == "v2"
