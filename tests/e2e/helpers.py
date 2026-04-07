"""E2E test helper utilities."""

from __future__ import annotations


def create_session(client, skill_id="hr-onboarding", user_id="e2e-user") -> dict:
    """Helper: create a session and return the response."""
    resp = client.post("/v1/sessions", json={
        "user_id": user_id,
        "user_role": "hr_specialist",
        "org_unit": "hr-dept",
        "skill_id": skill_id,
        "runtime_preference": "grid",
    })
    assert resp.status_code == 200
    return resp.json()
