"""Sandbox adapter ABC and result models.

Defines the contract all sandbox backends (grid-cli, gRPC runtime, multi-runtime)
must implement, plus shared data models for telemetry and hook events.
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from collections.abc import AsyncIterator
from pathlib import Path

from pydantic import BaseModel, Field

from eaasp.models.message import ResponseChunk, UserMessage
from eaasp.models.session import SessionConfig
from eaasp.models.skill import Skill


class SandboxError(Exception):
    """Raised when a sandbox operation fails."""


class HookFiredEvent(BaseModel):
    """Record of a single hook firing during a session."""

    event: str  # PreToolUse | PostToolUse | Stop
    hook_source: str = ""  # e.g. "check_pii.py"
    decision: str = ""  # allow | block | modify
    tool_name: str | None = None
    latency_ms: float = 0.0


class TelemetrySummary(BaseModel):
    """Aggregated telemetry from a completed sandbox session."""

    session_id: str = ""
    total_turns: int = 0
    tools_called: list[str] = Field(default_factory=list)
    hooks_fired: list[HookFiredEvent] = Field(default_factory=list)
    input_tokens: int = 0
    output_tokens: int = 0
    duration_ms: float = 0.0
    skill_loaded: str = ""
    completed_normally: bool = False


class SandboxAdapter(ABC):
    """Abstract base for sandbox backends.

    Lifecycle: initialize → send (repeated) → terminate.
    """

    @abstractmethod
    async def initialize(
        self, skill: Skill, config: SessionConfig | None = None
    ) -> str:
        """Start a sandbox session. Returns session_id."""

    @abstractmethod
    async def send(self, message: UserMessage) -> AsyncIterator[ResponseChunk]:
        """Send a user message and yield streaming response chunks."""

    @abstractmethod
    async def terminate(self) -> TelemetrySummary:
        """End the session and return telemetry summary."""

    async def validate_skill(self, skill: Skill) -> bool:
        """Validate a skill against the runtime. Default: always True."""
        return True

    @staticmethod
    def load_skill(path: Path) -> Skill:
        """Convenience: load a Skill from a SKILL.md file."""
        return Skill.from_file(path)
