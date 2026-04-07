"""E2E: 会话生命周期测试 (2 tests).

Verifies full session lifecycle including message send and termination.
"""

from __future__ import annotations

import pytest

from tests.e2e.helpers import create_session


@pytest.mark.e2e
@pytest.mark.mock_llm
def test_session_message_round_trip(l3_client):
    """Send a message through the session and receive response chunks."""
    session = create_session(l3_client)
    session_id = session["session_id"]

    # Send message
    resp = l3_client.post(
        f"/v1/sessions/{session_id}/message",
        json={"content": "请帮张三办理入职手续"},
    )
    assert resp.status_code == 200
    data = resp.json()
    assert "chunks" in data
    assert len(data["chunks"]) > 0

    chunk_types = {c["chunk_type"] for c in data["chunks"]}
    assert "text_delta" in chunk_types or "done" in chunk_types


@pytest.mark.e2e
@pytest.mark.mock_llm
def test_session_full_lifecycle(l3_client):
    """Full lifecycle: create → send → check status → terminate."""
    # Create
    session = create_session(l3_client)
    session_id = session["session_id"]

    # Send message
    l3_client.post(
        f"/v1/sessions/{session_id}/message",
        json={"content": "开始入职流程"},
    )

    # Check status
    resp = l3_client.get(f"/v1/sessions/{session_id}")
    assert resp.json()["status"] == "active"

    # Terminate
    resp = l3_client.delete(f"/v1/sessions/{session_id}")
    assert resp.json()["status"] == "terminated"

    # Verify terminated
    resp = l3_client.get(f"/v1/sessions/{session_id}")
    assert resp.json()["status"] == "terminated"
