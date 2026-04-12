"""End-to-end API tests (in-process ASGI + respx for L2/L3 mocks)."""

from __future__ import annotations

import json

import httpx
import respx

L2_DEFAULT = "http://127.0.0.1:18085"
L3_DEFAULT = "http://127.0.0.1:18083"


async def test_health(app_client: httpx.AsyncClient) -> None:
    resp = await app_client.get("/health")
    assert resp.status_code == 200
    assert resp.json() == {"status": "ok"}


@respx.mock
async def test_create_session_happy_path(app_client: httpx.AsyncClient) -> None:
    respx.post(f"{L2_DEFAULT}/api/v1/memory/search").mock(
        return_value=httpx.Response(
            200,
            json={"hits": [{"memory_id": "m1", "memory_type": "anchor"}]},
        )
    )
    respx.post(url__regex=rf"{L3_DEFAULT}/v1/sessions/.*/validate").mock(
        return_value=httpx.Response(
            200,
            json={
                "session_id": "placeholder",
                "hooks_to_attach": [],
                "managed_settings_version": 1,
                "validated_at": "2026-04-12 02:00:00",
                "runtime_tier": "strict",
            },
        )
    )

    resp = await app_client.post(
        "/v1/sessions/create",
        json={
            "intent_text": "do the thing",
            "skill_id": "skill.test",
            "runtime_pref": "strict",
            "user_id": "u-1",
        },
    )
    assert resp.status_code == 200, resp.text
    body = resp.json()
    assert body["status"] == "active"  # Phase 0.5: L1 Initialize succeeds → active
    sid = body["session_id"]

    # GET /v1/sessions/{id} returns the persisted row.
    get_resp = await app_client.get(f"/v1/sessions/{sid}")
    assert get_resp.status_code == 200
    assert get_resp.json()["status"] == "active"


@respx.mock
async def test_create_session_l2_unavailable(app_client: httpx.AsyncClient) -> None:
    respx.post(f"{L2_DEFAULT}/api/v1/memory/search").mock(
        side_effect=httpx.ConnectError("no l2")
    )
    resp = await app_client.post(
        "/v1/sessions/create",
        json={
            "intent_text": "x",
            "skill_id": "skill.s",
            "runtime_pref": "strict",
        },
    )
    assert resp.status_code == 503
    detail = resp.json()["detail"]
    assert detail["code"] == "upstream_unavailable"
    assert detail["service"] == "l2"


@respx.mock
async def test_create_session_l3_no_policy(app_client: httpx.AsyncClient) -> None:
    respx.post(f"{L2_DEFAULT}/api/v1/memory/search").mock(
        return_value=httpx.Response(200, json={"hits": []})
    )
    respx.post(url__regex=rf"{L3_DEFAULT}/v1/sessions/.*/validate").mock(
        return_value=httpx.Response(
            404, json={"detail": {"code": "no_policy", "message": "empty"}}
        )
    )
    resp = await app_client.post(
        "/v1/sessions/create",
        json={
            "intent_text": "x",
            "skill_id": "skill.s",
            "runtime_pref": "strict",
        },
    )
    assert resp.status_code == 424
    assert resp.json()["detail"]["code"] == "no_policy"


@respx.mock
async def test_send_message_happy_path(app_client: httpx.AsyncClient) -> None:
    respx.post(f"{L2_DEFAULT}/api/v1/memory/search").mock(
        return_value=httpx.Response(200, json={"hits": []})
    )
    respx.post(url__regex=rf"{L3_DEFAULT}/v1/sessions/.*/validate").mock(
        return_value=httpx.Response(
            200,
            json={
                "session_id": "placeholder",
                "hooks_to_attach": [],
                "managed_settings_version": 1,
                "validated_at": "2026-04-12 02:00:00",
                "runtime_tier": "strict",
            },
        )
    )
    created = await app_client.post(
        "/v1/sessions/create",
        json={"intent_text": "x", "skill_id": "skill.s", "runtime_pref": "strict"},
    )
    sid = created.json()["session_id"]

    resp = await app_client.post(
        f"/v1/sessions/{sid}/message", json={"content": "hello"}
    )
    assert resp.status_code == 200, resp.text
    body = resp.json()
    assert body["session_id"] == sid
    assert "response_text" in body  # Phase 0.5: real L1 Send returns text
    assert len(body["events"]) > 0


async def test_send_message_unknown_session_404(app_client: httpx.AsyncClient) -> None:
    resp = await app_client.post(
        "/v1/sessions/sess_ghost/message", json={"content": "hi"}
    )
    assert resp.status_code == 404
    assert resp.json()["detail"]["code"] == "session_not_found"


@respx.mock
async def test_list_events_range(app_client: httpx.AsyncClient) -> None:
    respx.post(f"{L2_DEFAULT}/api/v1/memory/search").mock(
        return_value=httpx.Response(200, json={"hits": []})
    )
    respx.post(url__regex=rf"{L3_DEFAULT}/v1/sessions/.*/validate").mock(
        return_value=httpx.Response(
            200,
            json={
                "session_id": "placeholder",
                "hooks_to_attach": [],
                "managed_settings_version": 1,
                "validated_at": "2026-04-12 02:00:00",
                "runtime_tier": "strict",
            },
        )
    )
    created = await app_client.post(
        "/v1/sessions/create",
        json={"intent_text": "x", "skill_id": "skill.s", "runtime_pref": "strict"},
    )
    sid = created.json()["session_id"]
    await app_client.post(f"/v1/sessions/{sid}/message", json={"content": "hi-1"})
    await app_client.post(f"/v1/sessions/{sid}/message", json={"content": "hi-2"})

    resp = await app_client.get(f"/v1/sessions/{sid}/events")
    assert resp.status_code == 200
    events = resp.json()["events"]
    seqs = [e["seq"] for e in events]
    assert seqs == sorted(seqs)
    assert len(events) >= 4  # SESSION_CREATED + RUNTIME_INITIALIZE_STUBBED + 2x(USER+STUB)


async def test_list_events_limit_over_cap_422(app_client: httpx.AsyncClient) -> None:
    resp = await app_client.get("/v1/sessions/sess_x/events?limit=501")
    assert resp.status_code == 422


async def test_get_session_unknown_404(app_client: httpx.AsyncClient) -> None:
    resp = await app_client.get("/v1/sessions/sess_ghost")
    assert resp.status_code == 404


async def test_create_session_missing_field_422(app_client: httpx.AsyncClient) -> None:
    resp = await app_client.post(
        "/v1/sessions/create",
        json={"intent_text": "x", "runtime_pref": "strict"},  # missing skill_id
    )
    assert resp.status_code == 422


# ─── SSE streaming tests ────────────────────────────────────────────────────


@respx.mock
async def test_send_message_stream_sse(app_client: httpx.AsyncClient) -> None:
    """POST /message/stream should return text/event-stream with SSE events."""
    respx.post(f"{L2_DEFAULT}/api/v1/memory/search").mock(
        return_value=httpx.Response(200, json={"hits": []})
    )
    respx.post(url__regex=rf"{L3_DEFAULT}/v1/sessions/.*/validate").mock(
        return_value=httpx.Response(
            200,
            json={
                "session_id": "placeholder",
                "hooks_to_attach": [],
                "managed_settings_version": 1,
                "validated_at": "2026-04-12 02:00:00",
                "runtime_tier": "strict",
            },
        )
    )
    created = await app_client.post(
        "/v1/sessions/create",
        json={"intent_text": "x", "skill_id": "skill.s", "runtime_pref": "strict"},
    )
    sid = created.json()["session_id"]

    resp = await app_client.post(
        f"/v1/sessions/{sid}/message/stream", json={"content": "hello sse"}
    )
    assert resp.status_code == 200
    assert "text/event-stream" in resp.headers.get("content-type", "")

    # Parse SSE events from the response body.
    lines = resp.text.strip().split("\n")
    sse_events: list[dict] = []
    current_event = "chunk"
    for line in lines:
        if line.startswith("event: "):
            current_event = line[7:]
        elif line.startswith("data: "):
            data = json.loads(line[6:])
            sse_events.append({"event": current_event, "data": data})
            current_event = "chunk"

    # Should have chunk events and a done event.
    chunk_events = [e for e in sse_events if e["event"] == "chunk"]
    done_events = [e for e in sse_events if e["event"] == "done"]
    assert len(chunk_events) >= 1
    assert len(done_events) == 1
    assert done_events[0]["data"]["session_id"] == sid


async def test_send_message_stream_unknown_session_404(
    app_client: httpx.AsyncClient,
) -> None:
    resp = await app_client.post(
        "/v1/sessions/sess_ghost/message/stream", json={"content": "hi"}
    )
    assert resp.status_code == 404
    assert resp.json()["detail"]["code"] == "session_not_found"
