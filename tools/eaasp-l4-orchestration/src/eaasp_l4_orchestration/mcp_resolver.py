"""L2 MCP Orchestrator client for resolving skill MCP dependencies.

Follows the same client pattern as ``handshake.py`` (L2Client, L3Client):
shared ``httpx.AsyncClient``, normalized error handling, env-configurable
base URL.
"""

from __future__ import annotations

import logging
import os
from typing import Any

import httpx

logger = logging.getLogger(__name__)

MCP_ORCH_URL_DEFAULT = os.environ.get(
    "EAASP_MCP_ORCH_URL", "http://127.0.0.1:18082"
)


class McpResolveError(Exception):
    """Raised when MCP dependency resolution fails."""

    def __init__(self, detail: str = "") -> None:
        self.detail = detail
        super().__init__(detail)


class McpResolver:
    """Queries L2 MCP Orchestrator to resolve skill dependencies into server configs.

    The resolver filters for ``mcp:`` prefixed dependencies, calls the
    orchestrator's ``/v1/mcp/resolve`` endpoint, and returns a list of
    server config dicts compatible with ``L1RuntimeClient.connect_mcp()``.
    """

    def __init__(
        self,
        client: httpx.AsyncClient,
        base_url: str = MCP_ORCH_URL_DEFAULT,
    ) -> None:
        self._client = client
        self._base = base_url.rstrip("/")

    async def resolve(
        self,
        dependencies: list[str],
        runtime_id: str = "",
    ) -> list[dict[str, Any]]:
        """Resolve MCP dependencies to server config dicts.

        Args:
            dependencies: List like ``["mcp:mock-scada", "mcp:eaasp-l2-memory"]``.
                Non-``mcp:`` entries are silently filtered out.
            runtime_id: Target runtime identifier (reserved for future
                transport override, e.g. hermes -> SSE).

        Returns:
            List of server config dicts, each containing at minimum
            ``name`` and ``transport``.  May also include ``command``,
            ``args``, ``url``, ``env`` depending on transport type.

        Raises:
            McpResolveError: On communication or HTTP error from L2 MCP
                Orchestrator.
        """
        mcp_deps = [d for d in dependencies if d.startswith("mcp:")]
        if not mcp_deps:
            return []

        try:
            resp = await self._client.post(
                f"{self._base}/v1/mcp/resolve",
                json={"dependencies": mcp_deps, "runtime_id": runtime_id},
                timeout=10.0,
            )
            resp.raise_for_status()
            data = resp.json()
            servers = data.get("servers", [])
            logger.info(
                "MCP resolve: %d deps -> %d servers (runtime=%s)",
                len(mcp_deps),
                len(servers),
                runtime_id,
            )
            return servers
        except httpx.ConnectError as exc:
            logger.warning("MCP Orchestrator unreachable: %s", exc)
            raise McpResolveError(
                f"L2 MCP Orchestrator unreachable: {exc}"
            ) from exc
        except httpx.HTTPStatusError as exc:
            logger.warning("MCP resolve failed: %s", exc)
            raise McpResolveError(f"L2 MCP resolve error: {exc}") from exc
