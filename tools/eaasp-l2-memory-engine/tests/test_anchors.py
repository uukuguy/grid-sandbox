"""Layer 1 — Evidence anchor append-only store tests."""

from __future__ import annotations

import pytest

from eaasp_l2_memory_engine.anchors import AnchorIn, AnchorStore


pytestmark = pytest.mark.asyncio


async def test_write_and_read_anchor(anchor_store: AnchorStore) -> None:
    out = await anchor_store.write(
        AnchorIn(
            event_id="evt_1",
            session_id="sess_1",
            type="tool_result",
            data_ref="s3://bucket/obj",
            snapshot_hash="sha256:deadbeef",
            source_system="claude-code-runtime",
            rule_version="v2.0.0",
            metadata={"tool": "bash"},
        )
    )
    assert out.anchor_id.startswith("anc_")
    assert out.event_id == "evt_1"
    assert out.metadata == {"tool": "bash"}

    fetched = await anchor_store.get(out.anchor_id)
    assert fetched is not None
    assert fetched.anchor_id == out.anchor_id
    assert fetched.metadata == {"tool": "bash"}


async def test_list_anchors_by_event(anchor_store: AnchorStore) -> None:
    for i in range(3):
        await anchor_store.write(
            AnchorIn(
                event_id="evt_batch",
                session_id=f"sess_{i}",
                type="tool_result",
            )
        )
    await anchor_store.write(
        AnchorIn(event_id="other_evt", session_id="sess_x", type="tool_result")
    )

    rows = await anchor_store.list_by_event("evt_batch")
    assert len(rows) == 3
    assert all(r.event_id == "evt_batch" for r in rows)


async def test_list_anchors_by_session(anchor_store: AnchorStore) -> None:
    for i in range(2):
        await anchor_store.write(
            AnchorIn(
                event_id=f"evt_{i}",
                session_id="sess_same",
                type="policy_decision",
            )
        )
    rows = await anchor_store.list_by_session("sess_same")
    assert len(rows) == 2


async def test_anchor_not_found(anchor_store: AnchorStore) -> None:
    assert await anchor_store.get("anc_nonexistent") is None
