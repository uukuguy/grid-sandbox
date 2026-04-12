"""Tests for context_assembly — P5 provider hint injection."""

from __future__ import annotations

from eaasp_l4_orchestration.context_assembly import build_session_payload


def _base_payload(**overrides):
    defaults = {
        "session_id": "sess-test",
        "user_id": "u-1",
        "runtime_id": "grid-runtime",
        "policy_context": {"hooks": [], "policy_version": "v1", "deploy_timestamp": "", "org_unit": "", "quotas": {}},
        "event_context": None,
        "memory_refs": [],
        "skill_instructions": {},
        "user_preferences": {"user_id": "u-1", "prefs": {}},
        "created_at": 1234567890,
    }
    defaults.update(overrides)
    return build_session_payload(**defaults)


def test_provider_hint_from_env(monkeypatch):
    """LLM_PROVIDER and LLM_MODEL env vars are injected into P5."""
    monkeypatch.setenv("LLM_PROVIDER", "openai")
    monkeypatch.setenv("LLM_MODEL", "gpt-4o")
    payload = _base_payload()
    assert payload["user_preferences"]["llm_provider"] == "openai"
    assert payload["user_preferences"]["llm_model"] == "gpt-4o"


def test_provider_hint_env_not_set(monkeypatch):
    """When env vars are not set, P5 should not contain provider/model keys."""
    monkeypatch.delenv("LLM_PROVIDER", raising=False)
    monkeypatch.delenv("LLM_MODEL", raising=False)
    payload = _base_payload()
    # Keys should be absent (not injected).
    assert "llm_provider" not in payload["user_preferences"]
    assert "llm_model" not in payload["user_preferences"]


def test_provider_hint_explicit_takes_precedence(monkeypatch):
    """If user_preferences already has llm_provider, env should NOT override."""
    monkeypatch.setenv("LLM_PROVIDER", "openai")
    monkeypatch.setenv("LLM_MODEL", "gpt-4o")
    payload = _base_payload(
        user_preferences={
            "user_id": "u-1",
            "prefs": {},
            "llm_provider": "anthropic",
            "llm_model": "claude-sonnet-4-20250514",
        }
    )
    assert payload["user_preferences"]["llm_provider"] == "anthropic"
    assert payload["user_preferences"]["llm_model"] == "claude-sonnet-4-20250514"


def test_payload_structure_unchanged():
    """Verify the overall payload structure is preserved with provider injection."""
    payload = _base_payload()
    assert "session_id" in payload
    assert "policy_context" in payload
    assert "memory_refs" in payload
    assert "user_preferences" in payload
    assert payload["allow_trim_p5"] is True
    assert payload["allow_trim_p4"] is False
