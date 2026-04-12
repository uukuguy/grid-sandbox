"""Tests for SessionOrchestrator (three-way handshake + send_message)."""

from __future__ import annotations

from typing import Any

import httpx
import pytest
import respx

from eaasp_l4_orchestration.event_stream import SessionEventStream
from eaasp_l4_orchestration.handshake import L2Client, L3Client, UpstreamError
from eaasp_l4_orchestration.session_orchestrator import (
    SessionNotFound,
    SessionOrchestrator,
)

L2_BASE = "http://l2.test"
L3_BASE = "http://l3.test"


class _StubL1:
    def __init__(self, runtime_id: str) -> None:
        self.runtime_id = runtime_id

    async def initialize(self, payload_dict: dict[str, Any]) -> dict[str, str]:
        sid = payload_dict.get("session_id", "mock")
        return {"session_id": sid, "runtime_id": self.runtime_id}

    async def send(self, session_id: str, content: str, message_type: str = "text"):
        yield {"chunk_type": "text_delta", "content": content}
        yield {"chunk_type": "done", "content": ""}

    async def terminate(self) -> None:
        pass

    async def close(self) -> None:
        pass


async def _make_orchestrator(
    tmp_db_path: str, http_client: httpx.AsyncClient
) -> SessionOrchestrator:
    l2 = L2Client(http_client, base_url=L2_BASE)
    l3 = L3Client(http_client, base_url=L3_BASE)
    stream = SessionEventStream(tmp_db_path)
    return SessionOrchestrator(
        tmp_db_path, l2=l2, l3=l3, event_stream=stream,
        l1_factory=lambda rid: _StubL1(rid),
    )


@respx.mock
async def test_create_session_happy_path(tmp_db_path: str) -> None:
    respx.post(f"{L2_BASE}/api/v1/memory/search").mock(
        return_value=httpx.Response(
            200,
            json={
                "hits": [
                    {"memory_id": "m1", "memory_type": "anchor", "score": 0.9},
                    {"memory_id": "m2", "memory_type": "file", "score": 0.6},
                ]
            },
        )
    )
    respx.post(url__regex=rf"{L3_BASE}/v1/sessions/.*/validate").mock(
        return_value=httpx.Response(
            200,
            json={
                "session_id": "placeholder",
                "hooks_to_attach": [
                    {"hook_id": "h1", "phase": "PreToolUse", "mode": "enforce"}
                ],
                "managed_settings_version": 3,
                "validated_at": "2026-04-12 01:00:00",
                "runtime_tier": "strict",
            },
        )
    )

    async with httpx.AsyncClient() as client:
        orch = await _make_orchestrator(tmp_db_path, client)
        out = await orch.create_session(
            intent_text="hire a new SDE",
            skill_id="skill.hr.onboard",
            runtime_pref="strict",
            user_id="u-1",
        )

    assert out["session_id"].startswith("sess_")
    assert out["status"] == "active"  # Phase 0.5: L1 Initialize → active
    payload = out["payload"]
    assert len(payload["memory_refs"]) == 2
    assert payload["memory_refs"][0]["memory_id"] == "m1"
    assert len(payload["policy_context"]["hooks"]) == 1
    assert payload["policy_context"]["policy_version"] == "3"
    # Sessions row persisted as "active" after L1 Initialize.
    fetched = await orch.get_session(out["session_id"])
    assert fetched["status"] == "active"
    # Boot events present: SESSION_CREATED + RUNTIME_INITIALIZED.
    events = await orch.list_events(out["session_id"])
    types = [e["event_type"] for e in events]
    assert "SESSION_CREATED" in types
    assert "RUNTIME_INITIALIZED" in types
    # N3 (reviewer): enforce boot-event ordering — SESSION_CREATED must
    # always land at a lower seq than RUNTIME_INITIALIZED so that
    # consumers replaying the stream see handshake completion before the
    # runtime initialization marker.
    seq_created = next(
        e["seq"] for e in events if e["event_type"] == "SESSION_CREATED"
    )
    seq_init = next(
        e["seq"] for e in events if e["event_type"] == "RUNTIME_INITIALIZED"
    )
    assert seq_created < seq_init


@respx.mock
async def test_create_session_l2_unavailable(tmp_db_path: str) -> None:
    respx.post(f"{L2_BASE}/api/v1/memory/search").mock(
        side_effect=httpx.ConnectError("no l2")
    )
    async with httpx.AsyncClient() as client:
        orch = await _make_orchestrator(tmp_db_path, client)
        with pytest.raises(UpstreamError) as exc_info:
            await orch.create_session(
                intent_text="x",
                skill_id="skill.s",
                runtime_pref="strict",
            )
    assert exc_info.value.service == "l2"
    assert exc_info.value.kind == "unavailable"


@respx.mock
async def test_create_session_l3_no_policy(tmp_db_path: str) -> None:
    respx.post(f"{L2_BASE}/api/v1/memory/search").mock(
        return_value=httpx.Response(200, json={"hits": []})
    )
    respx.post(url__regex=rf"{L3_BASE}/v1/sessions/.*/validate").mock(
        return_value=httpx.Response(
            404, json={"detail": {"code": "no_policy", "message": "empty"}}
        )
    )
    async with httpx.AsyncClient() as client:
        orch = await _make_orchestrator(tmp_db_path, client)
        with pytest.raises(UpstreamError) as exc_info:
            await orch.create_session(
                intent_text="x",
                skill_id="skill.s",
                runtime_pref="strict",
            )
    assert exc_info.value.service == "l3"
    assert exc_info.value.kind == "no_policy"


@respx.mock
async def test_send_message_happy_path(tmp_db_path: str) -> None:
    respx.post(f"{L2_BASE}/api/v1/memory/search").mock(
        return_value=httpx.Response(200, json={"hits": []})
    )
    respx.post(url__regex=rf"{L3_BASE}/v1/sessions/.*/validate").mock(
        return_value=httpx.Response(
            200,
            json={
                "session_id": "placeholder",
                "hooks_to_attach": [],
                "managed_settings_version": 1,
                "validated_at": "2026-04-12 01:00:00",
                "runtime_tier": "strict",
            },
        )
    )
    async with httpx.AsyncClient() as client:
        orch = await _make_orchestrator(tmp_db_path, client)
        created = await orch.create_session(
            intent_text="x", skill_id="skill.s", runtime_pref="strict"
        )
        sid = created["session_id"]
        result = await orch.send_message(sid, "hello world")

    assert result["session_id"] == sid
    assert "response_text" in result  # Phase 0.5: real L1 Send
    assert any(e["event_type"] == "USER_MESSAGE" for e in result["events"])

    events = await orch.list_events(sid)
    types = [e["event_type"] for e in events]
    assert "USER_MESSAGE" in types
    assert "RESPONSE_CHUNK" in types  # Phase 0.5: replaces RUNTIME_SEND_STUBBED


async def test_send_message_unknown_session_raises(tmp_db_path: str) -> None:
    async with httpx.AsyncClient() as client:
        orch = await _make_orchestrator(tmp_db_path, client)
        with pytest.raises(SessionNotFound):
            await orch.send_message("sess_nope", "hi")


async def test_get_session_unknown_raises(tmp_db_path: str) -> None:
    async with httpx.AsyncClient() as client:
        orch = await _make_orchestrator(tmp_db_path, client)
        with pytest.raises(SessionNotFound):
            await orch.get_session("sess_nope")


async def test_list_events_unknown_raises(tmp_db_path: str) -> None:
    async with httpx.AsyncClient() as client:
        orch = await _make_orchestrator(tmp_db_path, client)
        with pytest.raises(SessionNotFound):
            await orch.list_events("sess_nope")


@respx.mock
async def test_stream_message_yields_chunks(tmp_db_path: str) -> None:
    """stream_message should yield chunk events then a done event."""
    respx.post(f"{L2_BASE}/api/v1/memory/search").mock(
        return_value=httpx.Response(200, json={"hits": []})
    )
    respx.post(url__regex=rf"{L3_BASE}/v1/sessions/.*/validate").mock(
        return_value=httpx.Response(
            200,
            json={
                "session_id": "placeholder",
                "hooks_to_attach": [],
                "managed_settings_version": 1,
                "validated_at": "2026-04-12 01:00:00",
                "runtime_tier": "strict",
            },
        )
    )
    async with httpx.AsyncClient() as client:
        orch = await _make_orchestrator(tmp_db_path, client)
        created = await orch.create_session(
            intent_text="x", skill_id="skill.s", runtime_pref="strict"
        )
        sid = created["session_id"]

        messages: list[dict[str, Any]] = []
        async for msg in orch.stream_message(sid, "hello stream"):
            messages.append(msg)

    # Should have at least 2 chunk events + 1 done event.
    chunk_msgs = [m for m in messages if m["event"] == "chunk"]
    done_msgs = [m for m in messages if m["event"] == "done"]
    assert len(chunk_msgs) >= 2  # text_delta + done chunk
    assert len(done_msgs) == 1
    assert done_msgs[0]["data"]["session_id"] == sid
    assert "response_text" in done_msgs[0]["data"]


async def test_stream_message_unknown_session_yields_nothing(tmp_db_path: str) -> None:
    """stream_message on non-existent session should raise SessionNotFound."""
    async with httpx.AsyncClient() as client:
        orch = await _make_orchestrator(tmp_db_path, client)
        with pytest.raises(SessionNotFound):
            async for _ in orch.stream_message("sess_ghost", "hi"):
                pass
