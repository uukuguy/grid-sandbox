"""Skill model — the core abstraction in EAASP.

A Skill is a SKILL.md file with YAML frontmatter + natural language prose.
"""

from __future__ import annotations

from pathlib import Path
from typing import Literal

import yaml
from pydantic import BaseModel, Field


class ScopedHook(BaseModel):
    """A hook binding scoped to a Skill."""

    event: Literal["PreToolUse", "PostToolUse", "Stop"]
    handler_type: Literal["command", "http", "prompt", "agent"]
    config: dict = Field(default_factory=dict)
    match: dict | None = None


class SkillFrontmatter(BaseModel):
    """YAML frontmatter of a SKILL.md file."""

    name: str
    version: str = "1.0.0"
    description: str = ""
    author: str = ""
    tags: list[str] = Field(default_factory=list)
    skill_type: Literal["workflow", "production", "domain", "meta"] = "workflow"
    preferred_runtime: str | None = None
    compatible_runtimes: list[str] = Field(default_factory=list)
    hooks: list[ScopedHook] = Field(default_factory=list)
    dependencies: list[str] = Field(default_factory=list)
    scope: Literal["global", "bu", "dept", "team"] = "team"


class Skill(BaseModel):
    """Complete Skill = frontmatter + prose."""

    frontmatter: SkillFrontmatter
    prose: str = ""

    def to_skill_md(self) -> str:
        """Render as SKILL.md content (YAML frontmatter between --- + prose)."""
        fm_dict = self.frontmatter.model_dump(exclude_none=True)
        # Convert ScopedHook models to dicts for clean YAML
        if "hooks" in fm_dict:
            fm_dict["hooks"] = [
                {k: v for k, v in h.items() if v is not None} for h in fm_dict["hooks"]
            ]
        yaml_str = yaml.dump(fm_dict, default_flow_style=False, allow_unicode=True, sort_keys=False)
        return f"---\n{yaml_str}---\n\n{self.prose}"

    @classmethod
    def from_skill_md(cls, content: str) -> Skill:
        """Parse a SKILL.md string into a Skill model."""
        parts = content.split("---", 2)
        if len(parts) < 3:
            raise ValueError("SKILL.md must have YAML frontmatter between --- delimiters")
        yaml_str = parts[1].strip()
        prose = parts[2].strip()
        fm_data = yaml.safe_load(yaml_str)
        if not isinstance(fm_data, dict):
            raise ValueError("SKILL.md frontmatter must be a YAML mapping")
        frontmatter = SkillFrontmatter(**fm_data)
        return cls(frontmatter=frontmatter, prose=prose)

    @classmethod
    def from_file(cls, path: Path) -> Skill:
        """Read a SKILL.md file and parse it."""
        content = path.read_text(encoding="utf-8")
        return cls.from_skill_md(content)
