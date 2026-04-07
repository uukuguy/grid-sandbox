"""Playbook model — multi-Skill orchestration workflows."""

from __future__ import annotations

from typing import Literal

from pydantic import BaseModel, Field


class PlaybookStep(BaseModel):
    """A single step in a Playbook execution plan."""

    name: str
    skill_id: str  # Reference to a Skill (e.g. "org/skill-name")
    input_mapping: dict = Field(default_factory=dict)  # Map from step context to skill input
    output_key: str | None = None  # Key to store result in context
    condition: str | None = None  # Guard expression (skip if false)
    on_failure: Literal["abort", "skip", "retry"] = "abort"
    max_retries: int = 0


class Playbook(BaseModel):
    """A named orchestration plan that chains multiple Skills."""

    name: str
    version: str = "1.0.0"
    description: str = ""
    author: str = ""
    steps: list[PlaybookStep] = Field(default_factory=list)
    tags: list[str] = Field(default_factory=list)
    scope: Literal["global", "bu", "dept", "team"] = "team"
