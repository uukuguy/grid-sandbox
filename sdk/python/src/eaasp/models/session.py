"""Session model — session configuration and state.

Aligned with proto eaasp.runtime.v1.SessionPayload and SessionState.
"""

from __future__ import annotations

from typing import Literal

from pydantic import BaseModel, Field


class SessionConfig(BaseModel):
    """Configuration for initializing a session.

    Maps to proto SessionPayload fields.
    """

    user_id: str = ""
    user_role: str = ""
    org_unit: str = ""
    managed_hooks_json: str | None = None
    quotas: dict[str, str] = Field(default_factory=dict)
    context: dict[str, str] = Field(default_factory=dict)
    hook_bridge_url: str | None = None
    telemetry_endpoint: str | None = None
    # L2 Skill Registry fields (proto v1.3)
    skill_ids: list[str] = Field(default_factory=list)
    skill_registry_url: str | None = None
    allowed_skill_search: bool = False
    skill_search_scope: list[str] = Field(default_factory=list)


class SessionState(BaseModel):
    """Serialized session state for persistence/restore.

    Aligned with proto SessionState.
    """

    session_id: str
    state_data: bytes = b""
    runtime_id: str = ""
    created_at: str = ""  # ISO 8601
    state_format: Literal["rust-serde-v1", "python-json", "ts-json"] = "python-json"
