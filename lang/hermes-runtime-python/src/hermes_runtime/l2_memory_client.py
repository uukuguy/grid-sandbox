"""L2 Memory Engine HTTP client for hermes-runtime.

Mirrors claude-code-runtime's L2 client: fire-and-forget writes of
evidence anchors + searchable memory files to the EAASP L2 Memory Engine.
"""
from __future__ import annotations

import logging
import os
from typing import Any

import httpx

logger = logging.getLogger(__name__)


class L2MemoryClient:
    """Async HTTP client for the EAASP L2 Memory Engine REST API."""

    def __init__(self, base_url: str | None = None, *, client: httpx.AsyncClient | None = None):
        port = os.environ.get("EAASP_L2_PORT", "18085")
        host = os.environ.get("EAASP_L2_HOST", "127.0.0.1")
        self._base_url = (base_url or f"http://{host}:{port}").rstrip("/")
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
        source_system: str = "hermes-runtime",
    ) -> dict[str, Any]:
        args: dict[str, Any] = {
            "event_id": event_id,
            "session_id": session_id,
            "type": anchor_type,
            "source_system": source_system,
        }
        if data_ref is not None:
            args["data_ref"] = data_ref

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
        evidence_refs: list[str] | None = None,
        status: str = "agent_suggested",
    ) -> dict[str, Any]:
        args: dict[str, Any] = {
            "scope": scope,
            "category": category,
            "content": content,
            "status": status,
        }
        if evidence_refs is not None:
            args["evidence_refs"] = evidence_refs

        resp = await self._client.post(
            "/tools/memory_write_file/invoke",
            json={"args": args},
        )
        resp.raise_for_status()
        return resp.json()

    async def close(self) -> None:
        await self._client.aclose()
