"""Tests for L2 Memory Engine HTTP client."""
from __future__ import annotations

import pytest
import httpx

from claude_code_runtime.l2_memory_client import L2MemoryClient


# ── Construction ──


def test_default_base_url(monkeypatch):
    monkeypatch.delenv("EAASP_L2_PORT", raising=False)
    monkeypatch.delenv("EAASP_L2_HOST", raising=False)
    client = L2MemoryClient()
    assert client.base_url == "http://127.0.0.1:18085"


def test_custom_base_url():
    client = L2MemoryClient("http://mem.local:9999/")
    assert client.base_url == "http://mem.local:9999"


def test_env_port_override(monkeypatch):
    monkeypatch.setenv("EAASP_L2_PORT", "28085")
    monkeypatch.setenv("EAASP_L2_HOST", "10.0.0.5")
    client = L2MemoryClient()
    assert client.base_url == "http://10.0.0.5:28085"


# ── Mock transport for HTTP assertions ──


class _RecordingTransport(httpx.AsyncBaseTransport):
    """Records requests and returns canned responses."""

    def __init__(self, response_json: dict | None = None, status_code: int = 200):
        self.requests: list[httpx.Request] = []
        self._response_json = response_json or {}
        self._status_code = status_code

    async def handle_async_request(self, request: httpx.Request) -> httpx.Response:
        self.requests.append(request)
        import json
        body = json.dumps(self._response_json).encode()
        return httpx.Response(
            status_code=self._status_code,
            content=body,
            headers={"content-type": "application/json"},
        )


def _make_client(transport: _RecordingTransport) -> L2MemoryClient:
    http = httpx.AsyncClient(
        base_url="http://test:18085",
        transport=transport,
        trust_env=False,
    )
    return L2MemoryClient("http://test:18085", client=http)


# ── write_anchor ──


@pytest.mark.asyncio
async def test_write_anchor_request_body():
    transport = _RecordingTransport(response_json={"anchor_id": 1})
    client = _make_client(transport)

    result = await client.write_anchor(
        event_id="evt-1",
        session_id="sess-1",
        anchor_type="tool_execution",
        data_ref="some output",
        source_system="test-runtime",
    )
    assert result == {"anchor_id": 1}

    import json
    req = transport.requests[0]
    body = json.loads(req.content)
    assert body["args"]["event_id"] == "evt-1"
    assert body["args"]["session_id"] == "sess-1"
    assert body["args"]["type"] == "tool_execution"
    assert body["args"]["data_ref"] == "some output"
    assert body["args"]["source_system"] == "test-runtime"
    assert str(req.url).endswith("/tools/memory_write_anchor/invoke")


@pytest.mark.asyncio
async def test_write_anchor_optional_fields_omitted():
    transport = _RecordingTransport(response_json={"anchor_id": 2})
    client = _make_client(transport)

    await client.write_anchor(
        event_id="evt-2",
        session_id="sess-2",
        anchor_type="observation",
    )

    import json
    body = json.loads(transport.requests[0].content)
    assert "data_ref" not in body["args"]
    assert "snapshot_hash" not in body["args"]


@pytest.mark.asyncio
async def test_write_anchor_http_error():
    transport = _RecordingTransport(
        response_json={"detail": "bad request"},
        status_code=400,
    )
    client = _make_client(transport)

    with pytest.raises(httpx.HTTPStatusError):
        await client.write_anchor(
            event_id="e", session_id="s", anchor_type="t"
        )


# ── write_file ──


@pytest.mark.asyncio
async def test_write_file_request_body():
    transport = _RecordingTransport(
        response_json={"memory_id": "mem-1", "version": 1}
    )
    client = _make_client(transport)

    result = await client.write_file(
        scope="session:s1",
        category="tool_evidence",
        content="tool output",
        evidence_refs=["anchor-1"],
    )
    assert result["memory_id"] == "mem-1"
    assert result["version"] == 1

    import json
    body = json.loads(transport.requests[0].content)
    assert body["args"]["scope"] == "session:s1"
    assert body["args"]["category"] == "tool_evidence"
    assert body["args"]["content"] == "tool output"
    assert body["args"]["evidence_refs"] == ["anchor-1"]
    assert body["args"]["status"] == "agent_suggested"
    assert str(transport.requests[0].url).endswith("/tools/memory_write_file/invoke")


@pytest.mark.asyncio
async def test_write_file_optional_fields_omitted():
    transport = _RecordingTransport(
        response_json={"memory_id": "mem-2", "version": 1}
    )
    client = _make_client(transport)

    await client.write_file(scope="s", category="c", content="x")

    import json
    body = json.loads(transport.requests[0].content)
    assert "memory_id" not in body["args"]
    assert "evidence_refs" not in body["args"]


# ── health ──


@pytest.mark.asyncio
async def test_health_ok():
    transport = _RecordingTransport(response_json={"status": "ok"})
    client = _make_client(transport)
    assert await client.health() is True


@pytest.mark.asyncio
async def test_health_down():
    transport = _RecordingTransport(status_code=503)
    client = _make_client(transport)
    assert await client.health() is False
