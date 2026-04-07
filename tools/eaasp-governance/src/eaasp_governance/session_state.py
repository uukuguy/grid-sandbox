"""Session state machine for L3 governance sessions.

States: creating → active → terminating → terminated
                 → error (from any state)
"""

from __future__ import annotations

import logging
from dataclasses import dataclass, field
from datetime import datetime, timezone
from enum import Enum

logger = logging.getLogger(__name__)


class SessionStatus(str, Enum):
    CREATING = "creating"
    ACTIVE = "active"
    TERMINATING = "terminating"
    TERMINATED = "terminated"
    ERROR = "error"


_VALID_TRANSITIONS: dict[SessionStatus, set[SessionStatus]] = {
    SessionStatus.CREATING: {SessionStatus.ACTIVE, SessionStatus.ERROR},
    SessionStatus.ACTIVE: {SessionStatus.TERMINATING, SessionStatus.ERROR},
    SessionStatus.TERMINATING: {SessionStatus.TERMINATED, SessionStatus.ERROR},
    SessionStatus.TERMINATED: set(),
    SessionStatus.ERROR: set(),
}


@dataclass
class GovernanceSession:
    """L3-side session state (not the L1 session — this tracks governance)."""

    session_id: str
    user_id: str
    org_unit: str
    skill_id: str
    runtime_id: str = ""
    runtime_endpoint: str = ""
    status: SessionStatus = SessionStatus.CREATING
    managed_hooks_digest: str = ""
    hooks_count: int = 0
    created_at: str = field(default_factory=lambda: datetime.now(timezone.utc).isoformat())
    updated_at: str = field(default_factory=lambda: datetime.now(timezone.utc).isoformat())

    def transition(self, target: SessionStatus) -> None:
        """Transition to a new status. Raises ValueError on invalid transition."""
        if target not in _VALID_TRANSITIONS.get(self.status, set()):
            raise ValueError(
                f"Invalid transition: {self.status.value} → {target.value}"
            )
        logger.info("Session %s: %s → %s", self.session_id, self.status.value, target.value)
        self.status = target
        self.updated_at = datetime.now(timezone.utc).isoformat()

    def to_dict(self) -> dict:
        return {
            "session_id": self.session_id,
            "user_id": self.user_id,
            "org_unit": self.org_unit,
            "skill_id": self.skill_id,
            "runtime_id": self.runtime_id,
            "runtime_endpoint": self.runtime_endpoint,
            "status": self.status.value,
            "managed_hooks_digest": self.managed_hooks_digest,
            "hooks_count": self.hooks_count,
            "created_at": self.created_at,
            "updated_at": self.updated_at,
        }
