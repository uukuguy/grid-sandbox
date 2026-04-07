"""E2E: Hook 强制执行测试 (4 tests).

Verifies that compiled policies are correctly applied to sessions.
"""

from __future__ import annotations

import json

import pytest

from tests.e2e.helpers import create_session


@pytest.mark.e2e
@pytest.mark.mock_llm
def test_policies_compiled_into_hooks(l3_client):
    """Deployed policies are compiled into hooks and attached to session."""
    session = create_session(l3_client)
    gov = session["governance_summary"]
    # Enterprise (2 rules) + BU (2 rules) = 4 total
    # After merge, same-id rules override, so we expect >= 3
    assert gov["hooks_count"] >= 3


@pytest.mark.e2e
@pytest.mark.mock_llm
def test_pii_guard_in_compiled_hooks(l3_client):
    """PII guard rule from enterprise policy is present in compiled hooks."""
    # Get the enterprise policy details
    resp = l3_client.get("/v1/policies/enterprise-security-baseline")
    assert resp.status_code == 200
    policy = resp.json()

    hooks = json.loads(policy["compiled_hooks_json"])
    rule_ids = {r["id"] for r in hooks["rules"]}
    assert "pii-guard" in rule_ids

    # Verify PII guard denies
    pii_rule = next(r for r in hooks["rules"] if r["id"] == "pii-guard")
    assert pii_rule["action"] == "deny"
    assert pii_rule["hook_type"] == "pre_tool_call"
    assert "file_write" in pii_rule["tool_pattern"]


@pytest.mark.e2e
@pytest.mark.mock_llm
def test_audit_rule_present(l3_client):
    """Audit rule from enterprise policy enables audit logging."""
    resp = l3_client.get("/v1/policies/enterprise-security-baseline")
    hooks = json.loads(resp.json()["compiled_hooks_json"])

    audit_rule = next(r for r in hooks["rules"] if r["id"] == "audit-all-writes")
    assert audit_rule["action"] == "allow"
    assert audit_rule.get("audit") is True


@pytest.mark.e2e
@pytest.mark.mock_llm
def test_stop_rule_present(l3_client):
    """Stop rule from BU policy enforces checklist completion."""
    resp = l3_client.get("/v1/policies/hr-department-policies")
    assert resp.status_code == 200
    hooks = json.loads(resp.json()["compiled_hooks_json"])

    stop_rule = next(r for r in hooks["rules"] if r["id"] == "checklist-enforcement")
    assert stop_rule["hook_type"] == "stop"
    assert stop_rule["action"] == "deny"
