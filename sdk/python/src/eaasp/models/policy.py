"""Policy model — enterprise governance rules for Skill execution."""

from __future__ import annotations

from typing import Literal

from pydantic import BaseModel, Field


class PolicyRule(BaseModel):
    """A single policy rule."""

    name: str
    description: str = ""
    condition: str  # Expression evaluated at runtime (e.g. "tool_name == 'bash'")
    action: Literal["allow", "deny", "audit", "modify"] = "deny"
    priority: int = 100  # Lower number = higher priority
    scope: Literal["global", "bu", "dept", "team"] = "global"


class Policy(BaseModel):
    """A named set of policy rules."""

    name: str
    version: str = "1.0.0"
    description: str = ""
    author: str = ""
    rules: list[PolicyRule] = Field(default_factory=list)
    tags: list[str] = Field(default_factory=list)
