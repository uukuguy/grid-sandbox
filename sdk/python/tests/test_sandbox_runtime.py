"""Tests for RuntimeSandbox and MultiRuntimeSandbox.

All tests mock gRPC interactions — no real gRPC server required.
"""

from __future__ import annotations

import asyncio
import sys
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from eaasp.models.message import ResponseChunk, UserMessage
from eaasp.models.session import SessionConfig
from eaasp.models.skill import Skill, SkillFrontmatter, ScopedHook
from eaasp.sandbox.base import HookFiredEvent, SandboxError, TelemetrySummary
from eaasp.sandbox.multi_runtime import (
    ComparisonResult,
    ConsistencyReport,
    MultiRuntimeSandbox,
)
from eaasp.sandbox.runtime import (
    RuntimeSandbox,
    _build_send_request,
    _build_session_payload,
    _build_skill_content,
    _parse_endpoint,
    _proto_chunk_to_response,
)


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture
def sample_skill() -> Skill:
    return Skill(
        frontmatter=SkillFrontmatter(
            name="test-skill",
            version="1.0.0",
            description="A test skill for sandbox testing",
            author="tester",
            tags=["test"],
            skill_type="workflow",
            scope="team",
        ),
        prose="You are a helpful test assistant that follows workflows carefully.",
    )


@pytest.fixture
def sample_config() -> SessionConfig:
    return SessionConfig(
        user_id="user-1",
        user_role="developer",
        org_unit="engineering",
        quotas={"max_turns": "10"},
        context={"env": "test"},
    )


@pytest.fixture
def sample_message() -> UserMessage:
    return UserMessage(content="Hello, run the workflow")


# ---------------------------------------------------------------------------
# _parse_endpoint tests
# ---------------------------------------------------------------------------


class TestParseEndpoint:
    def test_grpc_scheme(self):
        host, port = _parse_endpoint("grpc://localhost:50051")
        assert host == "localhost"
        assert port == 50051

    def test_plain_host_port(self):
        host, port = _parse_endpoint("10.0.0.1:50052")
        assert host == "10.0.0.1"
        assert port == 50052

    def test_default_port(self):
        host, port = _parse_endpoint("grpc://myhost")
        assert host == "myhost"
        assert port == 50051

    def test_plain_host_only(self):
        host, port = _parse_endpoint("myhost")
        assert host == "myhost"
        assert port == 50051


# ---------------------------------------------------------------------------
# Payload builder tests
# ---------------------------------------------------------------------------


class TestPayloadBuilders:
    def test_build_session_payload(self, sample_skill, sample_config):
        payload = _build_session_payload(sample_config, sample_skill)
        assert payload["user_id"] == "user-1"
        assert payload["user_role"] == "developer"
        assert payload["org_unit"] == "engineering"
        assert payload["quotas"] == {"max_turns": "10"}
        assert payload["context"]["skill_name"] == "test-skill"
        assert payload["context"]["skill_type"] == "workflow"
        assert payload["context"]["env"] == "test"

    def test_build_skill_content(self, sample_skill):
        content = _build_skill_content(sample_skill)
        assert content["skill_id"] == "test-skill"
        assert content["name"] == "test-skill"
        assert "name: test-skill" in content["frontmatter_yaml"]
        assert content["prose"] == sample_skill.prose

    def test_build_send_request(self, sample_message):
        req = _build_send_request("session-42", sample_message)
        assert req["session_id"] == "session-42"
        assert req["message"]["content"] == "Hello, run the workflow"
        assert req["message"]["message_type"] == "text"


# ---------------------------------------------------------------------------
# _proto_chunk_to_response tests
# ---------------------------------------------------------------------------


class TestProtoChunkMapping:
    def test_dict_chunk(self):
        chunk = _proto_chunk_to_response({
            "chunk_type": "text_delta",
            "content": "Hello",
        })
        assert chunk.chunk_type == "text_delta"
        assert chunk.content == "Hello"

    def test_object_chunk(self):
        proto = MagicMock()
        proto.chunk_type = "tool_start"
        proto.content = ""
        proto.tool_name = "bash"
        proto.tool_id = "t-1"
        proto.is_error = False

        chunk = _proto_chunk_to_response(proto)
        assert chunk.chunk_type == "tool_start"
        assert chunk.tool_name == "bash"

    def test_unknown_chunk_type_normalizes(self):
        chunk = _proto_chunk_to_response({
            "chunk_type": "unknown_type",
            "content": "data",
        })
        assert chunk.chunk_type == "text_delta"

    def test_done_chunk(self):
        chunk = _proto_chunk_to_response({
            "chunk_type": "done",
            "content": "",
        })
        assert chunk.chunk_type == "done"

    def test_error_chunk(self):
        chunk = _proto_chunk_to_response({
            "chunk_type": "error",
            "content": "something broke",
            "is_error": True,
        })
        assert chunk.chunk_type == "error"
        assert chunk.is_error is True


# ---------------------------------------------------------------------------
# RuntimeSandbox tests (mocked gRPC)
# ---------------------------------------------------------------------------


class TestRuntimeSandbox:
    def test_grpc_not_installed_raises(self):
        """RuntimeSandbox._ensure_grpc raises SandboxError if grpcio missing."""
        sandbox = RuntimeSandbox("grpc://localhost:50051")
        # Mock the import to simulate grpcio not being installed
        with patch.dict("sys.modules", {"grpc": None}):
            with pytest.raises(SandboxError, match="grpcio is required"):
                sandbox._ensure_grpc()

    @pytest.mark.asyncio
    async def test_initialize_with_mock(self, sample_skill, sample_config):
        """Initialize creates session via mocked stub."""
        sandbox = RuntimeSandbox("grpc://localhost:50051")

        mock_stub = MagicMock()
        mock_init_response = MagicMock()
        mock_init_response.session_id = "session-mock-1"
        mock_stub.Initialize.return_value = mock_init_response

        mock_load_response = MagicMock()
        mock_load_response.success = True
        mock_stub.LoadSkill.return_value = mock_load_response

        # Patch _get_stub to return our mock (bypasses grpc import)
        sandbox._get_stub = lambda: mock_stub

        session_id = await sandbox.initialize(sample_skill, sample_config)
        assert session_id == "session-mock-1"
        assert sandbox._skill_name == "test-skill"

    @pytest.mark.asyncio
    async def test_send_yields_chunks(self, sample_message):
        """send() yields ResponseChunk objects from mocked stream."""
        sandbox = RuntimeSandbox("grpc://localhost:50051")
        sandbox._session_id = "session-1"

        # Mock proto chunks
        chunk1 = MagicMock()
        chunk1.chunk_type = "text_delta"
        chunk1.content = "Hello"
        chunk1.tool_name = ""
        chunk1.tool_id = ""
        chunk1.is_error = False

        chunk2 = MagicMock()
        chunk2.chunk_type = "done"
        chunk2.content = ""
        chunk2.tool_name = ""
        chunk2.tool_id = ""
        chunk2.is_error = False

        mock_stub = MagicMock()
        mock_stub.Send.return_value = [chunk1, chunk2]
        sandbox._get_stub = lambda: mock_stub

        chunks = []
        async for c in sandbox.send(sample_message):
            chunks.append(c)

        assert len(chunks) == 2
        assert chunks[0].chunk_type == "text_delta"
        assert chunks[0].content == "Hello"
        assert chunks[1].chunk_type == "done"

    @pytest.mark.asyncio
    async def test_send_without_initialize_raises(self, sample_message):
        """send() raises if not initialized."""
        sandbox = RuntimeSandbox("grpc://localhost:50051")
        with pytest.raises(SandboxError, match="not initialized"):
            async for _ in sandbox.send(sample_message):
                pass

    @pytest.mark.asyncio
    async def test_terminate_builds_telemetry(self):
        """terminate() returns TelemetrySummary from collected chunks."""
        sandbox = RuntimeSandbox("grpc://localhost:50051")
        sandbox._session_id = "session-2"
        sandbox._skill_name = "test-skill"

        # Simulate collected chunks
        sandbox._chunks_collected = [
            ResponseChunk(chunk_type="text_delta", content="hi"),
            ResponseChunk(chunk_type="tool_start", content="", tool_name="bash"),
            ResponseChunk(chunk_type="tool_result", content="ok"),
            ResponseChunk(chunk_type="done", content=""),
        ]

        mock_stub = MagicMock()
        mock_term = MagicMock()
        mock_term.final_telemetry = None
        mock_stub.Terminate.return_value = mock_term
        sandbox._stub = mock_stub
        sandbox._channel = MagicMock()
        sandbox._get_stub = lambda: mock_stub

        summary = await sandbox.terminate()
        assert summary.session_id == "session-2"
        assert summary.completed_normally is True
        assert "bash" in summary.tools_called
        assert summary.total_turns == 1  # 1 text_delta
        assert summary.skill_loaded == "test-skill"

    @pytest.mark.asyncio
    async def test_connection_failure_raises(self, sample_skill, sample_config):
        """Initialize with a bad connection raises SandboxError."""
        sandbox = RuntimeSandbox("grpc://localhost:50051")

        mock_stub = MagicMock()
        mock_stub.Initialize.side_effect = Exception("Connection refused")
        sandbox._get_stub = lambda: mock_stub

        with pytest.raises(SandboxError, match="gRPC initialization failed"):
            await sandbox.initialize(sample_skill, sample_config)


# ---------------------------------------------------------------------------
# ConsistencyReport / MultiRuntimeSandbox tests
# ---------------------------------------------------------------------------


class TestConsistencyReport:
    def test_model_creation(self):
        report = ConsistencyReport(
            all_completed=True,
            tools_diff=[],
            hooks_diff=[],
            output_similarity=1.0,
        )
        assert report.all_completed is True
        assert report.output_similarity == 1.0

    def test_comparison_result_creation(self):
        result = ComparisonResult(
            results={"ep1": TelemetrySummary(session_id="s1", completed_normally=True)},
            consistency=ConsistencyReport(all_completed=True, output_similarity=1.0),
        )
        assert "ep1" in result.results
        assert result.consistency.all_completed is True


class TestMultiRuntimeSandbox:
    def test_requires_at_least_two_endpoints(self):
        with pytest.raises(SandboxError, match="at least 2 endpoints"):
            MultiRuntimeSandbox(["grpc://localhost:50051"])

    def test_compute_consistency_identical(self):
        """Identical summaries → all_completed=True, similarity=1.0."""
        summaries = {
            "ep1": TelemetrySummary(
                session_id="s1",
                completed_normally=True,
                tools_called=["bash", "file_read"],
                hooks_fired=[HookFiredEvent(event="PreToolUse", decision="allow")],
            ),
            "ep2": TelemetrySummary(
                session_id="s2",
                completed_normally=True,
                tools_called=["bash", "file_read"],
                hooks_fired=[HookFiredEvent(event="PreToolUse", decision="allow")],
            ),
        }
        report = MultiRuntimeSandbox._compute_consistency(summaries)
        assert report.all_completed is True
        assert report.tools_diff == []
        assert report.hooks_diff == []
        assert report.output_similarity == 1.0

    def test_compute_consistency_different_tools(self):
        """Different tool sets → tools_diff populated, similarity < 1.0."""
        summaries = {
            "ep1": TelemetrySummary(
                session_id="s1",
                completed_normally=True,
                tools_called=["bash", "file_read"],
            ),
            "ep2": TelemetrySummary(
                session_id="s2",
                completed_normally=True,
                tools_called=["bash", "file_write"],
            ),
        }
        report = MultiRuntimeSandbox._compute_consistency(summaries)
        assert report.all_completed is True
        assert set(report.tools_diff) == {"file_read", "file_write"}
        # Intersection = {bash}, Union = {bash, file_read, file_write} → 1/3
        assert abs(report.output_similarity - 1 / 3) < 0.01

    def test_compute_consistency_partial_failure(self):
        """One runtime not completed → all_completed=False."""
        summaries = {
            "ep1": TelemetrySummary(session_id="s1", completed_normally=True),
            "ep2": TelemetrySummary(session_id="s2", completed_normally=False),
        }
        report = MultiRuntimeSandbox._compute_consistency(summaries)
        assert report.all_completed is False

    def test_compute_consistency_empty_tools(self):
        """No tools called → similarity=1.0 (trivially consistent)."""
        summaries = {
            "ep1": TelemetrySummary(session_id="s1", completed_normally=True),
            "ep2": TelemetrySummary(session_id="s2", completed_normally=True),
        }
        report = MultiRuntimeSandbox._compute_consistency(summaries)
        assert report.output_similarity == 1.0
        assert report.tools_diff == []

    def test_compute_consistency_hook_diff(self):
        """Different hooks → hooks_diff populated."""
        summaries = {
            "ep1": TelemetrySummary(
                session_id="s1",
                completed_normally=True,
                hooks_fired=[
                    HookFiredEvent(event="PreToolUse", decision="allow"),
                    HookFiredEvent(event="Stop", decision="allow"),
                ],
            ),
            "ep2": TelemetrySummary(
                session_id="s2",
                completed_normally=True,
                hooks_fired=[
                    HookFiredEvent(event="PreToolUse", decision="allow"),
                ],
            ),
        }
        report = MultiRuntimeSandbox._compute_consistency(summaries)
        assert "Stop" in report.hooks_diff

    @pytest.mark.asyncio
    async def test_compare_with_mock_runtimes(self, sample_skill, sample_config, sample_message):
        """compare() with mocked _run_single returns ComparisonResult."""
        multi = MultiRuntimeSandbox([
            "grpc://localhost:50051",
            "grpc://localhost:50052",
        ])

        summary1 = TelemetrySummary(
            session_id="s1",
            completed_normally=True,
            tools_called=["bash"],
        )
        summary2 = TelemetrySummary(
            session_id="s2",
            completed_normally=True,
            tools_called=["bash"],
        )

        # Mock _run_single to return pre-built summaries
        async def mock_run(endpoint, config, skill, message):
            return summary1 if "50051" in endpoint else summary2

        multi._run_single = mock_run

        result = await multi.compare(sample_config, sample_skill, sample_message)
        assert len(result.results) == 2
        assert result.consistency.all_completed is True
        assert result.consistency.output_similarity == 1.0

    @pytest.mark.asyncio
    async def test_compare_handles_runtime_failure(self, sample_skill, sample_config, sample_message):
        """compare() handles one runtime failing gracefully."""
        multi = MultiRuntimeSandbox([
            "grpc://localhost:50051",
            "grpc://localhost:50052",
        ])

        summary1 = TelemetrySummary(session_id="s1", completed_normally=True)

        async def mock_run(endpoint, config, skill, message):
            if "50052" in endpoint:
                raise SandboxError("Connection refused")
            return summary1

        multi._run_single = mock_run

        result = await multi.compare(sample_config, sample_skill, sample_message)
        assert len(result.results) == 2
        # One failed → not all completed
        assert result.consistency.all_completed is False
