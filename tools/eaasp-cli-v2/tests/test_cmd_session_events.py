"""Tests for `eaasp session events` command."""

from __future__ import annotations

import json
from unittest.mock import patch

from typer.testing import CliRunner

from eaasp_cli_v2.main import app

runner = CliRunner()

_MOCK_EVENTS = {
    "session_id": "sess_001",
    "events": [
        {
            "seq": 1,
            "event_type": "SESSION_START",
            "payload": {"runtime_id": "grid-runtime"},
            "created_at": 1700000000,
            "cluster_id": None,
        },
        {
            "seq": 2,
            "event_type": "PRE_TOOL_USE",
            "payload": {"tool_name": "scada_read"},
            "created_at": 1700000001,
            "cluster_id": "c-abc12345",
        },
        {
            "seq": 3,
            "event_type": "STOP",
            "payload": {"reason": "complete"},
            "created_at": 1700000005,
            "cluster_id": "c-abc12345",
        },
    ],
}


async def _mock_fetch_events(*args, **kwargs):
    """Async mock that returns events directly."""
    return _MOCK_EVENTS


async def _mock_fetch_empty(*args, **kwargs):
    return {"session_id": "sess_empty", "events": []}


def test_session_events_table_format():
    """session events <id> should list events in table format."""
    with patch(
        "eaasp_cli_v2.cmd_session._fetch_events",
        new=_mock_fetch_events,
    ):
        result = runner.invoke(app, ["session", "events", "sess_001"])
        assert result.exit_code == 0, result.output
        assert "SESSION_START" in result.output
        assert "PRE_TOOL_USE" in result.output
        assert "STOP" in result.output


def test_session_events_json_format():
    """session events <id> --format json should output raw JSON."""
    with patch(
        "eaasp_cli_v2.cmd_session._fetch_events",
        new=_mock_fetch_events,
    ):
        result = runner.invoke(
            app, ["session", "events", "sess_001", "--format", "json"]
        )
        assert result.exit_code == 0, result.output
        parsed = json.loads(result.output)
        assert len(parsed["events"]) == 3
        assert parsed["events"][0]["event_type"] == "SESSION_START"


def test_session_events_empty():
    """session events for a session with no events."""
    with patch(
        "eaasp_cli_v2.cmd_session._fetch_events",
        new=_mock_fetch_empty,
    ):
        result = runner.invoke(app, ["session", "events", "sess_empty"])
        assert result.exit_code == 0, result.output
        assert "No events" in result.output


# ── S4.T2 (D84) — follow mode ────────────────────────────────────────────


def test_session_events_follow_hits_stream_endpoint(monkeypatch):
    """--follow hits GET /events/stream, renders events, propagates --from."""
    import httpx
    from eaasp_cli_v2 import main as cli_main
    from eaasp_cli_v2.client import ServiceClient

    sse_body = (
        "event: event\n"
        'data: {"seq": 1, "event_type": "SESSION_START", '
        '"payload": {"runtime_id": "grid-runtime"}, "created_at": 1700000000}\n\n'
        "event: event\n"
        'data: {"seq": 2, "event_type": "STOP", '
        '"payload": {"reason": "complete"}, "created_at": 1700000001}\n\n'
    )

    captured: dict = {}

    def handler(req: httpx.Request) -> httpx.Response:
        captured["method"] = req.method
        captured["url"] = str(req.url)
        return httpx.Response(
            200,
            content=sse_body.encode("utf-8"),
            headers={"content-type": "text/event-stream"},
        )

    transport = httpx.MockTransport(handler)
    mock_client = httpx.AsyncClient(transport=transport)
    monkeypatch.setattr(
        cli_main,
        "_client_factory",
        lambda cfg: ServiceClient.from_httpx(mock_client),
    )

    result = runner.invoke(app, ["session", "events", "sess_follow", "--follow"])
    assert result.exit_code == 0, result.output
    # Lock the real HTTP contract: GET + correct path + default from=1.
    # (The original test didn't capture method → FastAPI would have returned
    # 405 against a GET route if stream_sse kept its POST default.)
    assert captured["method"] == "GET"
    assert "/v1/sessions/sess_follow/events/stream" in captured["url"]
    assert "from=1" in captured["url"]
    assert "SESSION_START" in result.output
    assert "STOP" in result.output


def test_session_events_follow_respects_from(monkeypatch):
    """--follow --from N propagates ?from=N and renders the seeded event.

    Also locks method=GET (C1 regression guard) and round-trips payload through
    ``_format_event_line`` (M3 — previous version used an empty body which
    bypassed the render path entirely).
    """
    import httpx
    from eaasp_cli_v2 import main as cli_main
    from eaasp_cli_v2.client import ServiceClient

    sse_body = (
        "event: event\n"
        'data: {"seq": 42, "event_type": "PRE_TOOL_USE", '
        '"payload": {"tool_name": "scada_read"}, "created_at": 1700000100}\n\n'
    )
    captured: dict = {}

    def handler(req: httpx.Request) -> httpx.Response:
        captured["method"] = req.method
        captured["url"] = str(req.url)
        return httpx.Response(
            200,
            content=sse_body.encode("utf-8"),
            headers={"content-type": "text/event-stream"},
        )

    transport = httpx.MockTransport(handler)
    mock_client = httpx.AsyncClient(transport=transport)
    monkeypatch.setattr(
        cli_main,
        "_client_factory",
        lambda cfg: ServiceClient.from_httpx(mock_client),
    )

    result = runner.invoke(
        app, ["session", "events", "sess_from", "--follow", "--from", "42"]
    )
    assert result.exit_code == 0, result.output
    assert captured["method"] == "GET"
    assert "from=42" in captured["url"]
    assert "PRE_TOOL_USE" in result.output
    # Payload round-trip proof — _format_event_line surfaces tool_name.
    assert "scada_read" in result.output


def test_session_events_follow_surfaces_stream_error(monkeypatch):
    """Server-side SSE ``event: error`` frame is surfaced and exits cleanly.

    Exit code is 0 by design: the CLI successfully consumed the stream to EOF;
    the logical error is a server-reported condition, not a client failure.
    (If a future op wants non-zero exit on logical errors, that's a separate
    UX decision — document it in the plan before changing behavior.)
    """
    import httpx
    from eaasp_cli_v2 import main as cli_main
    from eaasp_cli_v2.client import ServiceClient

    sse_body = (
        "event: error\n"
        'data: {"code": "session_not_found", "session_id": "sess_gone"}\n\n'
    )
    captured: dict = {}

    def handler(req: httpx.Request) -> httpx.Response:
        captured["method"] = req.method
        return httpx.Response(
            200,
            content=sse_body.encode("utf-8"),
            headers={"content-type": "text/event-stream"},
        )

    transport = httpx.MockTransport(handler)
    mock_client = httpx.AsyncClient(transport=transport)
    monkeypatch.setattr(
        cli_main,
        "_client_factory",
        lambda cfg: ServiceClient.from_httpx(mock_client),
    )

    # Force stderr to a separate stream so we can assert on it directly
    # instead of a merged buffer (M2: previous OR-assertion would have passed
    # even if the server's `code` field never reached the CLI).
    strict_runner = CliRunner()
    result = strict_runner.invoke(
        app, ["session", "events", "sess_gone", "--follow"]
    )
    assert result.exit_code == 0, result.output
    assert captured["method"] == "GET"
    # The server-reported code must round-trip into the CLI rendering.
    combined = result.output + (result.stderr or "")
    assert "session_not_found" in combined
    assert "stream error" in combined
