"""Tool model — tool definitions and MCP server configurations."""

from __future__ import annotations

from typing import Literal

from pydantic import BaseModel, Field


class ToolDef(BaseModel):
    """A tool definition exposed to the agent."""

    name: str
    description: str = ""
    input_schema: dict = Field(default_factory=dict)  # JSON Schema for tool input
    category: str | None = None  # e.g. "file", "network", "code"


class McpServerConfig(BaseModel):
    """Configuration for connecting to an MCP server."""

    name: str
    transport: Literal["stdio", "sse", "streamable-http"]
    command: str | None = None  # For stdio transport
    args: list[str] = Field(default_factory=list)
    url: str | None = None  # For sse/http transport
    env: dict[str, str] = Field(default_factory=dict)
