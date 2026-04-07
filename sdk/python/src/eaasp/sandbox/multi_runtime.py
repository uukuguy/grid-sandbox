"""MultiRuntimeSandbox — cross-runtime comparison testing.

Runs a Skill against multiple L1 runtimes in parallel and produces a
ConsistencyReport comparing tool calls, hook firings, and completion status.

Inspired by the BF certifier blindbox comparison pattern.
"""

from __future__ import annotations

import asyncio
import logging

from pydantic import BaseModel, Field

from eaasp.models.message import ResponseChunk, UserMessage
from eaasp.models.session import SessionConfig
from eaasp.models.skill import Skill
from eaasp.sandbox.base import SandboxError, TelemetrySummary
from eaasp.sandbox.runtime import RuntimeSandbox

logger = logging.getLogger(__name__)


class ConsistencyReport(BaseModel):
    """Result of comparing multiple runtime executions.

    Attributes:
        all_completed: True if every runtime completed normally.
        tools_diff: Symmetric difference of tool call sets across runtimes.
        hooks_diff: Hook event differences (event names not matching).
        output_similarity: 1.0 = identical tool sets, degrades by diff count.
    """

    all_completed: bool = False
    tools_diff: list[str] = Field(default_factory=list)
    hooks_diff: list[str] = Field(default_factory=list)
    output_similarity: float = 0.0


class ComparisonResult(BaseModel):
    """Aggregated results from all runtimes plus consistency analysis.

    Attributes:
        results: Mapping of endpoint → TelemetrySummary.
        consistency: Cross-runtime consistency analysis.
    """

    results: dict[str, TelemetrySummary] = Field(default_factory=dict)
    consistency: ConsistencyReport = Field(default_factory=ConsistencyReport)


class MultiRuntimeSandbox:
    """Run a Skill against multiple runtimes and compare results.

    Usage::

        multi = MultiRuntimeSandbox([
            "grpc://localhost:50051",
            "grpc://localhost:50052",
        ])
        result = await multi.compare(config, skill, message)
        print(result.consistency.all_completed)
        print(result.consistency.output_similarity)
    """

    def __init__(
        self, endpoints: list[str], timeout: float = 120.0
    ) -> None:
        if len(endpoints) < 2:
            raise SandboxError(
                "MultiRuntimeSandbox requires at least 2 endpoints for comparison."
            )
        self._endpoints = endpoints
        self._timeout = timeout

    async def compare(
        self,
        config: SessionConfig,
        skill: Skill,
        message: UserMessage,
    ) -> ComparisonResult:
        """Execute skill on all runtimes in parallel and compare results.

        Steps:
        1. Create RuntimeSandbox for each endpoint
        2. asyncio.gather: initialize → send → terminate
        3. Compute ConsistencyReport
        """
        tasks = [
            self._run_single(endpoint, config, skill, message)
            for endpoint in self._endpoints
        ]

        # Gather with return_exceptions to handle partial failures
        raw_results = await asyncio.gather(*tasks, return_exceptions=True)

        results: dict[str, TelemetrySummary] = {}
        for endpoint, result in zip(self._endpoints, raw_results):
            if isinstance(result, Exception):
                logger.warning("Runtime %s failed: %s", endpoint, result)
                results[endpoint] = TelemetrySummary(
                    session_id=f"error-{endpoint}",
                    completed_normally=False,
                )
            else:
                results[endpoint] = result

        consistency = self._compute_consistency(results)

        return ComparisonResult(results=results, consistency=consistency)

    async def _run_single(
        self,
        endpoint: str,
        config: SessionConfig,
        skill: Skill,
        message: UserMessage,
    ) -> TelemetrySummary:
        """Run the full lifecycle on a single runtime."""
        sandbox = RuntimeSandbox(endpoint, timeout=self._timeout)

        try:
            await sandbox.initialize(skill, config)

            # Consume all chunks (we only care about telemetry for comparison)
            async for _chunk in sandbox.send(message):
                pass

            return await sandbox.terminate()
        except Exception as e:
            # Try to clean up
            try:
                await sandbox.terminate()
            except Exception:
                pass
            raise SandboxError(
                f"Runtime {endpoint} execution failed: {e}"
            ) from e

    @staticmethod
    def _compute_consistency(
        summaries: dict[str, TelemetrySummary],
    ) -> ConsistencyReport:
        """Compute consistency across runtime summaries.

        - all_completed: every runtime completed_normally
        - tools_diff: symmetric difference of tool sets
        - hooks_diff: differences in hook event lists
        - output_similarity: 1.0 if identical tool sets, degraded by diffs
        """
        if not summaries:
            return ConsistencyReport()

        all_completed = all(s.completed_normally for s in summaries.values())

        # Collect tool sets per runtime
        tool_sets = [set(s.tools_called) for s in summaries.values()]

        # Symmetric difference: tools in any but not all
        if tool_sets:
            union_all = set().union(*tool_sets)
            intersection_all = set.intersection(*tool_sets) if tool_sets else set()
            tools_diff = sorted(union_all - intersection_all)
        else:
            tools_diff = []

        # Hook event comparison
        hook_event_sets = [
            set(h.event for h in s.hooks_fired) for s in summaries.values()
        ]
        if hook_event_sets:
            hook_union = set().union(*hook_event_sets)
            hook_intersection = (
                set.intersection(*hook_event_sets) if hook_event_sets else set()
            )
            hooks_diff = sorted(hook_union - hook_intersection)
        else:
            hooks_diff = []

        # Similarity score (Jaccard index of tool sets)
        if tool_sets:
            union_size = len(union_all)
            if union_size == 0:
                # No tools called by any runtime → trivially consistent
                output_similarity = 1.0
            else:
                intersection_size = len(intersection_all)
                output_similarity = intersection_size / union_size
        else:
            output_similarity = 1.0

        return ConsistencyReport(
            all_completed=all_completed,
            tools_diff=tools_diff,
            hooks_diff=hooks_diff,
            output_similarity=output_similarity,
        )
