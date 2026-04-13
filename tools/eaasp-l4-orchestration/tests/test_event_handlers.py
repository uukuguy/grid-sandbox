"""Tests for the 4 Event Engine handlers."""

from __future__ import annotations

import time

from eaasp_l4_orchestration.event_handlers import (
    DefaultIngestor,
    FTS5Indexer,
    TimeWindowClusterer,
    TimeWindowDeduplicator,
)
from eaasp_l4_orchestration.event_models import Event, EventMetadata


# ── DefaultIngestor ──────────────────────────────────────────────────────────


async def test_ingestor_assigns_event_id_when_empty():
    handler = DefaultIngestor()
    e = Event(session_id="s1", event_type="STOP", event_id="")
    # Force event_id to empty (post_init assigns one)
    e.event_id = ""
    result = await handler.handle(e)
    assert result is not None
    assert result.event_id
    assert len(result.event_id) == 36


async def test_ingestor_preserves_existing_event_id():
    handler = DefaultIngestor()
    e = Event(session_id="s1", event_type="STOP", event_id="keep-me")
    result = await handler.handle(e)
    assert result is not None
    assert result.event_id == "keep-me"


async def test_ingestor_assigns_unknown_source():
    handler = DefaultIngestor()
    e = Event(session_id="s1", event_type="STOP")
    result = await handler.handle(e)
    assert result is not None
    assert result.metadata.source == "unknown"


async def test_ingestor_preserves_existing_source():
    handler = DefaultIngestor()
    e = Event(
        session_id="s1",
        event_type="STOP",
        metadata=EventMetadata(source="runtime:grid"),
    )
    result = await handler.handle(e)
    assert result is not None
    assert result.metadata.source == "runtime:grid"


# ── TimeWindowDeduplicator ───────────────────────────────────────────────────


async def test_deduplicator_drops_duplicate_in_window():
    handler = TimeWindowDeduplicator(window_seconds=2.0)
    now = int(time.time())
    e1 = Event(
        session_id="s1",
        event_type="PRE_TOOL_USE",
        payload={"tool_name": "scada"},
        created_at=now,
    )
    e2 = Event(
        session_id="s1",
        event_type="PRE_TOOL_USE",
        payload={"tool_name": "scada"},
        created_at=now + 1,
    )
    r1 = await handler.handle(e1)
    r2 = await handler.handle(e2)
    assert r1 is not None
    assert r2 is None  # duplicate dropped


async def test_deduplicator_allows_after_window():
    handler = TimeWindowDeduplicator(window_seconds=2.0)
    now = int(time.time())
    e1 = Event(
        session_id="s1",
        event_type="PRE_TOOL_USE",
        payload={"tool_name": "scada"},
        created_at=now,
    )
    e2 = Event(
        session_id="s1",
        event_type="PRE_TOOL_USE",
        payload={"tool_name": "scada"},
        created_at=now + 3,
    )
    r1 = await handler.handle(e1)
    r2 = await handler.handle(e2)
    assert r1 is not None
    assert r2 is not None  # outside window


async def test_deduplicator_different_types_not_deduped():
    handler = TimeWindowDeduplicator(window_seconds=2.0)
    now = int(time.time())
    e1 = Event(
        session_id="s1",
        event_type="PRE_TOOL_USE",
        payload={"tool_name": "scada"},
        created_at=now,
    )
    e2 = Event(
        session_id="s1",
        event_type="POST_TOOL_USE",
        payload={"tool_name": "scada"},
        created_at=now,
    )
    r1 = await handler.handle(e1)
    r2 = await handler.handle(e2)
    assert r1 is not None
    assert r2 is not None


# ── TimeWindowClusterer ──────────────────────────────────────────────────────


async def test_clusterer_assigns_cluster_id():
    handler = TimeWindowClusterer(window_seconds=30.0)
    now = int(time.time())
    e = Event(session_id="s1", event_type="STOP", created_at=now)
    result = await handler.handle(e)
    assert result is not None
    assert result.cluster_id is not None
    assert result.cluster_id.startswith("c-")


async def test_clusterer_same_window_same_cluster():
    handler = TimeWindowClusterer(window_seconds=30.0)
    now = int(time.time())
    e1 = Event(session_id="s1", event_type="A", created_at=now)
    e2 = Event(session_id="s1", event_type="B", created_at=now + 5)
    r1 = await handler.handle(e1)
    r2 = await handler.handle(e2)
    assert r1 is not None and r2 is not None
    assert r1.cluster_id == r2.cluster_id


async def test_clusterer_different_window_different_cluster():
    handler = TimeWindowClusterer(window_seconds=10.0)
    now = int(time.time())
    e1 = Event(session_id="s1", event_type="A", created_at=now)
    e2 = Event(session_id="s1", event_type="B", created_at=now + 15)
    r1 = await handler.handle(e1)
    r2 = await handler.handle(e2)
    assert r1 is not None and r2 is not None
    assert r1.cluster_id != r2.cluster_id


async def test_clusterer_different_sessions_independent():
    handler = TimeWindowClusterer(window_seconds=30.0)
    now = int(time.time())
    e1 = Event(session_id="s1", event_type="A", created_at=now)
    e2 = Event(session_id="s2", event_type="A", created_at=now)
    r1 = await handler.handle(e1)
    r2 = await handler.handle(e2)
    assert r1 is not None and r2 is not None
    assert r1.cluster_id != r2.cluster_id


# ── FTS5Indexer ──────────────────────────────────────────────────────────────


async def test_indexer_passthrough():
    handler = FTS5Indexer()
    e = Event(session_id="s1", event_type="STOP", payload={"x": 1})
    result = await handler.handle(e)
    assert result is e  # no modification
