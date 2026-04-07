"""Mock L2 Skill Registry HTTP client for MVP.

In production, this would use httpx to call L2 REST API.
MVP returns mock skill content to verify the three-way handshake.
"""

from __future__ import annotations

import logging
from dataclasses import dataclass

logger = logging.getLogger(__name__)


@dataclass
class SkillContent:
    """Skill content fetched from L2 registry."""

    skill_id: str
    name: str
    frontmatter_yaml: str
    prose: str


# Mock skill database
_MOCK_SKILLS: dict[str, SkillContent] = {
    "hr-onboarding": SkillContent(
        skill_id="hr-onboarding",
        name="HR 新员工入职",
        frontmatter_yaml=(
            "name: hr-onboarding\n"
            "version: '1.0.0'\n"
            "description: 新员工入职流程自动化\n"
            "preferred_runtime: grid\n"
            "scope: bu\n"
        ),
        prose="你是一位经验丰富的 HR 专家，负责协助新员工完成入职流程。",
    ),
}


class L2RegistryClient:
    """Mock HTTP client for L2 Skill Registry."""

    def __init__(self, base_url: str) -> None:
        self.base_url = base_url.rstrip("/")

    async def get_skill_content(self, skill_id: str) -> SkillContent | None:
        """GET /api/v1/skills/{id}/content — fetch skill content."""
        # MVP: use mock data
        skill = _MOCK_SKILLS.get(skill_id)
        if skill:
            logger.info("L2 GetSkill: %s → found", skill_id)
        else:
            logger.warning("L2 GetSkill: %s → not found", skill_id)
        return skill

    async def list_skills(self) -> list[dict]:
        """GET /api/v1/skills — list available skills."""
        return [
            {"id": s.skill_id, "name": s.name}
            for s in _MOCK_SKILLS.values()
        ]
