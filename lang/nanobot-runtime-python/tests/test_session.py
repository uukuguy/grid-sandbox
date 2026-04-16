"""W2.T3 session tests — multi-turn agent loop with event emission."""
from __future__ import annotations

import json
import stat
from pathlib import Path
from unittest.mock import AsyncMock, MagicMock

import pytest

from nanobot_runtime.provider import OpenAICompatProvider
from nanobot_runtime.session import AgentEvent, AgentSession, EventType


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _make_text_response(content: str) -> dict:
    return {"choices": [{"message": {"role": "assistant", "content": content}}]}


def _make_tc(id: str, name: str, args: dict) -> dict:
    return {
        "id": id,
        "type": "function",
        "function": {"name": name, "arguments": json.dumps(args)},
    }


def _make_tool_call_response(tool_calls: list[dict]) -> dict:
    return {
        "choices": [
            {
                "message": {
                    "role": "assistant",
                    "content": None,
                    "tool_calls": tool_calls,
                }
            }
        ]
    }


@pytest.fixture
def mock_provider():
    p = MagicMock(spec=OpenAICompatProvider)
    p.chat = AsyncMock()
    return p


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------

async def test_pure_text_response_emits_chunk_and_stop(mock_provider):
    mock_provider.chat.return_value = _make_text_response("Hello, world!")

    session = AgentSession(provider=mock_provider)
    events = [ev async for ev in session.run("hi")]

    types = [e.event_type for e in events]
    assert types == [EventType.CHUNK, EventType.STOP]
    assert events[0].content == "Hello, world!"
    assert events[1].content == "Hello, world!"


async def test_single_tool_call_emits_expected_sequence(mock_provider):
    tc = _make_tc("tc-1", "get_weather", {"city": "Tokyo"})
    mock_provider.chat.side_effect = [
        _make_tool_call_response([tc]),
        _make_text_response("It's sunny in Tokyo."),
    ]

    session = AgentSession(provider=mock_provider)
    events = [ev async for ev in session.run("What's the weather?")]

    types = [e.event_type for e in events]
    assert types == [EventType.TOOL_CALL, EventType.TOOL_RESULT, EventType.CHUNK, EventType.STOP]

    tool_call_ev = events[0]
    assert tool_call_ev.tool_name == "get_weather"
    assert tool_call_ev.tool_call_id == "tc-1"

    tool_result_ev = events[1]
    assert tool_result_ev.tool_call_id == "tc-1"
    assert tool_result_ev.tool_name == "get_weather"

    # Second chat() call must include the tool result message
    second_call_messages = mock_provider.chat.call_args_list[1][1]["messages"]
    roles = [m["role"] for m in second_call_messages]
    assert "tool" in roles


async def test_multi_turn_two_tool_call_rounds(mock_provider):
    tc1 = _make_tc("tc-a", "search", {"q": "rust"})
    tc2 = _make_tc("tc-b", "summarize", {"text": "..."})
    mock_provider.chat.side_effect = [
        _make_tool_call_response([tc1]),
        _make_tool_call_response([tc2]),
        _make_text_response("Done."),
    ]

    session = AgentSession(provider=mock_provider)
    events = [ev async for ev in session.run("Do research")]

    types = [e.event_type for e in events]
    assert types == [
        EventType.TOOL_CALL, EventType.TOOL_RESULT,
        EventType.TOOL_CALL, EventType.TOOL_RESULT,
        EventType.CHUNK, EventType.STOP,
    ]
    assert mock_provider.chat.call_count == 3


async def test_post_tool_use_hook_fires(mock_provider, tmp_path):
    hook_script = tmp_path / "hook.sh"
    hook_script.write_text("#!/bin/sh\nexit 0\n")
    hook_script.chmod(hook_script.stat().st_mode | stat.S_IEXEC)

    tc = _make_tc("tc-h", "echo_tool", {"msg": "hi"})
    mock_provider.chat.side_effect = [
        _make_tool_call_response([tc]),
        _make_text_response("ok"),
    ]

    session = AgentSession(
        provider=mock_provider,
        post_tool_use_hooks=[str(hook_script)],
    )
    events = [ev async for ev in session.run("test hooks")]

    hook_events = [e for e in events if e.event_type == EventType.HOOK_FIRED]
    assert len(hook_events) == 1
    assert hook_events[0].hook_event == "PostToolUse"
    assert hook_events[0].hook_decision == "allow"


async def test_provider_error_emits_error_event(mock_provider):
    mock_provider.chat.side_effect = Exception("timeout")

    session = AgentSession(provider=mock_provider)
    events = [ev async for ev in session.run("this will fail")]

    assert len(events) == 1
    assert events[0].event_type == EventType.ERROR
    assert events[0].is_error is True
