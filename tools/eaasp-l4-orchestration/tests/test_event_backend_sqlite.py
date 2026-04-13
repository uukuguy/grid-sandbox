"""Tests for SqliteWalBackend — append, list, count, search, subscribe, update_cluster."""

from __future__ import annotations

import asyncio

import pytest

from eaasp_l4_orchestration.event_backend_sqlite import SqliteWalBackend


async def test_append_returns_seq_and_event_id(
    tmp_db_path: str, seed_session
) -> None:
    sid = await seed_session("sess_be_1")
    backend = SqliteWalBackend(tmp_db_path)
    seq, event_id = await backend.append(
        session_id=sid,
        event_type="SESSION_START",
        payload={"runtime_id": "grid-runtime"},
        source="interceptor:grid-runtime",
    )
    assert seq >= 1
    assert len(event_id) == 36  # UUID


async def test_list_events_with_new_columns(
    tmp_db_path: str, seed_session
) -> None:
    sid = await seed_session("sess_be_2")
    backend = SqliteWalBackend(tmp_db_path)
    await backend.append(sid, "A", {"k": 1}, source="src-a")
    await backend.append(sid, "B", {"k": 2}, source="src-b")
    events = await backend.list_events(sid)
    assert len(events) == 2
    assert events[0]["event_type"] == "A"
    assert events[0]["source"] == "src-a"
    assert events[0]["event_id"]  # non-empty
    assert events[1]["event_type"] == "B"


async def test_list_events_filter_by_event_types(
    tmp_db_path: str, seed_session
) -> None:
    sid = await seed_session("sess_be_3")
    backend = SqliteWalBackend(tmp_db_path)
    await backend.append(sid, "PRE_TOOL_USE", {})
    await backend.append(sid, "POST_TOOL_USE", {})
    await backend.append(sid, "STOP", {})
    events = await backend.list_events(
        sid, event_types=["PRE_TOOL_USE", "STOP"]
    )
    assert [e["event_type"] for e in events] == ["PRE_TOOL_USE", "STOP"]


async def test_count(tmp_db_path: str, seed_session) -> None:
    sid = await seed_session("sess_be_4")
    backend = SqliteWalBackend(tmp_db_path)
    assert await backend.count(sid) == 0
    await backend.append(sid, "A", {})
    await backend.append(sid, "B", {})
    assert await backend.count(sid) == 2


async def test_search_fts(tmp_db_path: str, seed_session) -> None:
    sid = await seed_session("sess_be_5")
    backend = SqliteWalBackend(tmp_db_path)
    await backend.append(
        sid, "PRE_TOOL_USE", {"tool_name": "scada_read_snapshot"}
    )
    await backend.append(
        sid, "POST_TOOL_USE", {"tool_name": "memory_write_anchor"}
    )
    await backend.append(sid, "STOP", {"reason": "complete"})
    results = await backend.search(sid, "scada")
    assert len(results) >= 1
    assert results[0]["event_type"] == "PRE_TOOL_USE"


async def test_update_cluster(tmp_db_path: str, seed_session) -> None:
    sid = await seed_session("sess_be_6")
    backend = SqliteWalBackend(tmp_db_path)
    _, eid = await backend.append(sid, "A", {})
    await backend.update_cluster(eid, "c-001")
    events = await backend.list_events(sid)
    assert events[0]["cluster_id"] == "c-001"


async def test_subscribe_yields_new_events(
    tmp_db_path: str, seed_session
) -> None:
    sid = await seed_session("sess_be_7")
    backend = SqliteWalBackend(tmp_db_path)
    await backend.append(sid, "A", {})

    received: list[dict] = []

    async def _consume() -> None:
        async for event in backend.subscribe(sid, from_seq=0):
            received.append(event)
            if len(received) >= 2:
                break

    async def _produce() -> None:
        await asyncio.sleep(0.3)
        await backend.append(sid, "B", {})

    await asyncio.gather(
        asyncio.wait_for(_consume(), timeout=3.0),
        _produce(),
    )
    assert len(received) == 2
    assert received[0]["event_type"] == "A"
    assert received[1]["event_type"] == "B"


async def test_append_with_explicit_event_id(
    tmp_db_path: str, seed_session
) -> None:
    sid = await seed_session("sess_be_8")
    backend = SqliteWalBackend(tmp_db_path)
    _, eid = await backend.append(
        sid, "X", {}, event_id="my-custom-id"
    )
    assert eid == "my-custom-id"
    events = await backend.list_events(sid)
    assert events[0]["event_id"] == "my-custom-id"
