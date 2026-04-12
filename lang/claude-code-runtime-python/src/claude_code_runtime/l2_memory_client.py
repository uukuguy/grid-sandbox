"""L2 Memory Engine HTTP client for claude-code-runtime.

Provides async methods to write evidence anchors and memory files
to the EAASP L2 Memory Engine via its MCP tool REST facade.
"""
from __future__ import annotations

import logging
import os
from typing import Any

import httpx

logger = logging.getLogger(__name__)


class L2MemoryClient:
    """Async HTTP client for the EAASP L2 Memory Engine REST API.

    All methods are fire-and-forget safe: callers should catch exceptions
    and log rather than propagate, since L2 writes must never block the
    agent execution pipeline.
    """

    def __init__(self, base_url: str | None = None, *, client: httpx.AsyncClient | None = None):
        port = os.environ.get("EAASP_L2_PORT", "18085")
        host = os.environ.get("EAASP_L2_HOST", "127.0.0.1")
        self._base_url = (base_url or f"http://{host}:{port}").rstrip("/")
        # trust_env=False: avoid macOS proxy issues (MEMORY.md known issue —
        # Clash proxy turns localhost calls into 502, learned from S4.T2).
        self._client = client or httpx.AsyncClient(
            base_url=self._base_url,
            trust_env=False,
            timeout=10.0,
        )

    async def write_anchor(
        self,
        event_id: str,
        session_id: str,
        anchor_type: str,
        data_ref: str | None = None,
        snapshot_hash: str | None = None,
        source_system: str = "claude-code-runtime",
    ) -> dict[str, Any]:
        """Write an evidence anchor to L2.

        Returns the L2 response dict (contains ``anchor_id``).
        Raises ``httpx.HTTPStatusError`` on non-2xx responses.
        """
        args: dict[str, Any] = {
            "event_id": event_id,
            "session_id": session_id,
            "type": anchor_type,
            "source_system": source_system,
        }
        if data_ref is not None:
            args["data_ref"] = data_ref
        if snapshot_hash is not None:
            args["snapshot_hash"] = snapshot_hash

        resp = await self._client.post(
            "/tools/memory_write_anchor/invoke",
            json={"args": args},
        )
        resp.raise_for_status()
        return resp.json()

    async def write_file(
        self,
        scope: str,
        category: str,
        content: str,
        memory_id: str | None = None,
        evidence_refs: list[str] | None = None,
        status: str = "agent_suggested",
    ) -> dict[str, Any]:
        """Write a memory file to L2.

        Returns the L2 response dict (contains ``memory_id``, ``version``).
        Raises ``httpx.HTTPStatusError`` on non-2xx responses.
        """
        args: dict[str, Any] = {
            "scope": scope,
            "category": category,
            "content": content,
            "status": status,
        }
        if memory_id is not None:
            args["memory_id"] = memory_id
        if evidence_refs is not None:
            args["evidence_refs"] = evidence_refs

        resp = await self._client.post(
            "/tools/memory_write_file/invoke",
            json={"args": args},
        )
        resp.raise_for_status()
        return resp.json()

    async def health(self) -> bool:
        """Check L2 service health."""
        try:
            resp = await self._client.get("/health")
            return resp.status_code == 200
        except Exception:
            return False

    @property
    def base_url(self) -> str:
        return self._base_url

    async def close(self) -> None:
        await self._client.aclose()
