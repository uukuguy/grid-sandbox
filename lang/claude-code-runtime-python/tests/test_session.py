"""Tests for SessionManager."""

from claude_code_runtime.session import Session, SessionManager, SessionState


def test_create_session():
    mgr = SessionManager()
    s = mgr.create(user_id="u1", user_role="dev", org_unit="eng")
    assert s.session_id.startswith("crt-")
    assert s.user_id == "u1"
    assert s.state == SessionState.ACTIVE
    assert mgr.count == 1


def test_get_session():
    mgr = SessionManager()
    s = mgr.create(user_id="u1")
    assert mgr.get(s.session_id) is s
    assert mgr.get("nonexistent") is None


def test_pause_resume():
    mgr = SessionManager()
    s = mgr.create(user_id="u1")
    assert mgr.pause(s.session_id) is True
    assert s.state == SessionState.PAUSED
    assert mgr.pause(s.session_id) is False  # already paused

    assert mgr.resume(s.session_id) is True
    assert s.state == SessionState.ACTIVE
    assert mgr.resume(s.session_id) is False  # already active


def test_terminate():
    mgr = SessionManager()
    s = mgr.create(user_id="u1")
    sid = s.session_id
    terminated = mgr.terminate(sid)
    assert terminated is not None
    assert terminated.state == SessionState.TERMINATED
    assert mgr.get(sid) is None
    assert mgr.count == 0


def test_session_serialization():
    s = Session(
        session_id="crt-abc",
        user_id="u1",
        user_role="dev",
        skills=[{"skill_id": "s1", "name": "Test"}],
    )
    data = s.to_dict()
    assert data["session_id"] == "crt-abc"
    assert data["user_id"] == "u1"
    assert len(data["skills"]) == 1

    restored = Session.from_dict(data)
    assert restored.session_id == "crt-abc"
    assert restored.user_id == "u1"
    assert restored.state == SessionState.ACTIVE


def test_restore_session():
    mgr = SessionManager()
    data = {
        "session_id": "crt-old",
        "user_id": "u1",
        "state": "paused",
        "skills": [],
        "mcp_servers": [],
        "telemetry_events": [],
        "context": {},
    }
    s = mgr.restore(data)
    assert s.session_id == "crt-old"
    assert s.state == SessionState.ACTIVE  # restored sessions become active
    assert mgr.get("crt-old") is s


# ── D2-py — memory_refs + policy_context plumbing ───────────────────


def test_session_carries_memory_refs():
    """Session.memory_refs + policy_context survive create() and round-trip.

    D2-py: SessionPayload P3 memory_refs and P1 policy_context are projected
    into plain dicts by the Initialize handler before being stored on the
    Session dataclass. This test exercises the dataclass/SessionManager layer
    independently of the gRPC service.
    """
    memory_refs = [
        {
            "memory_id": "mem-1",
            "memory_type": "fact",
            "relevance_score": 0.95,
            "content": "Temperature threshold is 75C",
            "source_session_id": "s-prev",
            "created_at": "2026-04-10T00:00:00Z",
            "tags": {},
        },
        {
            "memory_id": "mem-2",
            "memory_type": "preference",
            "relevance_score": 0.80,
            "content": "User prefers conservative thresholds",
            "source_session_id": "s-prev",
            "created_at": "2026-04-10T00:00:00Z",
            "tags": {"device": "transformer-001"},
        },
    ]
    policy_context = {
        "org_unit": "engineering",
        "policy_version": "v2.0-20260412",
        "hooks": [
            {
                "hook_id": "h1",
                "hook_type": "pre_tool_call",
                "condition": "tool:^bash$",
                "action": "deny",
                "precedence": 1,
                "scope": "managed",
            }
        ],
    }

    # 1. Via SessionManager.create() — the path Initialize uses.
    mgr = SessionManager()
    session = mgr.create(
        user_id="alice",
        org_unit="engineering",
        memory_refs=memory_refs,
        policy_context=policy_context,
    )
    assert session.memory_refs == memory_refs
    assert session.policy_context == policy_context
    assert session.preamble_injected is False

    # 2. to_dict() / from_dict() round-trip must preserve both fields.
    data = session.to_dict()
    assert data["memory_refs"] == memory_refs
    assert data["policy_context"] == policy_context
    assert data["preamble_injected"] is False

    restored = Session.from_dict(data)
    assert restored.memory_refs == memory_refs
    assert restored.policy_context == policy_context
    assert restored.preamble_injected is False

    # 3. Defaults: create() without the new kwargs still works (backcompat).
    legacy = mgr.create(user_id="bob")
    assert legacy.memory_refs == []
    assert legacy.policy_context is None
    assert legacy.preamble_injected is False
