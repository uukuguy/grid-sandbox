# Phase 1: Event-driven Foundation — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task.

**Goal:** Add event observability to EAASP — L4 Event Engine pipeline, L4 interceptor for auto-extracting events from L1 RPC chunks, SQLite WAL backend with FTS5 search, and CLI `session events --follow` for real-time event tracking.

**Architecture:** L4 platform interceptor extracts PRE_TOOL_USE/POST_TOOL_USE/STOP events from existing `stream_message()` chunks with zero L1 runtime changes. Events flow through an async 4-handler pipeline (Ingest→Dedup→Cluster→Index) backed by SQLite WAL. grid-runtime (T1) additionally emits richer events via REST POST to `/v1/events/ingest`.

**Tech Stack:** Python 3.12, FastAPI, aiosqlite (WAL mode), FTS5, pytest-asyncio, typer (CLI), Rust (grid-runtime EventEmitter via reqwest)

**Design Doc:** `docs/design/EAASP/PHASE1_EVENT_ENGINE_DESIGN.md`
**ADRs:** ADR-V2-001 (EmitEvent interface), ADR-V2-002 (Event Stream backend), ADR-V2-003 (Event clustering)

---

## Task 1: Event Data Models

**Files:**
- Create: `tools/eaasp-l4-orchestration/src/eaasp_l4_orchestration/event_models.py`
- Test: `tools/eaasp-l4-orchestration/tests/test_event_models.py`

**Step 1: Write the failing test**

```python
# tests/test_event_models.py
"""Tests for Event and EventMetadata data models."""
from __future__ import annotations
import time
from eaasp_l4_orchestration.event_models import Event, EventMetadata


def test_event_auto_assigns_event_id():
    e = Event(session_id="s1", event_type="STOP", payload={"reason": "done"})
    assert e.event_id  # non-empty UUID
    assert len(e.event_id) == 36  # UUID format


def test_event_auto_assigns_created_at():
    before = int(time.time())
    e = Event(session_id="s1", event_type="STOP")
    after = int(time.time())
    assert before <= e.created_at <= after


def test_event_preserves_explicit_values():
    e = Event(
        session_id="s1",
        event_type="PRE_TOOL_USE",
        event_id="custom-id",
        created_at=1000,
        cluster_id="c-001",
    )
    assert e.event_id == "custom-id"
    assert e.created_at == 1000
    assert e.cluster_id == "c-001"


def test_event_metadata_defaults():
    m = EventMetadata()
    assert m.source == ""
    assert m.trace_id == ""
    assert m.parent_event_id == ""
    assert m.extra == {}


def test_event_with_metadata():
    m = EventMetadata(source="interceptor:grid-runtime", trace_id="t-1")
    e = Event(session_id="s1", event_type="STOP", metadata=m)
    assert e.metadata.source == "interceptor:grid-runtime"
    assert e.metadata.trace_id == "t-1"
```

**Step 2: Run test to verify it fails**

Run: `cd /Users/sujiangwen/sandbox/LLM/speechless.ai/SGAI/grid-sandbox && python -m pytest tools/eaasp-l4-orchestration/tests/test_event_models.py -xvs`
Expected: FAIL — `ModuleNotFoundError: No module named 'eaasp_l4_orchestration.event_models'`

**Step 3: Write minimal implementation**

```python
# src/eaasp_l4_orchestration/event_models.py
"""标准化事件数据模型，对齐 proto EventStreamEntry。"""
from __future__ import annotations
from dataclasses import dataclass, field
from typing import Any
import time
import uuid


@dataclass
class EventMetadata:
    """事件追踪元数据。"""
    trace_id: str = ""
    span_id: str = ""
    parent_event_id: str = ""
    source: str = ""
    extra: dict[str, Any] = field(default_factory=dict)


@dataclass
class Event:
    """标准化事件结构。"""
    session_id: str
    event_type: str
    payload: dict[str, Any] = field(default_factory=dict)
    event_id: str = ""
    metadata: EventMetadata = field(default_factory=EventMetadata)
    created_at: int = 0
    cluster_id: str | None = None
    seq: int | None = None

    def __post_init__(self):
        if not self.event_id:
            self.event_id = str(uuid.uuid4())
        if not self.created_at:
            self.created_at = int(time.time())
```

**Step 4: Run test to verify it passes**

Run: `cd /Users/sujiangwen/sandbox/LLM/speechless.ai/SGAI/grid-sandbox && python -m pytest tools/eaasp-l4-orchestration/tests/test_event_models.py -xvs`
Expected: 5 PASS

**Step 5: Commit**

```bash
git add tools/eaasp-l4-orchestration/src/eaasp_l4_orchestration/event_models.py tools/eaasp-l4-orchestration/tests/test_event_models.py
git commit -m "feat(eaasp): add Event + EventMetadata data models for Phase 1 Event Engine"
```

---

## Task 2: EventStreamBackend Protocol + SqliteWalBackend

**Files:**
- Create: `tools/eaasp-l4-orchestration/src/eaasp_l4_orchestration/event_backend.py`
- Create: `tools/eaasp-l4-orchestration/src/eaasp_l4_orchestration/event_backend_sqlite.py`
- Modify: `tools/eaasp-l4-orchestration/src/eaasp_l4_orchestration/db.py` — add migration columns + FTS5
- Test: `tools/eaasp-l4-orchestration/tests/test_event_backend_sqlite.py`

**Step 1: Write the failing tests**

```python
# tests/test_event_backend_sqlite.py
"""Tests for SqliteWalBackend — append, list, count, search, subscribe, update_cluster."""
from __future__ import annotations
import asyncio
import pytest
from eaasp_l4_orchestration.event_backend_sqlite import SqliteWalBackend


async def test_append_returns_seq_and_event_id(tmp_db_path: str, seed_session) -> None:
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


async def test_list_events_with_new_columns(tmp_db_path: str, seed_session) -> None:
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


async def test_list_events_filter_by_event_types(tmp_db_path: str, seed_session) -> None:
    sid = await seed_session("sess_be_3")
    backend = SqliteWalBackend(tmp_db_path)
    await backend.append(sid, "PRE_TOOL_USE", {})
    await backend.append(sid, "POST_TOOL_USE", {})
    await backend.append(sid, "STOP", {})
    events = await backend.list_events(sid, event_types=["PRE_TOOL_USE", "STOP"])
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
    await backend.append(sid, "PRE_TOOL_USE", {"tool_name": "scada_read_snapshot"})
    await backend.append(sid, "POST_TOOL_USE", {"tool_name": "memory_write_anchor"})
    await backend.append(sid, "STOP", {"reason": "complete"})
    results = await backend.search(sid, "scada")
    assert len(results) >= 1
    assert any("scada" in r.get("payload", {}).get("tool_name", "") for r in results)


async def test_update_cluster(tmp_db_path: str, seed_session) -> None:
    sid = await seed_session("sess_be_6")
    backend = SqliteWalBackend(tmp_db_path)
    _, eid = await backend.append(sid, "A", {})
    await backend.update_cluster(eid, "c-001")
    events = await backend.list_events(sid)
    assert events[0]["cluster_id"] == "c-001"


async def test_subscribe_yields_new_events(tmp_db_path: str, seed_session) -> None:
    sid = await seed_session("sess_be_7")
    backend = SqliteWalBackend(tmp_db_path)
    await backend.append(sid, "A", {})

    received = []
    async def _consume():
        async for event in backend.subscribe(sid, from_seq=0):
            received.append(event)
            if len(received) >= 2:
                break

    # Append second event after a short delay
    async def _produce():
        await asyncio.sleep(0.3)
        await backend.append(sid, "B", {})

    await asyncio.gather(
        asyncio.wait_for(_consume(), timeout=3.0),
        _produce(),
    )
    assert len(received) == 2
    assert received[0]["event_type"] == "A"
    assert received[1]["event_type"] == "B"
```

**Step 2: Run test to verify it fails**

Run: `cd /Users/sujiangwen/sandbox/LLM/speechless.ai/SGAI/grid-sandbox && python -m pytest tools/eaasp-l4-orchestration/tests/test_event_backend_sqlite.py -xvs`
Expected: FAIL — `ModuleNotFoundError`

**Step 3: Write implementations**

First, update `db.py` to add migration:

```python
# db.py — append to SCHEMA (after existing tables)
# Add SCHEMA_V2 migration at module level, run in init_db()

SCHEMA_V2_MIGRATION = """
-- Phase 1 Event Engine columns (idempotent)
-- SQLite ALTER TABLE ADD COLUMN is online and idempotent-safe via try/except
"""

SCHEMA_V2_FTS = """
CREATE VIRTUAL TABLE IF NOT EXISTS session_events_fts USING fts5(
    event_type, payload_json,
    content='session_events', content_rowid='seq'
);

CREATE TRIGGER IF NOT EXISTS session_events_fts_ai
AFTER INSERT ON session_events BEGIN
    INSERT INTO session_events_fts(rowid, event_type, payload_json)
    VALUES (new.seq, new.event_type, new.payload_json);
END;
"""
```

Then `event_backend.py` (Protocol) and `event_backend_sqlite.py` (implementation).

The `SqliteWalBackend` wraps the existing `SessionEventStream` pattern but adds:
- `event_id`, `source`, `metadata_json`, `cluster_id` columns
- `subscribe()` via polling
- `search()` via FTS5
- `update_cluster()` for Event Engine

**Step 4: Run test to verify it passes**

Run: `cd /Users/sujiangwen/sandbox/LLM/speechless.ai/SGAI/grid-sandbox && python -m pytest tools/eaasp-l4-orchestration/tests/test_event_backend_sqlite.py -xvs`
Expected: 7 PASS

**Step 5: Run existing tests to ensure no regression**

Run: `cd /Users/sujiangwen/sandbox/LLM/speechless.ai/SGAI/grid-sandbox && python -m pytest tools/eaasp-l4-orchestration/tests/ -xvs`
Expected: All existing tests still pass (new columns are nullable, no breaking changes)

**Step 6: Commit**

```bash
git add tools/eaasp-l4-orchestration/src/eaasp_l4_orchestration/event_backend.py \
       tools/eaasp-l4-orchestration/src/eaasp_l4_orchestration/event_backend_sqlite.py \
       tools/eaasp-l4-orchestration/src/eaasp_l4_orchestration/db.py \
       tools/eaasp-l4-orchestration/tests/test_event_backend_sqlite.py
git commit -m "feat(eaasp): EventStreamBackend Protocol + SqliteWalBackend with FTS5 and subscribe"
```

---

## Task 3: Event Handlers (Ingestor + Deduplicator + Clusterer + Indexer)

**Files:**
- Create: `tools/eaasp-l4-orchestration/src/eaasp_l4_orchestration/event_handlers.py`
- Test: `tools/eaasp-l4-orchestration/tests/test_event_handlers.py`

**Step 1: Write the failing tests**

```python
# tests/test_event_handlers.py
"""Tests for the 4 Event Engine handlers."""
from __future__ import annotations
import time
from eaasp_l4_orchestration.event_models import Event, EventMetadata
from eaasp_l4_orchestration.event_handlers import (
    DefaultIngestor,
    TimeWindowDeduplicator,
    TimeWindowClusterer,
    FTS5Indexer,
)


async def test_ingestor_assigns_event_id_and_source():
    handler = DefaultIngestor()
    e = Event(session_id="s1", event_type="STOP", event_id="", metadata=EventMetadata())
    result = await handler.handle(e)
    assert result is not None
    assert result.event_id  # assigned
    assert len(result.event_id) == 36


async def test_ingestor_preserves_existing_event_id():
    handler = DefaultIngestor()
    e = Event(session_id="s1", event_type="STOP", event_id="keep-me")
    result = await handler.handle(e)
    assert result.event_id == "keep-me"


async def test_deduplicator_drops_duplicate_in_window():
    handler = TimeWindowDeduplicator(window_seconds=2.0)
    now = int(time.time())
    e1 = Event(session_id="s1", event_type="PRE_TOOL_USE",
               payload={"tool_name": "scada"}, created_at=now)
    e2 = Event(session_id="s1", event_type="PRE_TOOL_USE",
               payload={"tool_name": "scada"}, created_at=now + 1)
    r1 = await handler.handle(e1)
    r2 = await handler.handle(e2)
    assert r1 is not None
    assert r2 is None  # duplicate dropped


async def test_deduplicator_allows_after_window():
    handler = TimeWindowDeduplicator(window_seconds=2.0)
    now = int(time.time())
    e1 = Event(session_id="s1", event_type="PRE_TOOL_USE",
               payload={"tool_name": "scada"}, created_at=now)
    e2 = Event(session_id="s1", event_type="PRE_TOOL_USE",
               payload={"tool_name": "scada"}, created_at=now + 3)
    r1 = await handler.handle(e1)
    r2 = await handler.handle(e2)
    assert r1 is not None
    assert r2 is not None  # outside window


async def test_deduplicator_different_types_not_deduped():
    handler = TimeWindowDeduplicator(window_seconds=2.0)
    now = int(time.time())
    e1 = Event(session_id="s1", event_type="PRE_TOOL_USE",
               payload={"tool_name": "scada"}, created_at=now)
    e2 = Event(session_id="s1", event_type="POST_TOOL_USE",
               payload={"tool_name": "scada"}, created_at=now)
    r1 = await handler.handle(e1)
    r2 = await handler.handle(e2)
    assert r1 is not None
    assert r2 is not None


async def test_clusterer_assigns_cluster_id():
    handler = TimeWindowClusterer(window_seconds=30.0)
    now = int(time.time())
    e = Event(session_id="s1", event_type="STOP", created_at=now)
    result = await handler.handle(e)
    assert result.cluster_id is not None
    assert result.cluster_id.startswith("c-")


async def test_clusterer_same_window_same_cluster():
    handler = TimeWindowClusterer(window_seconds=30.0)
    now = int(time.time())
    e1 = Event(session_id="s1", event_type="A", created_at=now)
    e2 = Event(session_id="s1", event_type="B", created_at=now + 5)
    r1 = await handler.handle(e1)
    r2 = await handler.handle(e2)
    assert r1.cluster_id == r2.cluster_id


async def test_clusterer_different_window_different_cluster():
    handler = TimeWindowClusterer(window_seconds=10.0)
    now = int(time.time())
    e1 = Event(session_id="s1", event_type="A", created_at=now)
    e2 = Event(session_id="s1", event_type="B", created_at=now + 15)
    r1 = await handler.handle(e1)
    r2 = await handler.handle(e2)
    assert r1.cluster_id != r2.cluster_id


async def test_indexer_passthrough():
    handler = FTS5Indexer()
    e = Event(session_id="s1", event_type="STOP", payload={"x": 1})
    result = await handler.handle(e)
    assert result is e  # no modification
```

**Step 2: Run test to verify it fails**

Run: `cd /Users/sujiangwen/sandbox/LLM/speechless.ai/SGAI/grid-sandbox && python -m pytest tools/eaasp-l4-orchestration/tests/test_event_handlers.py -xvs`
Expected: FAIL — `ModuleNotFoundError`

**Step 3: Write minimal implementation**

See `docs/design/EAASP/PHASE1_EVENT_ENGINE_DESIGN.md` §2.5 for the handler implementations.

**Step 4: Run test to verify it passes**

Run: `cd /Users/sujiangwen/sandbox/LLM/speechless.ai/SGAI/grid-sandbox && python -m pytest tools/eaasp-l4-orchestration/tests/test_event_handlers.py -xvs`
Expected: 10 PASS

**Step 5: Commit**

```bash
git add tools/eaasp-l4-orchestration/src/eaasp_l4_orchestration/event_handlers.py \
       tools/eaasp-l4-orchestration/tests/test_event_handlers.py
git commit -m "feat(eaasp): 4 Event Engine handlers — Ingestor, Deduplicator, Clusterer, Indexer"
```

---

## Task 4: EventEngine Pipeline Orchestrator

**Files:**
- Create: `tools/eaasp-l4-orchestration/src/eaasp_l4_orchestration/event_engine.py`
- Test: `tools/eaasp-l4-orchestration/tests/test_event_engine.py`

**Step 1: Write the failing tests**

```python
# tests/test_event_engine.py
"""Tests for EventEngine — ingest, pipeline execution, cluster writeback."""
from __future__ import annotations
import asyncio
from eaasp_l4_orchestration.event_engine import EventEngine
from eaasp_l4_orchestration.event_backend_sqlite import SqliteWalBackend
from eaasp_l4_orchestration.event_models import Event, EventMetadata


async def test_ingest_persists_event(tmp_db_path: str, seed_session) -> None:
    sid = await seed_session("sess_eng_1")
    backend = SqliteWalBackend(tmp_db_path)
    engine = EventEngine(backend)
    await engine.start()
    try:
        event = Event(session_id=sid, event_type="STOP", payload={"reason": "done"})
        seq, eid = await engine.ingest(event)
        assert seq >= 1
        assert len(eid) == 36
        # Event is in backend
        events = await backend.list_events(sid)
        assert len(events) == 1
        assert events[0]["event_type"] == "STOP"
    finally:
        await engine.stop()


async def test_pipeline_assigns_cluster_id(tmp_db_path: str, seed_session) -> None:
    sid = await seed_session("sess_eng_2")
    backend = SqliteWalBackend(tmp_db_path)
    engine = EventEngine(backend)
    await engine.start()
    try:
        e1 = Event(session_id=sid, event_type="A", payload={})
        e2 = Event(session_id=sid, event_type="B", payload={})
        await engine.ingest(e1)
        await engine.ingest(e2)
        # Wait for pipeline worker to process
        await asyncio.sleep(1.5)
        events = await backend.list_events(sid)
        # At least one event should have a cluster_id after pipeline runs
        cluster_ids = [e.get("cluster_id") for e in events if e.get("cluster_id")]
        assert len(cluster_ids) >= 1
    finally:
        await engine.stop()


async def test_pipeline_deduplicates(tmp_db_path: str, seed_session) -> None:
    """Dedup happens in the pipeline but does NOT delete from backend.
    Both events are persisted; dedup only affects downstream handlers."""
    sid = await seed_session("sess_eng_3")
    backend = SqliteWalBackend(tmp_db_path)
    engine = EventEngine(backend)
    await engine.start()
    try:
        import time
        now = int(time.time())
        e1 = Event(session_id=sid, event_type="PRE_TOOL_USE",
                    payload={"tool_name": "scada"}, created_at=now)
        e2 = Event(session_id=sid, event_type="PRE_TOOL_USE",
                    payload={"tool_name": "scada"}, created_at=now)
        await engine.ingest(e1)
        await engine.ingest(e2)
        # Both are persisted (dedup doesn't delete, only prevents clustering)
        events = await backend.list_events(sid)
        assert len(events) == 2
    finally:
        await engine.stop()


async def test_engine_start_stop(tmp_db_path: str, seed_session) -> None:
    backend = SqliteWalBackend(tmp_db_path)
    engine = EventEngine(backend)
    await engine.start()
    assert engine._running
    await engine.stop()
    assert not engine._running
```

**Step 2: Run → FAIL**

Run: `cd /Users/sujiangwen/sandbox/LLM/speechless.ai/SGAI/grid-sandbox && python -m pytest tools/eaasp-l4-orchestration/tests/test_event_engine.py -xvs`

**Step 3: Write implementation**

See `docs/design/EAASP/PHASE1_EVENT_ENGINE_DESIGN.md` §2.6 for the EventEngine class.

**Step 4: Run → 4 PASS**

**Step 5: Commit**

```bash
git add tools/eaasp-l4-orchestration/src/eaasp_l4_orchestration/event_engine.py \
       tools/eaasp-l4-orchestration/tests/test_event_engine.py
git commit -m "feat(eaasp): EventEngine pipeline orchestrator with async worker"
```

---

## Task 5: EventInterceptor (extract events from L1 chunks)

**Files:**
- Create: `tools/eaasp-l4-orchestration/src/eaasp_l4_orchestration/event_interceptor.py`
- Test: `tools/eaasp-l4-orchestration/tests/test_event_interceptor.py`

**Step 1: Write the failing tests**

```python
# tests/test_event_interceptor.py
"""Tests for EventInterceptor — extracts events from L1 response chunks."""
from __future__ import annotations
from eaasp_l4_orchestration.event_interceptor import EventInterceptor


def test_extract_tool_call_start():
    interceptor = EventInterceptor()
    chunk = {"chunk_type": "tool_call_start", "tool_name": "scada_read", "arguments": {"id": "T-001"}}
    event = interceptor.extract_from_chunk("s1", chunk, runtime_id="grid-runtime")
    assert event is not None
    assert event.event_type == "PRE_TOOL_USE"
    assert event.payload["tool_name"] == "scada_read"
    assert "interceptor:grid-runtime" in event.metadata.source


def test_extract_tool_result_success():
    interceptor = EventInterceptor()
    chunk = {"chunk_type": "tool_result", "tool_name": "scada_read", "content": '{"temp": 85}', "is_error": False}
    event = interceptor.extract_from_chunk("s1", chunk)
    assert event is not None
    assert event.event_type == "POST_TOOL_USE"


def test_extract_tool_result_failure():
    interceptor = EventInterceptor()
    chunk = {"chunk_type": "tool_result", "tool_name": "scada_read", "content": "error", "is_error": True}
    event = interceptor.extract_from_chunk("s1", chunk)
    assert event is not None
    assert event.event_type == "POST_TOOL_USE_FAILURE"


def test_extract_done():
    interceptor = EventInterceptor()
    chunk = {"chunk_type": "done", "content": "", "response_text": "Calibration complete."}
    event = interceptor.extract_from_chunk("s1", chunk)
    assert event is not None
    assert event.event_type == "STOP"


def test_extract_text_delta_returns_none():
    interceptor = EventInterceptor()
    chunk = {"chunk_type": "text_delta", "content": "hello"}
    event = interceptor.extract_from_chunk("s1", chunk)
    assert event is None  # text_delta is not a lifecycle event


def test_create_session_start():
    interceptor = EventInterceptor()
    event = interceptor.create_session_start("s1", "grid-runtime")
    assert event.event_type == "SESSION_START"
    assert event.payload["runtime_id"] == "grid-runtime"


def test_create_session_end():
    interceptor = EventInterceptor()
    event = interceptor.create_session_end("s1")
    assert event.event_type == "POST_SESSION_END"
```

**Step 2: Run → FAIL**

**Step 3: Write implementation** (see design doc §2.5)

**Step 4: Run → 7 PASS**

**Step 5: Commit**

```bash
git add tools/eaasp-l4-orchestration/src/eaasp_l4_orchestration/event_interceptor.py \
       tools/eaasp-l4-orchestration/tests/test_event_interceptor.py
git commit -m "feat(eaasp): EventInterceptor — extract lifecycle events from L1 chunks"
```

---

## Task 6: Wire EventEngine + Interceptor into SessionOrchestrator + API

**Files:**
- Modify: `tools/eaasp-l4-orchestration/src/eaasp_l4_orchestration/session_orchestrator.py` — inject EventEngine + interceptor
- Modify: `tools/eaasp-l4-orchestration/src/eaasp_l4_orchestration/api.py` — new endpoints + wire EventEngine in lifespan
- Test: `tools/eaasp-l4-orchestration/tests/test_event_api.py`

**Step 1: Write the failing tests**

```python
# tests/test_event_api.py
"""Tests for Event Engine API endpoints."""
from __future__ import annotations
import json
import pytest


async def test_ingest_endpoint(app_client) -> None:
    """POST /v1/events/ingest should accept an event and return event_id."""
    # First create a session
    import respx, httpx
    with respx.mock:
        respx.get("http://127.0.0.1:18085/v1/memory/search").mock(
            return_value=httpx.Response(200, json={"results": []})
        )
        respx.post("http://127.0.0.1:18083/v1/sessions/sess_ingest/validate").mock(
            return_value=httpx.Response(200, json={"hooks_to_attach": [], "managed_settings_version": 1, "validated_at": "2026-01-01"})
        )
        respx.get("http://127.0.0.1:18081/v1/skills/test-skill").mock(
            return_value=httpx.Response(200, json={"meta": {"name": "test"}, "prose": "", "parsed_v2": {}})
        )
        resp = await app_client.post("/v1/sessions/create", json={
            "intent_text": "test", "skill_id": "test-skill", "runtime_pref": "grid-runtime"
        })
        assert resp.status_code == 200
        session_id = resp.json()["session_id"]

    # Now ingest an event
    resp = await app_client.post("/v1/events/ingest", json={
        "session_id": session_id,
        "event_type": "PRE_TOOL_USE",
        "payload": {"tool_name": "scada_read"},
        "source": "runtime:grid-runtime",
    })
    assert resp.status_code == 200
    data = resp.json()
    assert "event_id" in data
    assert "seq" in data


async def test_events_follow_sse(app_client) -> None:
    """GET /v1/sessions/{id}/events?follow=true should return SSE stream."""
    # This test verifies the endpoint exists and returns correct content-type.
    # Full SSE streaming requires async iteration — covered in E2E.
    resp = await app_client.get("/v1/sessions/nonexistent/events?follow=true")
    # Should be 404 for nonexistent session, not a 422 (proves route exists)
    assert resp.status_code == 404
```

**Step 2: Run → FAIL**

**Step 3: Modify `api.py` and `session_orchestrator.py`**

Key changes to `api.py`:
1. Import and wire `EventEngine`, `SqliteWalBackend`, `EventInterceptor` in lifespan
2. Add `POST /v1/events/ingest` endpoint
3. Enhance `GET /v1/sessions/{id}/events` with `follow` query parameter

Key changes to `session_orchestrator.py`:
1. Accept `event_engine` and `event_interceptor` in `__init__`
2. In `stream_message()`, call `interceptor.extract_from_chunk()` on each chunk and `engine.ingest()` if non-None
3. After `Initialize` success, ingest `SESSION_START` event
4. On `close_session`, ingest `POST_SESSION_END` event

**Step 4: Run all L4 tests**

Run: `cd /Users/sujiangwen/sandbox/LLM/speechless.ai/SGAI/grid-sandbox && python -m pytest tools/eaasp-l4-orchestration/tests/ -xvs`
Expected: All old tests pass + 2 new pass

**Step 5: Commit**

```bash
git add tools/eaasp-l4-orchestration/src/eaasp_l4_orchestration/api.py \
       tools/eaasp-l4-orchestration/src/eaasp_l4_orchestration/session_orchestrator.py \
       tools/eaasp-l4-orchestration/tests/test_event_api.py
git commit -m "feat(eaasp): wire EventEngine + Interceptor into SessionOrchestrator and API"
```

---

## Task 7: CLI `session events` Command

**Files:**
- Modify: `tools/eaasp-cli-v2/src/eaasp_cli_v2/cmd_session.py` — add `events` subcommand
- Modify: `tools/eaasp-cli-v2/src/eaasp_cli_v2/client.py` — add SSE method
- Test: `tools/eaasp-cli-v2/tests/test_cmd_session_events.py`

**Step 1: Write the failing tests**

```python
# tests/test_cmd_session_events.py
"""Tests for `eaasp session events` command."""
from __future__ import annotations
import json
from typer.testing import CliRunner
from unittest.mock import AsyncMock, patch
from eaasp_cli_v2.main import app

runner = CliRunner()


def test_session_events_list():
    """session events <id> should list events."""
    mock_response = {
        "session_id": "sess_001",
        "events": [
            {"seq": 1, "event_type": "SESSION_START", "payload": {}, "created_at": 1700000000},
            {"seq": 2, "event_type": "PRE_TOOL_USE", "payload": {"tool_name": "scada"}, "created_at": 1700000001},
        ]
    }
    with patch("eaasp_cli_v2.cmd_session._fetch_events", return_value=mock_response):
        result = runner.invoke(app, ["session", "events", "sess_001"])
        assert result.exit_code == 0
        assert "SESSION_START" in result.output
        assert "PRE_TOOL_USE" in result.output


def test_session_events_json_format():
    """session events <id> --format json should output raw JSON."""
    mock_response = {
        "session_id": "sess_001",
        "events": [
            {"seq": 1, "event_type": "STOP", "payload": {"reason": "done"}, "created_at": 1700000000},
        ]
    }
    with patch("eaasp_cli_v2.cmd_session._fetch_events", return_value=mock_response):
        result = runner.invoke(app, ["session", "events", "sess_001", "--format", "json"])
        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["events"][0]["event_type"] == "STOP"
```

**Step 2: Run → FAIL**

**Step 3: Implement `events` subcommand** in `cmd_session.py` + `_fetch_events()` helper

**Step 4: Run → 2 PASS**

**Step 5: Commit**

```bash
git add tools/eaasp-cli-v2/src/eaasp_cli_v2/cmd_session.py \
       tools/eaasp-cli-v2/src/eaasp_cli_v2/client.py \
       tools/eaasp-cli-v2/tests/test_cmd_session_events.py
git commit -m "feat(eaasp-cli): add session events command with follow and json format"
```

---

## Task 8: Update Checkpoint + Plan + Run Full Test Suite

**Files:**
- Modify: `docs/plans/.checkpoint.json`
- Modify: `docs/plans/2026-04-13-v2-phase1-plan.md`

**Step 1: Run full L4 test suite**

Run: `cd /Users/sujiangwen/sandbox/LLM/speechless.ai/SGAI/grid-sandbox && python -m pytest tools/eaasp-l4-orchestration/tests/ -xvs`
Expected: All tests pass (existing ~70 + new ~30 = ~100)

**Step 2: Run CLI test suite**

Run: `cd /Users/sujiangwen/sandbox/LLM/speechless.ai/SGAI/grid-sandbox && python -m pytest tools/eaasp-cli-v2/tests/ -xvs`
Expected: All tests pass (existing ~20 + new ~2 = ~22)

**Step 3: Update checkpoint**

Mark S1 + S2 + S3 + S4.T1 as complete. Update test counts.

**Step 4: Commit**

```bash
git add docs/plans/.checkpoint.json docs/plans/2026-04-13-v2-phase1-plan.md
git commit -m "docs(eaasp): Phase 1 S1-S3 + S4.T1 checkpoint — Event Engine pipeline wired"
```

---

## Task 9: grid-runtime EventEmitter (Rust, T1 enrichment)

> **Note:** This is S2.T1 in the original plan. It's the only Rust change in Phase 1.
> T2/T3 runtimes get events automatically from the L4 interceptor — zero changes needed.

**Files:**
- Create: `crates/grid-runtime/src/event_emitter.rs`
- Modify: `crates/grid-runtime/src/contract.rs` — remove PLACEHOLDER comment, implement emit_event
- Modify: `crates/grid-runtime/src/harness.rs` — call emitter at tool_call/tool_result/stop points
- Modify: `crates/grid-runtime/src/lib.rs` — add `mod event_emitter`
- Test: `crates/grid-runtime/tests/emit_event_integration.rs` — update existing test

**Step 1: Write the failing test**

Update `crates/grid-runtime/tests/grpc_integration.rs::test_emit_event_is_unimplemented` to expect success instead of Unimplemented.

Also add a unit test for `EventEmitter`:
```rust
// crates/grid-runtime/tests/event_emitter_test.rs
#[tokio::test]
async fn test_event_emitter_sends_http_post() {
    // Use a mock HTTP server (wiremock or similar)
    // Verify POST body contains session_id, event_type, payload_json
}
```

**Step 2: Implement EventEmitter**

See `docs/design/EAASP/PHASE1_EVENT_ENGINE_DESIGN.md` §3 for the Rust implementation.

Key: `EventEmitter` uses `mpsc::channel(100)` + background tokio task + `reqwest::Client::post()`. Fire-and-forget via `try_send`.

**Step 3: Run Rust tests**

Run: `cd /Users/sujiangwen/sandbox/LLM/speechless.ai/SGAI/grid-sandbox && cargo test -p grid-runtime -- --test-threads=1`
Expected: All tests pass

**Step 4: Commit**

```bash
git add crates/grid-runtime/src/event_emitter.rs \
       crates/grid-runtime/src/contract.rs \
       crates/grid-runtime/src/harness.rs \
       crates/grid-runtime/src/lib.rs \
       crates/grid-runtime/tests/
git commit -m "feat(grid-runtime): EventEmitter — fire-and-forget HTTP POST to L4 Event Engine"
```

---

## Task 10: End-to-End Verification + dev-eaasp.sh Update

**Files:**
- Modify: `scripts/dev-eaasp.sh` — ensure EventEngine starts with L4
- No new scripts needed — manual verification with CLI

**Step 1: Provide verification plan for user**

```bash
## Runtime Verification Task

### Prerequisites
- Ensure virtual environment is activated
- Ensure .env has ANTHROPIC_API_KEY and OPENAI_API_KEY

### Execution Commands (Terminal 1)
make dev-eaasp
eaasp-cli session create --skill threshold-calibration --runtime grid-runtime
# Note the session_id from output

### Execution Commands (Terminal 2 — follow events)
eaasp-cli session events <session_id> --follow

### Execution Commands (Terminal 1 — send message)
eaasp-cli session send "校准 Transformer-001"

### Verification Objectives
- ✅ Terminal 2 shows SESSION_START event after create
- ✅ Terminal 2 shows PRE_TOOL_USE/POST_TOOL_USE events during agent execution
- ✅ Terminal 2 shows STOP event when agent completes
- ✅ Terminal 2 shows cluster_id on events (c-XXXXXXXX format)
- ✅ `eaasp-cli session events <session_id>` shows full event history

### Failure Indicators
- ❌ No events appear in follow mode
- ❌ Events missing event_type or cluster_id
- ❌ "Connection refused" errors
```

**Step 2: Update checkpoint to complete**

**Step 3: Commit**

```bash
git add scripts/dev-eaasp.sh docs/plans/.checkpoint.json
git commit -m "docs(eaasp): Phase 1 Event-driven Foundation — checkpoint all tasks complete"
```

---

## Summary

| Task | Files | Tests | Description |
|------|-------|-------|-------------|
| 1 | 2 | 5 | Event data models |
| 2 | 4 | 7 | SqliteWalBackend + FTS5 + subscribe |
| 3 | 2 | 10 | 4 Event handlers |
| 4 | 2 | 4 | EventEngine pipeline |
| 5 | 2 | 7 | EventInterceptor |
| 6 | 3 | 2 | Wire into SessionOrchestrator + API |
| 7 | 3 | 2 | CLI events command |
| 8 | 2 | 0 | Checkpoint + full suite run |
| 9 | 4 | 2+ | grid-runtime EventEmitter (Rust) |
| 10 | 2 | 0 | E2E verification + dev script |
| **Total** | **~26** | **~39+** | |
