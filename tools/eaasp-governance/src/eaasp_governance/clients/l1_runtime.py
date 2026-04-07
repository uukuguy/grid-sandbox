"""Mock L1 Runtime gRPC client for MVP.

In production, this would use grpcio to call RuntimeService.
MVP uses in-memory mock to verify API contract flow.
"""

from __future__ import annotations

import logging
import uuid
from dataclasses import dataclass, field

logger = logging.getLogger(__name__)


@dataclass
class MockSession:
    session_id: str
    user_id: str
    org_unit: str
    managed_hooks_json: str = ""
    skills_loaded: list[str] = field(default_factory=list)
    terminated: bool = False


class L1RuntimeClient:
    """Mock gRPC client for L1 RuntimeService.

    Simulates Initialize, LoadSkill, Send, Terminate.
    """

    def __init__(self, endpoint: str) -> None:
        self.endpoint = endpoint
        self._sessions: dict[str, MockSession] = {}

    async def initialize(
        self,
        user_id: str,
        org_unit: str,
        managed_hooks_json: str,
        skill_ids: list[str] | None = None,
        skill_registry_url: str = "",
    ) -> str:
        """Initialize a session on the L1 runtime. Returns session_id."""
        session_id = f"sess-{uuid.uuid4().hex[:8]}"
        self._sessions[session_id] = MockSession(
            session_id=session_id,
            user_id=user_id,
            org_unit=org_unit,
            managed_hooks_json=managed_hooks_json,
        )
        logger.info("L1 Initialize: %s @ %s", session_id, self.endpoint)
        return session_id

    async def load_skill(
        self, session_id: str, skill_id: str, frontmatter_yaml: str, prose: str
    ) -> bool:
        """Load a skill into the session."""
        session = self._sessions.get(session_id)
        if not session:
            raise ValueError(f"Unknown session: {session_id}")
        session.skills_loaded.append(skill_id)
        logger.info("L1 LoadSkill: %s → session %s", skill_id, session_id)
        return True

    async def send(self, session_id: str, content: str) -> list[dict]:
        """Send a user message. Returns mock response chunks."""
        session = self._sessions.get(session_id)
        if not session:
            raise ValueError(f"Unknown session: {session_id}")
        return [
            {"chunk_type": "text_delta", "content": f"Processing: {content[:50]}"},
            {"chunk_type": "done", "content": ""},
        ]

    async def terminate(self, session_id: str) -> dict:
        """Terminate a session."""
        session = self._sessions.get(session_id)
        if not session:
            raise ValueError(f"Unknown session: {session_id}")
        session.terminated = True
        logger.info("L1 Terminate: %s", session_id)
        return {"success": True, "session_id": session_id}

    def get_session(self, session_id: str) -> MockSession | None:
        return self._sessions.get(session_id)
