"""Tests for session state machine + L1 gRPC integration in orchestrator.

Phase 0.5 S1.T2/T3: Tests that create_session calls L1 Initialize,
send_message streams L1 Send, and close_session terminates properly.
Uses mock L1 client (no real gRPC).
"""

from __future__ import annotations

import json
from typing import Any
from unittest.mock import AsyncMock, MagicMock

import pytest
import pytest_asyncio
import respx

from eaasp_l4_orchestration.db import connect, init_db
from eaasp_l4_orchestration.event_stream import SessionEventStream
from eaasp_l4_orchestration.handshake import L2Client, L3Client
from eaasp_l4_orchestration.l1_client import L1RuntimeClient, L1RuntimeError
from eaasp_l4_orchestration.session_orchestrator import (
    InvalidStateTransition,
    SessionOrchestrator,
)


# ── Fixtures ────────────────────────────────────────────────────────────────


class MockL1Client:
    """In-memory mock of L1RuntimeClient for unit tests."""

    def __init__(self, runtime_id: str = "grid-runtime") -> None:
        self.runtime_id = runtime_id
        self.initialized = False
        self.terminated = False
        self.sent_messages: list[str] = []

    async def initialize(self, payload_dict: dict[str, Any]) -> dict[str, str]:
        self.initialized = True
        sid = payload_dict.get("session_id", "mock_session")
        return {"session_id": sid, "runtime_id": self.runtime_id}

    async def send(
        self, session_id: str, content: str, message_type: str = "text"
    ):
        self.sent_messages.append(content)
        # Yield mock response chunks.
        yield {"chunk_type": "text_delta", "content": f"Echo: {content}"}
        yield {"chunk_type": "done", "content": ""}

    async def terminate(self) -> None:
        self.terminated = True

    async def close(self) -> None:
        pass


class FailingL1Client(MockL1Client):
    """L1 client that fails on Initialize."""

    async def initialize(self, payload_dict: dict[str, Any]) -> dict[str, str]:
        raise L1RuntimeError("grid-runtime", "Initialize", "connection refused")


@pytest_asyncio.fixture
async def tmp_db(tmp_path):
    db_path = str(tmp_path / "test.db")
    await init_db(db_path)
    return db_path


@pytest_asyncio.fixture
async def orchestrator_with_mock_l1(tmp_db):
    """Build orchestrator with mock L2/L3/L1 clients."""
    import httpx

    mock_l1_instances: dict[str, MockL1Client] = {}

    def mock_l1_factory(runtime_id: str) -> MockL1Client:
        client = MockL1Client(runtime_id)
        mock_l1_instances[runtime_id] = client
        return client

    async with httpx.AsyncClient(timeout=5.0) as client:
        l2 = L2Client(client, base_url="http://l2.test:18085")
        l3 = L3Client(client, base_url="http://l3.test:18083")
        es = SessionEventStream(tmp_db)
        orch = SessionOrchestrator(
            tmp_db, l2=l2, l3=l3, event_stream=es, l1_factory=mock_l1_factory
        )
        yield orch, mock_l1_instances


# ── Tests: create_session with L1 Initialize ────────────────────────────────


@pytest.mark.asyncio
@respx.mock
async def test_create_session_calls_l1_initialize(orchestrator_with_mock_l1):
    orch, l1_instances = orchestrator_with_mock_l1

    # Mock L2 and L3 responses.
    respx.post("http://l2.test:18085/api/v1/memory/search").mock(
        return_value=respx.MockResponse(200, json={"hits": []})
    )
    respx.post("http://l3.test:18083/v1/sessions/sess_test/validate").mock(
        return_value=respx.MockResponse(
            200,
            json={
                "hooks_to_attach": [],
                "managed_settings_version": 1,
                "validated_at": "2026-04-12",
            },
        ),
    )
    # respx intercepts all, so the l3 URL pattern needs to be flexible.
    respx.post().mock(
        return_value=respx.MockResponse(
            200,
            json={
                "hooks_to_attach": [],
                "managed_settings_version": 1,
                "validated_at": "2026-04-12",
            },
        ),
    )

    result = await orch.create_session(
        intent_text="calibrate Transformer-001",
        skill_id="threshold-calibration",
        runtime_pref="grid-runtime",
    )

    assert result["status"] == "active"
    assert "grid-runtime" in l1_instances
    assert l1_instances["grid-runtime"].initialized is True

    # Verify events: SESSION_CREATED + RUNTIME_INITIALIZED.
    events = await orch.list_events(result["session_id"])
    event_types = [e["event_type"] for e in events]
    assert "SESSION_CREATED" in event_types
    assert "RUNTIME_INITIALIZED" in event_types
    assert "RUNTIME_INITIALIZE_STUBBED" not in event_types


@pytest.mark.asyncio
@respx.mock
async def test_create_session_l1_failure_marks_failed(tmp_db):
    """When L1 Initialize fails, session status → 'failed'."""
    import httpx

    def failing_factory(runtime_id: str):
        return FailingL1Client(runtime_id)

    async with httpx.AsyncClient(timeout=5.0) as client:
        l2 = L2Client(client, base_url="http://l2.test:18085")
        l3 = L3Client(client, base_url="http://l3.test:18083")
        es = SessionEventStream(tmp_db)
        orch = SessionOrchestrator(
            tmp_db, l2=l2, l3=l3, event_stream=es, l1_factory=failing_factory
        )

        respx.post().mock(
            return_value=respx.MockResponse(200, json={"hits": [], "hooks_to_attach": [], "managed_settings_version": 1, "validated_at": "now"})
        )

        with pytest.raises(L1RuntimeError, match="connection refused"):
            await orch.create_session(
                intent_text="test",
                skill_id="test-skill",
                runtime_pref="grid-runtime",
            )

    # Verify session is marked as 'failed'.
    db = await connect(tmp_db)
    try:
        cur = await db.execute("SELECT status FROM sessions LIMIT 1")
        row = await cur.fetchone()
    finally:
        await db.close()
    assert row is not None
    assert row["status"] == "failed"


# ── Tests: send_message with L1 Send streaming ─────────────────────────────


@pytest.mark.asyncio
@respx.mock
async def test_send_message_streams_l1_response(orchestrator_with_mock_l1):
    orch, l1_instances = orchestrator_with_mock_l1

    # Create a session first.
    respx.post().mock(
        return_value=respx.MockResponse(200, json={"hits": [], "hooks_to_attach": [], "managed_settings_version": 1, "validated_at": "now"})
    )
    session = await orch.create_session(
        intent_text="test", skill_id="s1", runtime_pref="grid-runtime"
    )
    sid = session["session_id"]

    # Now send a message.
    result = await orch.send_message(sid, "Hello agent")

    assert result["session_id"] == sid
    assert "Echo: Hello agent" in result["response_text"]
    assert len(result["chunks"]) == 2  # text_delta + done
    assert l1_instances["grid-runtime"].sent_messages == ["Hello agent"]

    # Verify events.
    events = await orch.list_events(sid)
    event_types = [e["event_type"] for e in events]
    assert "USER_MESSAGE" in event_types
    assert "RESPONSE_CHUNK" in event_types
    assert "RUNTIME_SEND_STUBBED" not in event_types


# ── Tests: session state machine ────────────────────────────────────────────


@pytest.mark.asyncio
@respx.mock
async def test_close_session(orchestrator_with_mock_l1):
    orch, l1_instances = orchestrator_with_mock_l1

    respx.post().mock(
        return_value=respx.MockResponse(200, json={"hits": [], "hooks_to_attach": [], "managed_settings_version": 1, "validated_at": "now"})
    )
    session = await orch.create_session(
        intent_text="test", skill_id="s1", runtime_pref="grid-runtime"
    )
    sid = session["session_id"]
    assert session["status"] == "active"

    result = await orch.close_session(sid)
    assert result["status"] == "closed"
    assert l1_instances["grid-runtime"].terminated is True

    # Verify DB status.
    info = await orch.get_session(sid)
    assert info["status"] == "closed"
    assert info["closed_at"] is not None


@pytest.mark.asyncio
@respx.mock
async def test_close_already_closed_raises(orchestrator_with_mock_l1):
    orch, _ = orchestrator_with_mock_l1

    respx.post().mock(
        return_value=respx.MockResponse(200, json={"hits": [], "hooks_to_attach": [], "managed_settings_version": 1, "validated_at": "now"})
    )
    session = await orch.create_session(
        intent_text="test", skill_id="s1", runtime_pref="grid-runtime"
    )
    sid = session["session_id"]

    await orch.close_session(sid)

    with pytest.raises(InvalidStateTransition, match="closed"):
        await orch.close_session(sid)


@pytest.mark.asyncio
async def test_status_update_persists(tmp_db):
    """Direct _update_status helper works."""
    import httpx

    async with httpx.AsyncClient(timeout=5.0) as client:
        es = SessionEventStream(tmp_db)
        orch = SessionOrchestrator(
            tmp_db,
            l2=L2Client(client, base_url="http://l2:18085"),
            l3=L3Client(client, base_url="http://l3:18083"),
            event_stream=es,
        )

    # Seed a session row directly.
    db = await connect(tmp_db)
    try:
        await db.execute("BEGIN IMMEDIATE")
        await db.execute(
            "INSERT INTO sessions (session_id, status, payload_json, created_at) VALUES (?, ?, ?, ?)",
            ("sess_x", "created", "{}", 1000),
        )
        await db.commit()
    finally:
        await db.close()

    await orch._update_status("sess_x", "active")
    db = await connect(tmp_db)
    try:
        cur = await db.execute("SELECT status, closed_at FROM sessions WHERE session_id = 'sess_x'")
        row = await cur.fetchone()
    finally:
        await db.close()
    assert row["status"] == "active"
    assert row["closed_at"] is None

    await orch._update_status("sess_x", "failed")
    db = await connect(tmp_db)
    try:
        cur = await db.execute("SELECT status, closed_at FROM sessions WHERE session_id = 'sess_x'")
        row = await cur.fetchone()
    finally:
        await db.close()
    assert row["status"] == "failed"
    assert row["closed_at"] is not None
