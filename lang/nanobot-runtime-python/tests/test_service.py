"""W2.T4 service tests — direct method calls (no network gRPC).

Tests instantiate NanobotRuntimeService directly and invoke methods
with a MockContext to verify shape-correct responses without a live
gRPC server or LLM endpoint.
"""
from __future__ import annotations

import asyncio

import grpc
import pytest

from nanobot_runtime._proto.eaasp.runtime.v2 import common_pb2, runtime_pb2
from nanobot_runtime.service import NanobotRuntimeService, _RUNTIME_ID


class MockContext:
    """Minimal gRPC context shim for unit tests."""

    def __init__(self) -> None:
        self._code: grpc.StatusCode | None = None
        self._details: str = ""

    def set_code(self, code: grpc.StatusCode) -> None:
        self._code = code

    def set_details(self, details: str) -> None:
        self._details = details


# ---------------------------------------------------------------------------
# Test 1: Health returns healthy=True with a non-empty runtime_id
# ---------------------------------------------------------------------------

async def test_health_returns_healthy():
    svc = NanobotRuntimeService()
    resp = await svc.Health(common_pb2.Empty(), MockContext())
    assert resp.healthy is True
    assert resp.runtime_id == _RUNTIME_ID
    assert len(resp.runtime_id) > 0


# ---------------------------------------------------------------------------
# Test 2: Initialize with explicit session_id returns that session_id
# ---------------------------------------------------------------------------

async def test_initialize_returns_session_id():
    svc = NanobotRuntimeService()
    payload = common_pb2.SessionPayload(session_id="test-session-42")
    req = runtime_pb2.InitializeRequest(payload=payload)
    resp = await svc.Initialize(req, MockContext())
    assert resp.session_id == "test-session-42"
    assert resp.runtime_id == _RUNTIME_ID


# ---------------------------------------------------------------------------
# Test 3: Initialize without session_id auto-generates one
# ---------------------------------------------------------------------------

async def test_initialize_autogenerates_session_id():
    svc = NanobotRuntimeService()
    req = runtime_pb2.InitializeRequest(payload=common_pb2.SessionPayload())
    resp = await svc.Initialize(req, MockContext())
    assert len(resp.session_id) > 0
    assert resp.runtime_id == _RUNTIME_ID


# ---------------------------------------------------------------------------
# Test 4: GetCapabilities returns tier="aligned"
# ---------------------------------------------------------------------------

async def test_get_capabilities_tier():
    svc = NanobotRuntimeService()
    resp = await svc.GetCapabilities(common_pb2.Empty(), MockContext())
    assert resp.tier == "aligned"
    assert resp.runtime_id == _RUNTIME_ID


# ---------------------------------------------------------------------------
# Test 5: Terminate with no prior Initialize does not raise
# ---------------------------------------------------------------------------

async def test_terminate_no_session_does_not_raise():
    svc = NanobotRuntimeService()
    ctx = MockContext()
    resp = await svc.Terminate(common_pb2.Empty(), ctx)
    # Should return Empty without error; active_session_id remains None
    assert svc._active_session_id is None


# ---------------------------------------------------------------------------
# Test 6: OnToolCall returns "allow" decision stub
# ---------------------------------------------------------------------------

async def test_on_tool_call_returns_allow():
    svc = NanobotRuntimeService()
    event = runtime_pb2.ToolCallEvent(
        session_id="s1",
        tool_name="bash",
        tool_id="tc-1",
        input_json="{}",
    )
    ack = await svc.OnToolCall(event, MockContext())
    assert ack.decision == "allow"


# ---------------------------------------------------------------------------
# Test 7: OnToolResult returns "allow" decision stub
# ---------------------------------------------------------------------------

async def test_on_tool_result_returns_allow():
    svc = NanobotRuntimeService()
    event = runtime_pb2.ToolResultEvent(
        session_id="s1",
        tool_name="bash",
        tool_id="tc-1",
        output="ok",
    )
    ack = await svc.OnToolResult(event, MockContext())
    assert ack.decision == "allow"


# ---------------------------------------------------------------------------
# Test 8: EmitEvent sets UNIMPLEMENTED status
# ---------------------------------------------------------------------------

async def test_emit_event_sets_unimplemented():
    svc = NanobotRuntimeService()
    entry = runtime_pb2.EventStreamEntry(session_id="s1")
    ctx = MockContext()
    await svc.EmitEvent(entry, ctx)
    assert ctx._code == grpc.StatusCode.UNIMPLEMENTED
