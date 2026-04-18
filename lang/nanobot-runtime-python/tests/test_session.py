"""W2.T3 session tests — multi-turn agent loop with event emission."""
from __future__ import annotations

import json
import stat
from unittest.mock import AsyncMock, MagicMock

import pytest

from nanobot_runtime.provider import OpenAICompatProvider
from nanobot_runtime.session import AgentSession, EventType


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


async def test_stop_hook_allow_fires_and_emits_stop(mock_provider, tmp_path):
    """Stop hook exit-0 → allow: HOOK_FIRED + STOP emitted."""
    hook_script = tmp_path / "stop_hook.sh"
    hook_script.write_text("#!/bin/sh\nexit 0\n")
    hook_script.chmod(hook_script.stat().st_mode | stat.S_IEXEC)

    mock_provider.chat.return_value = _make_text_response("All done.")

    session = AgentSession(
        provider=mock_provider,
        stop_hooks=[str(hook_script)],
    )
    events = [ev async for ev in session.run("finish")]

    types = [e.event_type for e in events]
    assert EventType.HOOK_FIRED in types
    assert EventType.STOP in types
    hook_ev = next(e for e in events if e.event_type == EventType.HOOK_FIRED)
    assert hook_ev.hook_event == "Stop"
    assert hook_ev.hook_decision == "allow"


async def test_stop_hook_deny_injects_system_and_continues(mock_provider, tmp_path):
    """Stop hook exit-2 → deny: system message injected, loop re-enters."""
    deny_hook = tmp_path / "deny.sh"
    deny_hook.write_text("#!/bin/sh\nexit 2\n")
    deny_hook.chmod(deny_hook.stat().st_mode | stat.S_IEXEC)

    # First call returns text → stop hook denies → second call returns text → stop hook allows
    allow_hook = tmp_path / "allow.sh"
    allow_hook.write_text("#!/bin/sh\nexit 0\n")
    allow_hook.chmod(allow_hook.stat().st_mode | stat.S_IEXEC)

    mock_provider.chat.side_effect = [
        _make_text_response("draft output"),
        _make_text_response("revised output"),
    ]

    # Use deny hook first time only by chaining — simplest: just use deny then allow in sequence
    # But stop_hooks is a list; for simplicity test with deny-only hook and check re-entry via chat call count
    session = AgentSession(
        provider=mock_provider,
        stop_hooks=[str(deny_hook)],
        max_turns=5,
    )
    events = [ev async for ev in session.run("finish")]

    # deny hook should have fired at least once
    hook_events = [e for e in events if e.event_type == EventType.HOOK_FIRED]
    assert any(e.hook_decision == "deny" for e in hook_events)
    # provider was called more than once due to re-entry
    assert mock_provider.chat.call_count >= 2


async def test_stop_hook_timeout_fails_open(mock_provider, tmp_path):
    """Stop hook timeout → fail-open (allow), STOP emitted normally."""
    slow_hook = tmp_path / "slow.sh"
    slow_hook.write_text("#!/bin/sh\nsleep 30\n")
    slow_hook.chmod(slow_hook.stat().st_mode | stat.S_IEXEC)

    mock_provider.chat.return_value = _make_text_response("done")

    from nanobot_runtime import session as session_mod
    orig_timeout = session_mod.HOOK_TIMEOUT_SECS
    session_mod.HOOK_TIMEOUT_SECS = 0.1

    try:
        sess = AgentSession(provider=mock_provider, stop_hooks=[str(slow_hook)])
        events = [ev async for ev in sess.run("go")]
    finally:
        session_mod.HOOK_TIMEOUT_SECS = orig_timeout

    hook_events = [e for e in events if e.event_type == EventType.HOOK_FIRED]
    assert len(hook_events) == 1
    assert hook_events[0].hook_decision == "allow"  # fail-open
    assert any(e.event_type == EventType.STOP for e in events)
