"""Tests for SessionOrchestrator (three-way handshake + send_message)."""

from __future__ import annotations

from typing import Any

import httpx
import pytest
import respx

from eaasp_l4_orchestration.event_stream import SessionEventStream
from eaasp_l4_orchestration.handshake import L2Client, L3Client, SkillRegistryClient, UpstreamError
from eaasp_l4_orchestration.mcp_resolver import McpResolver
from eaasp_l4_orchestration.session_orchestrator import (
    SessionNotFound,
    SessionOrchestrator,
)

L2_BASE = "http://l2.test"
L3_BASE = "http://l3.test"
SKILL_REG_BASE = "http://skill-reg.test"
MCP_ORCH_BASE = "http://mcp-orch.test"


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


# ── MCP integration tests (S2.T2) ────────────────────────────────────────────


class _McpTrackingL1:
    """Stub L1 that records connect_mcp calls for assertion."""

    def __init__(self, runtime_id: str) -> None:
        self.runtime_id = runtime_id
        self.connect_mcp_calls: list[dict[str, Any]] = []

    async def initialize(self, payload_dict: dict[str, Any]) -> dict[str, str]:
        sid = payload_dict.get("session_id", "mock")
        return {"session_id": sid, "runtime_id": self.runtime_id}

    async def send(self, session_id: str, content: str, message_type: str = "text"):
        yield {"chunk_type": "done", "content": ""}

    async def connect_mcp(self, session_id: str, servers: list[dict]) -> dict:
        self.connect_mcp_calls.append({"session_id": session_id, "servers": servers})
        return {
            "success": True,
            "connected": [s["name"] for s in servers],
            "failed": [],
        }

    async def terminate(self) -> None:
        pass

    async def close(self) -> None:
        pass


def _mock_l2_l3_basics() -> None:
    """Set up standard L2 memory search + L3 validate mocks."""
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
                "validated_at": "2026-04-13T00:00:00Z",
                "runtime_tier": "strict",
            },
        ),
    )


def _mock_skill_with_mcp_deps() -> None:
    """Mock skill registry returning a skill with mcp: dependencies."""
    respx.post(f"{SKILL_REG_BASE}/tools/skill_read/invoke").mock(
        return_value=httpx.Response(
            200,
            json={
                "meta": {"name": "threshold-calibration"},
                "frontmatter_yaml": "",
                "prose": "calibrate thresholds",
                "parsed_v2": {
                    "scoped_hooks": {},
                    "dependencies": ["mcp:mock-scada", "mcp:eaasp-l2-memory"],
                    "runtime_affinity": ["grid-runtime"],
                    "access_scope": "org:eaasp-mvp",
                },
            },
        ),
    )


def _mock_skill_no_mcp_deps() -> None:
    """Mock skill registry returning a skill with no mcp: dependencies."""
    respx.post(f"{SKILL_REG_BASE}/tools/skill_read/invoke").mock(
        return_value=httpx.Response(
            200,
            json={
                "meta": {"name": "simple-skill"},
                "frontmatter_yaml": "",
                "prose": "a simple skill",
                "parsed_v2": {
                    "scoped_hooks": {},
                    "dependencies": ["pip:numpy"],
                    "runtime_affinity": ["grid-runtime"],
                    "access_scope": "",
                },
            },
        ),
    )


@respx.mock
async def test_create_session_with_mcp_dependencies(tmp_db_path: str) -> None:
    """create_session queries L2 MCP Orchestrator and calls ConnectMCP after Initialize."""
    _mock_l2_l3_basics()
    _mock_skill_with_mcp_deps()

    # Mock L2 MCP resolve
    respx.post(f"{MCP_ORCH_BASE}/v1/mcp/resolve").mock(
        return_value=httpx.Response(
            200,
            json={
                "servers": [
                    {"name": "mock-scada", "transport": "stdio", "command": "mock-scada", "args": []},
                    {"name": "eaasp-l2-memory", "transport": "stdio", "command": "l2-memory", "args": []},
                ],
            },
        ),
    )

    # Track connect_mcp via a shared L1 instance.
    shared_l1 = _McpTrackingL1("grid-runtime")

    async with httpx.AsyncClient() as client:
        l2 = L2Client(client, base_url=L2_BASE)
        l3 = L3Client(client, base_url=L3_BASE)
        skill_reg = SkillRegistryClient(client, base_url=SKILL_REG_BASE)
        mcp_resolver = McpResolver(client, base_url=MCP_ORCH_BASE)
        stream = SessionEventStream(tmp_db_path)

        orch = SessionOrchestrator(
            tmp_db_path,
            l2=l2,
            l3=l3,
            event_stream=stream,
            skill_registry=skill_reg,
            l1_factory=lambda _rid: shared_l1,
            mcp_resolver=mcp_resolver,
        )
        out = await orch.create_session(
            intent_text="calibrate sensor thresholds",
            skill_id="threshold-calibration",
            runtime_pref="grid-runtime",
            user_id="u-mcp",
        )

    # Session should be active.
    assert out["status"] == "active"

    # connect_mcp should have been called once with the resolved servers.
    assert len(shared_l1.connect_mcp_calls) == 1
    call = shared_l1.connect_mcp_calls[0]
    # L1 session_id comes from initialize() response.
    assert call["session_id"] == out["session_id"]
    assert len(call["servers"]) == 2
    server_names = [s["name"] for s in call["servers"]]
    assert "mock-scada" in server_names
    assert "eaasp-l2-memory" in server_names

    # SESSION_MCP_CONNECTED event should be present.
    events = await orch.list_events(out["session_id"])
    event_types = [e["event_type"] for e in events]
    assert "SESSION_MCP_CONNECTED" in event_types
    mcp_event = next(e for e in events if e["event_type"] == "SESSION_MCP_CONNECTED")
    mcp_payload = mcp_event["payload"]
    assert set(mcp_payload["connected"]) == {"mock-scada", "eaasp-l2-memory"}
    assert mcp_payload["failed"] == []


@respx.mock
async def test_create_session_mcp_resolve_failure_non_fatal(tmp_db_path: str) -> None:
    """MCP resolve failure should not block session creation — session stays active."""
    _mock_l2_l3_basics()
    _mock_skill_with_mcp_deps()

    # MCP resolve returns 500.
    respx.post(f"{MCP_ORCH_BASE}/v1/mcp/resolve").mock(
        return_value=httpx.Response(500, json={"error": "internal server error"})
    )

    shared_l1 = _McpTrackingL1("grid-runtime")

    async with httpx.AsyncClient() as client:
        l2 = L2Client(client, base_url=L2_BASE)
        l3 = L3Client(client, base_url=L3_BASE)
        skill_reg = SkillRegistryClient(client, base_url=SKILL_REG_BASE)
        mcp_resolver = McpResolver(client, base_url=MCP_ORCH_BASE)
        stream = SessionEventStream(tmp_db_path)

        orch = SessionOrchestrator(
            tmp_db_path,
            l2=l2,
            l3=l3,
            event_stream=stream,
            skill_registry=skill_reg,
            l1_factory=lambda _rid: shared_l1,
            mcp_resolver=mcp_resolver,
        )
        out = await orch.create_session(
            intent_text="calibrate",
            skill_id="threshold-calibration",
            runtime_pref="grid-runtime",
        )

    # Session should still be active despite MCP failure.
    assert out["status"] == "active"

    # connect_mcp should NOT have been called (resolve failed before it).
    assert len(shared_l1.connect_mcp_calls) == 0

    # SESSION_MCP_CONNECT_FAILED event should be present.
    events = await orch.list_events(out["session_id"])
    event_types = [e["event_type"] for e in events]
    assert "SESSION_MCP_CONNECT_FAILED" in event_types
    fail_event = next(e for e in events if e["event_type"] == "SESSION_MCP_CONNECT_FAILED")
    assert "error" in fail_event["payload"]


@respx.mock
async def test_create_session_no_mcp_deps_skips_resolve(tmp_db_path: str) -> None:
    """When skill has no mcp: deps, McpResolver.resolve is not called."""
    _mock_l2_l3_basics()
    _mock_skill_no_mcp_deps()

    # MCP resolve route — should NOT be called.
    mcp_route = respx.post(f"{MCP_ORCH_BASE}/v1/mcp/resolve").mock(
        return_value=httpx.Response(200, json={"servers": []})
    )

    shared_l1 = _McpTrackingL1("grid-runtime")

    async with httpx.AsyncClient() as client:
        l2 = L2Client(client, base_url=L2_BASE)
        l3 = L3Client(client, base_url=L3_BASE)
        skill_reg = SkillRegistryClient(client, base_url=SKILL_REG_BASE)
        mcp_resolver = McpResolver(client, base_url=MCP_ORCH_BASE)
        stream = SessionEventStream(tmp_db_path)

        orch = SessionOrchestrator(
            tmp_db_path,
            l2=l2,
            l3=l3,
            event_stream=stream,
            skill_registry=skill_reg,
            l1_factory=lambda _rid: shared_l1,
            mcp_resolver=mcp_resolver,
        )
        out = await orch.create_session(
            intent_text="simple task",
            skill_id="simple-skill",
            runtime_pref="grid-runtime",
        )

    assert out["status"] == "active"
    # MCP resolve endpoint should NOT have been called.
    assert mcp_route.call_count == 0
    # connect_mcp should NOT have been called.
    assert len(shared_l1.connect_mcp_calls) == 0
    # No MCP events should exist.
    events = await orch.list_events(out["session_id"])
    event_types = [e["event_type"] for e in events]
    assert "SESSION_MCP_CONNECTED" not in event_types
    assert "SESSION_MCP_CONNECT_FAILED" not in event_types


@respx.mock
async def test_create_session_no_mcp_resolver_skips_mcp(tmp_db_path: str) -> None:
    """When mcp_resolver is None, ConnectMCP step is skipped entirely."""
    _mock_l2_l3_basics()
    _mock_skill_with_mcp_deps()

    # No MCP route mocked — if resolver tried to call, it would fail.
    shared_l1 = _McpTrackingL1("grid-runtime")

    async with httpx.AsyncClient() as client:
        l2 = L2Client(client, base_url=L2_BASE)
        l3 = L3Client(client, base_url=L3_BASE)
        skill_reg = SkillRegistryClient(client, base_url=SKILL_REG_BASE)
        stream = SessionEventStream(tmp_db_path)

        orch = SessionOrchestrator(
            tmp_db_path,
            l2=l2,
            l3=l3,
            event_stream=stream,
            skill_registry=skill_reg,
            l1_factory=lambda _rid: shared_l1,
            mcp_resolver=None,  # Explicitly no resolver.
        )
        out = await orch.create_session(
            intent_text="calibrate",
            skill_id="threshold-calibration",
            runtime_pref="grid-runtime",
        )

    assert out["status"] == "active"
    assert len(shared_l1.connect_mcp_calls) == 0
