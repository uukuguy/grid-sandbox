"""Message model — user messages and response chunks.

Aligned with proto eaasp.runtime.v1.UserMessage and ResponseChunk.
"""

from __future__ import annotations

from typing import Literal

from pydantic import BaseModel, Field


class UserMessage(BaseModel):
    """A message from the user to the agent."""

    content: str
    message_type: Literal["text", "intent"] = "text"
    metadata: dict[str, str] = Field(default_factory=dict)


class ResponseChunk(BaseModel):
    """A streaming response chunk from the agent.

    Aligned with proto ResponseChunk chunk_type enum.
    """

    chunk_type: Literal[
        "text_delta", "tool_start", "tool_result", "thinking", "done", "error"
    ]
    content: str = ""
    tool_name: str | None = None
    tool_id: str | None = None
    is_error: bool = False
