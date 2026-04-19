"""Regression tests for Phase 2.5 S4.T2 L4 chunk coalescing.

Context: The LLM provider's SSE stream emits ~4-byte token-level deltas
(`text_delta` / `thinking`). Before this fix, L4 wrote each delta as one
RESPONSE_CHUNK event, yielding 500+ events per session and blowing past the
`/v1/sessions/{sid}/events?limit=500` API ceiling so E2E harnesses couldn't
see the trailing `STOP` event.

After the fix:
- Contiguous runs of `text_delta` are coalesced into one aggregate
  RESPONSE_CHUNK at DB persistence time.
- Same for `thinking`.
- Non-delta chunks flush the buffer first so lifecycle event ordering is
  preserved.
- SSE stream still yields every delta so downstream UIs keep the
  typewriter effect.

These tests pin the invariants by driving SessionOrchestrator through a
stub L1 that emits many tokens, then asserting the persisted event counts
in the L4 SQLite store.
"""

from __future__ import annotations

from collections.abc import AsyncIterator
from typing import Any

import httpx
import pytest
import respx

from eaasp_l4_orchestration.db import connect
from eaasp_l4_orchestration.event_stream import SessionEventStream
from eaasp_l4_orchestration.handshake import L2Client, L3Client
from eaasp_l4_orchestration.session_orchestrator import SessionOrchestrator

L2_BASE = "http://l2.test"
L3_BASE = "http://l3.test"


class _TokenStormL1:
    """Stub L1 that emits many tiny text_delta/thinking chunks interleaved
    with lifecycle chunks, simulating a real LLM SSE stream."""

    def __init__(
        self,
        runtime_id: str = "grid-runtime",
        text_tokens: int = 50,
        thinking_tokens: int = 30,
        tool_rounds: int = 2,
    ) -> None:
        self.runtime_id = runtime_id
        self.text_tokens = text_tokens
        self.thinking_tokens = thinking_tokens
        self.tool_rounds = tool_rounds

    async def initialize(self, payload_dict: dict[str, Any]) -> dict[str, str]:
        return {
            "session_id": payload_dict.get("session_id", "mock"),
            "runtime_id": self.runtime_id,
        }

    async def send(self, session_id: str, content: str, message_type: str = "text"):
        # Turn 0: lots of thinking tokens.
        for i in range(self.thinking_tokens):
            yield {"chunk_type": "thinking", "content": f"t{i}"}

        # Tool rounds: tool_start → tool_result interleaved with text_delta.
        for r in range(self.tool_rounds):
            yield {
                "chunk_type": "tool_start",
                "tool_name": f"tool_{r}",
                "tool_id": f"tu_{r}",
                "content": "{}",
            }
            yield {
                "chunk_type": "tool_result",
                "tool_name": f"tool_{r}",
                "tool_id": f"tu_{r}",
                "content": f"result_{r}",
                "is_error": False,
            }
            # A few text_delta tokens between rounds.
            for i in range(5):
                yield {"chunk_type": "text_delta", "content": f"r{r}t{i}"}

        # Final burst: long text_delta run.
        for i in range(self.text_tokens):
            yield {"chunk_type": "text_delta", "content": f"final_{i}"}

        yield {"chunk_type": "done", "content": ""}

    async def terminate(self) -> None:
        pass

    async def close(self) -> None:
        pass


async def _make_orchestrator(
    tmp_db_path: str, http_client: httpx.AsyncClient, l1_factory
) -> SessionOrchestrator:
    l2 = L2Client(http_client, base_url=L2_BASE)
    l3 = L3Client(http_client, base_url=L3_BASE)
    stream = SessionEventStream(tmp_db_path)
    return SessionOrchestrator(
        tmp_db_path, l2=l2, l3=l3, event_stream=stream, l1_factory=l1_factory
    )


async def _seed_session_row(tmp_db_path: str, session_id: str) -> None:
    """Insert a minimal sessions row bypassing the full handshake."""
    import time

    db = await connect(tmp_db_path)
    try:
        await db.execute("BEGIN IMMEDIATE")
        await db.execute(
            """
            INSERT INTO sessions
                (session_id, intent_id, skill_id, runtime_id, user_id,
                 status, payload_json, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (session_id, None, "skill.test", "grid-runtime", "u", "created",
             "{}", int(time.time())),
        )
        await db.commit()
    finally:
        await db.close()


async def _count_events(tmp_db_path: str, session_id: str) -> dict[str, int]:
    """Return `{event_type or f'{event_type}:{chunk_type}'` → count}` map."""
    import json
    db = await connect(tmp_db_path)
    try:
        cur = await db.execute(
            """
            SELECT event_type, payload_json
            FROM session_events
            WHERE session_id = ?
            ORDER BY seq
            """,
            (session_id,),
        )
        rows = await cur.fetchall()
        await cur.close()
    finally:
        await db.close()
    counts: dict[str, int] = {}
    for r in rows:
        et = r[0]
        pj = json.loads(r[1]) if r[1] else {}
        key = et if et != "RESPONSE_CHUNK" else f"RESPONSE_CHUNK:{pj.get('chunk_type', '')}"
        counts[key] = counts.get(key, 0) + 1
    return counts


async def _list_chunk_contents(
    tmp_db_path: str, session_id: str, chunk_type: str
) -> list[str]:
    import json
    db = await connect(tmp_db_path)
    try:
        cur = await db.execute(
            """
            SELECT payload_json FROM session_events
            WHERE session_id = ? AND event_type = 'RESPONSE_CHUNK'
            ORDER BY seq
            """,
            (session_id,),
        )
        rows = await cur.fetchall()
        await cur.close()
    finally:
        await db.close()
    return [
        json.loads(r[0]).get("content", "")
        for r in rows
        if json.loads(r[0]).get("chunk_type") == chunk_type
    ]


# ─── send_message path ────────────────────────────────────────────────────


@pytest.mark.asyncio
@respx.mock
async def test_send_message_coalesces_text_delta_runs(tmp_db_path: str) -> None:
    """50 contiguous text_delta tokens → 1 aggregate RESPONSE_CHUNK."""
    async with httpx.AsyncClient() as http:
        orch = await _make_orchestrator(
            tmp_db_path,
            http,
            lambda rid: _TokenStormL1(
                runtime_id=rid, text_tokens=50, thinking_tokens=0, tool_rounds=0
            ),
        )
        sid = "sess_coalesce_td"
        await _seed_session_row(tmp_db_path, sid)
        orch._l1_clients[sid] = orch._l1_factory("grid-runtime")
        orch._l1_session_ids[sid] = sid

        await orch.send_message(sid, "hello")

    counts = await _count_events(tmp_db_path, sid)
    # Exactly one coalesced text_delta chunk from 50 source tokens.
    assert counts.get("RESPONSE_CHUNK:text_delta", 0) == 1, counts
    # Content must be the concatenation of all 50 tokens.
    contents = await _list_chunk_contents(tmp_db_path, sid, "text_delta")
    assert contents == ["".join(f"final_{i}" for i in range(50))], contents
    # `done` chunk still present (lifecycle preserved).
    assert counts.get("RESPONSE_CHUNK:done", 0) == 1, counts


@pytest.mark.asyncio
@respx.mock
async def test_send_message_coalesces_thinking_runs(tmp_db_path: str) -> None:
    """30 contiguous thinking tokens → 1 aggregate RESPONSE_CHUNK."""
    async with httpx.AsyncClient() as http:
        orch = await _make_orchestrator(
            tmp_db_path,
            http,
            lambda rid: _TokenStormL1(
                runtime_id=rid, text_tokens=0, thinking_tokens=30, tool_rounds=0
            ),
        )
        sid = "sess_coalesce_thinking"
        await _seed_session_row(tmp_db_path, sid)
        orch._l1_clients[sid] = orch._l1_factory("grid-runtime")
        orch._l1_session_ids[sid] = sid

        await orch.send_message(sid, "think")

    counts = await _count_events(tmp_db_path, sid)
    assert counts.get("RESPONSE_CHUNK:thinking", 0) == 1, counts
    contents = await _list_chunk_contents(tmp_db_path, sid, "thinking")
    assert contents == ["".join(f"t{i}" for i in range(30))], contents


@pytest.mark.asyncio
@respx.mock
async def test_send_message_flushes_deltas_on_tool_boundary(
    tmp_db_path: str,
) -> None:
    """Tool rounds split text_delta runs — each contiguous run becomes its
    own aggregate chunk so ordering with tool_start/tool_result is preserved."""
    async with httpx.AsyncClient() as http:
        orch = await _make_orchestrator(
            tmp_db_path,
            http,
            lambda rid: _TokenStormL1(
                runtime_id=rid, text_tokens=10, thinking_tokens=0, tool_rounds=2
            ),
        )
        sid = "sess_coalesce_flush"
        await _seed_session_row(tmp_db_path, sid)
        orch._l1_clients[sid] = orch._l1_factory("grid-runtime")
        orch._l1_session_ids[sid] = sid

        await orch.send_message(sid, "calibrate")

    counts = await _count_events(tmp_db_path, sid)
    # 2 tool rounds → 2 tool_start + 2 tool_result (each lifecycle chunk
    # flushes any pending delta buffer, so these are still 1-per-round).
    assert counts.get("RESPONSE_CHUNK:tool_start", 0) == 2, counts
    assert counts.get("RESPONSE_CHUNK:tool_result", 0) == 2, counts
    # Text delta runs are split only by NON-delta chunks. In the stub's
    # event stream: (tool_start, tool_result, 5×delta, tool_start,
    # tool_result, 5×delta, 10×delta, done). The first 5×delta run is
    # split from the second by `tool_start` → aggregate #1. The second
    # 5×delta and the final 10×delta are contiguous (no non-delta chunk
    # between them) → they merge into aggregate #2. `done` then flushes.
    # So: 2 aggregate chunks, NOT 20 raw tokens.
    assert counts.get("RESPONSE_CHUNK:text_delta", 0) == 2, counts


@pytest.mark.asyncio
@respx.mock
async def test_send_message_event_explosion_capped(tmp_db_path: str) -> None:
    """A 100-token turn must NOT produce 100 RESPONSE_CHUNK events in DB."""
    async with httpx.AsyncClient() as http:
        orch = await _make_orchestrator(
            tmp_db_path,
            http,
            lambda rid: _TokenStormL1(
                runtime_id=rid, text_tokens=100, thinking_tokens=100, tool_rounds=3
            ),
        )
        sid = "sess_explosion_capped"
        await _seed_session_row(tmp_db_path, sid)
        orch._l1_clients[sid] = orch._l1_factory("grid-runtime")
        orch._l1_session_ids[sid] = sid

        await orch.send_message(sid, "go")

    counts = await _count_events(tmp_db_path, sid)
    # Raw source chunks: 100 text + 100 thinking + 3*(5 text + tool_start +
    # tool_result) = 100 + 100 + 3*5 + 3*2 = 221 text_delta + 100 thinking
    # + 6 tool chunks + 1 done ≈ 221 + 100 + 7 = 328 raw chunks.
    # After coalescing: ≤ ~10 RESPONSE_CHUNK events.
    total_chunks = sum(v for k, v in counts.items() if k.startswith("RESPONSE_CHUNK"))
    assert total_chunks < 20, (
        f"coalescing failed — got {total_chunks} chunks, expected <20: {counts}"
    )


# ─── stream_message path ───────────────────────────────────────────────────


@pytest.mark.asyncio
@respx.mock
async def test_stream_message_yields_every_delta_but_coalesces_db(
    tmp_db_path: str,
) -> None:
    """SSE consumers see every delta (typewriter UX preserved) while the
    DB only records one aggregate chunk per contiguous delta run."""
    async with httpx.AsyncClient() as http:
        orch = await _make_orchestrator(
            tmp_db_path,
            http,
            lambda rid: _TokenStormL1(
                runtime_id=rid, text_tokens=20, thinking_tokens=0, tool_rounds=0
            ),
        )
        sid = "sess_stream_coalesce"
        await _seed_session_row(tmp_db_path, sid)
        orch._l1_clients[sid] = orch._l1_factory("grid-runtime")
        orch._l1_session_ids[sid] = sid

        yielded_text_deltas = 0
        async for ev in orch.stream_message(sid, "stream"):
            if ev.get("event") == "chunk" and ev["data"].get("chunk_type") == "text_delta":
                yielded_text_deltas += 1

    # SSE yielded all 20 deltas one-by-one.
    assert yielded_text_deltas == 20

    # DB has only 1 aggregate text_delta chunk.
    counts = await _count_events(tmp_db_path, sid)
    assert counts.get("RESPONSE_CHUNK:text_delta", 0) == 1, counts


# ─── ADR-V2-021 chunk_type wire-string contract (Phase 3.5 S2.T1) ──────────


class _ProtoStreamL1:
    """Stub L1 whose ``send`` returns real ``SendResponse`` proto messages
    funneled through ``_send_response_to_dict`` — the exact boundary the
    real ``L1RuntimeClient.send`` uses. This exercises the ChunkType
    enum int → wire string conversion end-to-end, which a raw-dict stub
    cannot.

    Pre-fix bug (before ADR-V2-021 S2.T1): ``_send_response_to_dict`` put
    the raw enum ``int`` into the dict, so ``ctype == "text_delta"``
    string compares in the orchestrator NEVER matched, leaving
    ``response_text=""``. This test pins the fix.
    """

    def __init__(self, runtime_id: str = "grid-runtime", text_tokens: int = 5) -> None:
        self.runtime_id = runtime_id
        self.text_tokens = text_tokens

    async def initialize(self, payload_dict: dict[str, Any]) -> dict[str, str]:
        return {
            "session_id": payload_dict.get("session_id", "mock"),
            "runtime_id": self.runtime_id,
        }

    async def send(
        self, session_id: str, content: str, message_type: str = "text"
    ) -> AsyncIterator[dict[str, Any]]:
        from eaasp_l4_orchestration._proto.eaasp.runtime.v2 import (
            common_pb2,
            runtime_pb2,
        )
        from eaasp_l4_orchestration.l1_client import _send_response_to_dict

        for i in range(self.text_tokens):
            proto_chunk = runtime_pb2.SendResponse(
                chunk_type=common_pb2.CHUNK_TYPE_TEXT_DELTA,
                content=f"tok{i}",
            )
            yield _send_response_to_dict(proto_chunk)

        yield _send_response_to_dict(
            runtime_pb2.SendResponse(chunk_type=common_pb2.CHUNK_TYPE_DONE)
        )

    async def terminate(self) -> None:
        pass

    async def close(self) -> None:
        pass


@pytest.mark.asyncio
@respx.mock
async def test_response_text_accumulates_through_proto_boundary(
    tmp_db_path: str,
) -> None:
    """End-to-end regression for ADR-V2-021 S2.T1.

    Before the fix, ``_send_response_to_dict`` exposed the raw enum int
    (``chunk.chunk_type`` == ``1`` for TEXT_DELTA). Downstream
    ``ctype == "text_delta"`` comparisons in ``send_message`` never
    matched, so ``response_text`` was always "". With the enum→wire
    conversion at the proto boundary, text_delta content now accumulates
    correctly.
    """
    async with httpx.AsyncClient() as http:
        orch = await _make_orchestrator(
            tmp_db_path,
            http,
            lambda rid: _ProtoStreamL1(runtime_id=rid, text_tokens=5),
        )
        sid = "sess_proto_wire"
        await _seed_session_row(tmp_db_path, sid)
        orch._l1_clients[sid] = orch._l1_factory("grid-runtime")
        orch._l1_session_ids[sid] = sid

        result = await orch.send_message(sid, "hello")

    # Proto boundary produced wire strings, so the string compare matched
    # and full_text_parts accumulated.
    assert result["response_text"] != "", (
        "response_text empty — enum→wire conversion likely broken at boundary"
    )
    assert result["response_text"] == "tok0tok1tok2tok3tok4"

    # Persisted chunk_type must also be the wire string (not an int) so
    # the DB is readable by the SSE serialiser and CLI consumer.
    counts = await _count_events(tmp_db_path, sid)
    assert counts.get("RESPONSE_CHUNK:text_delta", 0) == 1, counts
    assert counts.get("RESPONSE_CHUNK:done", 0) == 1, counts


def test_chunk_type_to_wire_known_variants() -> None:
    """Unit-level lock on :func:`_chunk_type_to_wire` (ADR-V2-021 §2)."""
    from eaasp_l4_orchestration._proto.eaasp.runtime.v2 import common_pb2
    from eaasp_l4_orchestration.l1_client import _chunk_type_to_wire

    # All 7 contract-defined variants map to lowercase snake_case names.
    assert _chunk_type_to_wire(common_pb2.CHUNK_TYPE_TEXT_DELTA) == "text_delta"
    assert _chunk_type_to_wire(common_pb2.CHUNK_TYPE_THINKING) == "thinking"
    assert _chunk_type_to_wire(common_pb2.CHUNK_TYPE_TOOL_START) == "tool_start"
    assert _chunk_type_to_wire(common_pb2.CHUNK_TYPE_TOOL_RESULT) == "tool_result"
    assert _chunk_type_to_wire(common_pb2.CHUNK_TYPE_DONE) == "done"
    assert _chunk_type_to_wire(common_pb2.CHUNK_TYPE_ERROR) == "error"
    assert (
        _chunk_type_to_wire(common_pb2.CHUNK_TYPE_WORKFLOW_CONTINUATION)
        == "workflow_continuation"
    )
    # UNSPECIFIED (0) and unknown future ints collapse to "" — whitelist
    # rejects that and contract tests catch it.
    assert _chunk_type_to_wire(0) == ""
    assert _chunk_type_to_wire(9999) == ""
