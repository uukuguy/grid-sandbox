"""Tests for gRPC RuntimeService — unit tests without real SDK calls."""

import asyncio
import json

import grpc
import grpc.aio
import pytest

from claude_code_runtime._proto.eaasp.common.v1 import common_pb2
from claude_code_runtime._proto.eaasp.runtime.v1 import (
    runtime_pb2,
    runtime_pb2_grpc,
)
from claude_code_runtime.config import RuntimeConfig
from claude_code_runtime.service import RuntimeServiceImpl


class FakeContext:
    """Minimal fake gRPC context for unit tests."""

    def __init__(self):
        self.code = None
        self.details = None

    def set_code(self, code):
        self.code = code

    def set_details(self, details):
        self.details = details


@pytest.fixture
def config():
    return RuntimeConfig(
        grpc_port=50099,
        runtime_id="test-runtime",
        runtime_name="Test Runtime",
        anthropic_model_name="test-model",
    )


@pytest.fixture
def service(config):
    return RuntimeServiceImpl(config)


@pytest.fixture
def ctx():
    return FakeContext()


@pytest.mark.asyncio
async def test_health(service, ctx):
    resp = await service.Health(common_pb2.Empty(), ctx)
    assert resp.healthy is True
    assert resp.runtime_id == "test-runtime"
    assert "sdk" in resp.checks


@pytest.mark.asyncio
async def test_get_capabilities(service, ctx):
    resp = await service.GetCapabilities(common_pb2.Empty(), ctx)
    assert resp.runtime_id == "test-runtime"
    assert resp.runtime_name == "Test Runtime"
    assert resp.tier == "harness"
    assert resp.model == "test-model"
    assert resp.native_hooks is True
    assert resp.requires_hook_bridge is False
    assert len(resp.supported_tools) > 0


@pytest.mark.asyncio
async def test_initialize(service, ctx):
    req = runtime_pb2.InitializeRequest(
        payload=runtime_pb2.SessionPayload(
            user_id="test-user",
            user_role="developer",
            org_unit="engineering",
        )
    )
    resp = await service.Initialize(req, ctx)
    assert resp.session_id.startswith("crt-")
    assert resp.session_id in service.sessions


@pytest.mark.asyncio
async def test_load_skill(service, ctx):
    # First initialize a session
    init_req = runtime_pb2.InitializeRequest(
        payload=runtime_pb2.SessionPayload(user_id="u1")
    )
    init_resp = await service.Initialize(init_req, ctx)
    sid = init_resp.session_id

    # Load skill
    req = runtime_pb2.LoadSkillRequest(
        session_id=sid,
        skill=runtime_pb2.SkillContent(
            skill_id="s-1",
            name="Test Skill",
            frontmatter_yaml="---\nname: test\n---",
            prose="Do something.",
        ),
    )
    resp = await service.LoadSkill(req, ctx)
    assert resp.success is True
    assert len(service.sessions[sid]["skills"]) == 1


@pytest.mark.asyncio
async def test_on_tool_call(service, ctx):
    req = common_pb2.ToolCallEvent(
        session_id="s-1",
        tool_name="bash",
        tool_id="t-1",
        input_json='{"command": "ls"}',
    )
    resp = await service.OnToolCall(req, ctx)
    assert resp.decision == "allow"


@pytest.mark.asyncio
async def test_on_tool_result(service, ctx):
    req = common_pb2.ToolResultEvent(
        session_id="s-1",
        tool_name="bash",
        tool_id="t-1",
        output="file.txt",
        is_error=False,
    )
    resp = await service.OnToolResult(req, ctx)
    assert resp.decision == "allow"


@pytest.mark.asyncio
async def test_on_stop(service, ctx):
    req = common_pb2.StopRequest(session_id="s-1")
    resp = await service.OnStop(req, ctx)
    assert resp.decision == "complete"


@pytest.mark.asyncio
async def test_connect_disconnect_mcp(service, ctx):
    # Initialize session
    init_resp = await service.Initialize(
        runtime_pb2.InitializeRequest(
            payload=runtime_pb2.SessionPayload(user_id="u1")
        ),
        ctx,
    )
    sid = init_resp.session_id

    # Connect MCP
    req = runtime_pb2.ConnectMcpRequest(
        session_id=sid,
        servers=[
            runtime_pb2.McpServerConfig(
                name="test-mcp", transport="stdio", command="echo"
            )
        ],
    )
    resp = await service.ConnectMcp(req, ctx)
    assert resp.success is True
    assert "test-mcp" in resp.connected

    # Disconnect MCP
    disc_resp = await service.DisconnectMcp(
        runtime_pb2.DisconnectMcpRequest(
            session_id=sid, server_name="test-mcp"
        ),
        ctx,
    )
    assert disc_resp.success is True


@pytest.mark.asyncio
async def test_get_state_and_restore(service, ctx):
    # Initialize
    init_resp = await service.Initialize(
        runtime_pb2.InitializeRequest(
            payload=runtime_pb2.SessionPayload(user_id="u1", user_role="dev")
        ),
        ctx,
    )
    sid = init_resp.session_id

    # GetState
    state_resp = await service.GetState(
        runtime_pb2.GetStateRequest(session_id=sid), ctx
    )
    assert state_resp.session_id == sid
    assert state_resp.state_format == "python-json"
    assert len(state_resp.state_data) > 0

    # RestoreState
    restore_resp = await service.RestoreState(state_resp, ctx)
    assert restore_resp.session_id == sid


@pytest.mark.asyncio
async def test_pause_resume(service, ctx):
    init_resp = await service.Initialize(
        runtime_pb2.InitializeRequest(
            payload=runtime_pb2.SessionPayload(user_id="u1")
        ),
        ctx,
    )
    sid = init_resp.session_id

    # Pause
    pause_resp = await service.PauseSession(
        runtime_pb2.PauseRequest(session_id=sid), ctx
    )
    assert pause_resp.success is True
    assert service.sessions[sid]["state"] == "paused"

    # Resume
    resume_resp = await service.ResumeSession(
        runtime_pb2.ResumeRequest(session_id=sid), ctx
    )
    assert resume_resp.success is True
    assert service.sessions[sid]["state"] == "active"


@pytest.mark.asyncio
async def test_emit_telemetry(service, ctx):
    init_resp = await service.Initialize(
        runtime_pb2.InitializeRequest(
            payload=runtime_pb2.SessionPayload(user_id="u1")
        ),
        ctx,
    )
    sid = init_resp.session_id

    # Add some telemetry
    service.sessions[sid]["telemetry"].append(
        {"event_type": "test", "timestamp": 1234567890}
    )

    resp = await service.EmitTelemetry(
        runtime_pb2.EmitTelemetryRequest(session_id=sid), ctx
    )
    assert len(resp.events) == 1
    assert resp.events[0].event_type == "test"


@pytest.mark.asyncio
async def test_terminate(service, ctx):
    init_resp = await service.Initialize(
        runtime_pb2.InitializeRequest(
            payload=runtime_pb2.SessionPayload(user_id="u1")
        ),
        ctx,
    )
    sid = init_resp.session_id

    resp = await service.Terminate(
        runtime_pb2.TerminateRequest(session_id=sid), ctx
    )
    assert resp.success is True
    assert sid not in service.sessions


@pytest.mark.asyncio
async def test_session_not_found(service, ctx):
    resp = await service.GetState(
        runtime_pb2.GetStateRequest(session_id="nonexistent"), ctx
    )
    assert ctx.code == grpc.StatusCode.NOT_FOUND
