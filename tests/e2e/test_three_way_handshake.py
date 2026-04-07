"""E2E: 三方握手测试 (3 tests).

Verifies L4→L3→L2→L1 handshake flow via the session control API.
"""

from __future__ import annotations

import pytest

from tests.e2e.helpers import create_session


@pytest.mark.e2e
@pytest.mark.mock_llm
def test_handshake_creates_session(l3_client):
    """Three-way handshake creates a valid session with runtime assignment."""
    session = create_session(l3_client)
    assert "session_id" in session
    assert session["session_id"].startswith("sess-")
    assert session["runtime_id"] in ("grid", "claude-code")
    assert session["runtime_endpoint"]


@pytest.mark.e2e
@pytest.mark.mock_llm
def test_handshake_includes_governance(l3_client):
    """Three-way handshake returns governance summary with hooks."""
    session = create_session(l3_client)
    gov = session["governance_summary"]
    assert gov["hooks_count"] > 0
    assert "managed_hooks_digest" in gov
    assert len(gov["managed_hooks_digest"]) > 0
    assert "managed" in gov["scope_chain"]


@pytest.mark.e2e
@pytest.mark.mock_llm
def test_handshake_skill_loaded(l3_client):
    """Three-way handshake loads skill into L1 runtime."""
    session = create_session(l3_client)
    session_id = session["session_id"]

    # Verify session is active (skill loaded successfully)
    resp = l3_client.get(f"/v1/sessions/{session_id}")
    assert resp.status_code == 200
    data = resp.json()
    assert data["status"] == "active"
    assert data["skill_id"] == "hr-onboarding"
