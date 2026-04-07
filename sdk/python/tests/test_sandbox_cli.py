"""Tests for sandbox core models and GridCliSandbox."""

from __future__ import annotations

import asyncio
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from eaasp.models.message import ResponseChunk, UserMessage
from eaasp.models.session import SessionConfig
from eaasp.models.skill import Skill, SkillFrontmatter
from eaasp.sandbox.base import HookFiredEvent, SandboxError, TelemetrySummary
from eaasp.sandbox.grid_cli import GridCliSandbox


# ── Model tests ──────────────────────────────────────────────


def _make_skill() -> Skill:
    return Skill(
        frontmatter=SkillFrontmatter(
            name="test-skill",
            version="1.0.0",
            description="A test skill for sandbox testing",
            author="test",
        ),
        prose="You are a helpful assistant for testing purposes. Follow instructions carefully.",
    )


class TestTelemetrySummary:
    def test_create_default(self):
        summary = TelemetrySummary()
        assert summary.session_id == ""
        assert summary.total_turns == 0
        assert summary.tools_called == []
        assert summary.hooks_fired == []
        assert summary.completed_normally is False

    def test_create_with_values(self):
        summary = TelemetrySummary(
            session_id="sess-123",
            total_turns=5,
            tools_called=["bash", "file_read"],
            hooks_fired=[
                HookFiredEvent(
                    event="PreToolUse",
                    hook_source="check.py",
                    decision="allow",
                    tool_name="bash",
                    latency_ms=12.5,
                )
            ],
            input_tokens=100,
            output_tokens=200,
            duration_ms=1500.0,
            skill_loaded="test-skill",
            completed_normally=True,
        )
        assert summary.session_id == "sess-123"
        assert summary.total_turns == 5
        assert len(summary.tools_called) == 2
        assert len(summary.hooks_fired) == 1
        assert summary.hooks_fired[0].event == "PreToolUse"
        assert summary.completed_normally is True


class TestHookFiredEvent:
    def test_create(self):
        event = HookFiredEvent(
            event="PostToolUse",
            hook_source="format_check.py",
            decision="block",
            tool_name="file_write",
            latency_ms=8.3,
        )
        assert event.event == "PostToolUse"
        assert event.decision == "block"
        assert event.tool_name == "file_write"

    def test_defaults(self):
        event = HookFiredEvent(event="Stop")
        assert event.hook_source == ""
        assert event.decision == ""
        assert event.tool_name is None
        assert event.latency_ms == 0.0


# ── GridCliSandbox tests ────────────────────────────────────


class TestGridCliSandboxBinaryCheck:
    def test_binary_not_found_raises(self):
        sandbox = GridCliSandbox(grid_bin="nonexistent-grid-binary-xyz")
        with pytest.raises(SandboxError, match="not found in PATH"):
            sandbox._check_binary()

    @patch("shutil.which", return_value="/usr/local/bin/grid")
    def test_binary_found(self, mock_which):
        sandbox = GridCliSandbox(grid_bin="grid")
        result = sandbox._check_binary()
        assert result == "/usr/local/bin/grid"
        mock_which.assert_called_once_with("grid")


class TestGridCliSandboxInitialize:
    @pytest.mark.asyncio
    @patch("shutil.which", return_value="/usr/local/bin/grid")
    @patch("asyncio.create_subprocess_exec")
    async def test_initialize_creates_session(self, mock_exec, mock_which):
        mock_proc = MagicMock()
        mock_proc.stdin = MagicMock()
        mock_proc.stdout = MagicMock()
        mock_proc.stderr = MagicMock()
        mock_exec.return_value = mock_proc

        sandbox = GridCliSandbox()
        skill = _make_skill()
        session_id = await sandbox.initialize(skill)

        assert session_id.startswith("sandbox-")
        assert sandbox._skill_name == "test-skill"
        mock_exec.assert_called_once()

        # Cleanup
        if sandbox._tmpdir:
            sandbox._tmpdir.cleanup()


class TestGridCliSandboxParseOutput:
    def test_parse_json_line(self):
        sandbox = GridCliSandbox()
        chunk = sandbox._parse_output_line(
            '{"type": "text_delta", "content": "Hello"}'
        )
        assert chunk is not None
        assert chunk.chunk_type == "text_delta"
        assert chunk.content == "Hello"

    def test_parse_tool_start(self):
        sandbox = GridCliSandbox()
        chunk = sandbox._parse_output_line(
            '{"type": "tool_start", "tool_name": "bash", "tool_id": "t1"}'
        )
        assert chunk is not None
        assert chunk.chunk_type == "tool_start"
        assert chunk.tool_name == "bash"

    def test_parse_non_json_line(self):
        sandbox = GridCliSandbox()
        chunk = sandbox._parse_output_line("plain text output")
        assert chunk is not None
        assert chunk.chunk_type == "text_delta"
        assert chunk.content == "plain text output"

    def test_parse_done(self):
        sandbox = GridCliSandbox()
        chunk = sandbox._parse_output_line('{"type": "done"}')
        assert chunk is not None
        assert chunk.chunk_type == "done"


class TestGridCliSandboxTelemetry:
    def test_build_telemetry_empty(self):
        sandbox = GridCliSandbox()
        summary = sandbox._build_telemetry()
        assert summary.completed_normally is False
        assert summary.tools_called == []

    def test_build_telemetry_with_chunks(self):
        sandbox = GridCliSandbox()
        sandbox._session_id = "sess-test"
        sandbox._skill_name = "my-skill"
        sandbox._chunks_collected = [
            ResponseChunk(chunk_type="text_delta", content="hi"),
            ResponseChunk(chunk_type="tool_start", tool_name="bash"),
            ResponseChunk(chunk_type="tool_result", tool_name="bash", content="ok"),
            ResponseChunk(chunk_type="text_delta", content="done"),
            ResponseChunk(chunk_type="done"),
        ]
        summary = sandbox._build_telemetry()
        assert summary.session_id == "sess-test"
        assert summary.skill_loaded == "my-skill"
        assert summary.tools_called == ["bash"]
        assert summary.completed_normally is True
        assert summary.total_turns == 2  # two text_delta chunks
