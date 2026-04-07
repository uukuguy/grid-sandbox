"""Agent model — capability manifests for runtime agents.

Aligned with proto eaasp.runtime.v1.CapabilityManifest.
"""

from __future__ import annotations

from typing import Literal

from pydantic import BaseModel, Field


class CostEstimate(BaseModel):
    """Token cost estimate for a runtime."""

    input_cost_per_1k: float = 0.0
    output_cost_per_1k: float = 0.0


class AgentCapability(BaseModel):
    """A single capability flag for an agent/runtime."""

    name: str
    supported: bool = True
    description: str = ""


class CapabilityManifest(BaseModel):
    """Full capability manifest for a runtime agent.

    Aligned with proto CapabilityManifest.
    """

    runtime_id: str
    runtime_name: str = ""
    tier: Literal["harness", "aligned", "framework"] = "harness"
    model: str = ""
    context_window: int = 0
    supported_tools: list[str] = Field(default_factory=list)
    native_hooks: bool = False
    native_mcp: bool = False
    native_skills: bool = False
    cost: CostEstimate = Field(default_factory=CostEstimate)
    metadata: dict[str, str] = Field(default_factory=dict)
    requires_hook_bridge: bool = True
    deployment_mode: Literal["shared", "per_session"] = "shared"
