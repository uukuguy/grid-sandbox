"""SKILL.md bidirectional parser — parse ↔ render."""

from __future__ import annotations

from pathlib import Path

import yaml
from eaasp.models.skill import Skill, SkillFrontmatter


class SkillParser:
    """Parse and render SKILL.md files."""

    @staticmethod
    def parse(content: str) -> Skill:
        """Parse a SKILL.md string into a Skill model.

        Delegates to ``Skill.from_skill_md`` for consistency.
        """
        return Skill.from_skill_md(content)

    @staticmethod
    def render(skill: Skill) -> str:
        """Render a Skill model as SKILL.md content.

        Delegates to ``Skill.to_skill_md`` for consistency.
        """
        return skill.to_skill_md()

    @staticmethod
    def parse_file(path: Path) -> Skill:
        """Read a SKILL.md file and parse it into a Skill model."""
        return Skill.from_file(path)
