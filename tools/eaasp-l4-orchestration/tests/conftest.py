"""Shared pytest fixtures for L4 orchestration tests.

Strategy:
- Each test gets a fresh tmp SQLite db via ``tmp_db_path``.
- ``seed_session`` seeds a bare-bones ``sessions`` row without going through
  the full orchestrator (used by event_stream / API tests that don't want to
  mock L2 + L3).
- ``l4_http_client`` is a shared ``httpx.AsyncClient`` that respx can
  intercept. It is injected into ``create_app`` so L2/L3 calls from inside
  the ASGI app also flow through respx.
- ``app_client`` wraps the FastAPI app in an ASGI transport AsyncClient for
  in-process HTTP calls. A mock L1 factory is injected so tests don't need
  a real gRPC runtime.
"""

from __future__ import annotations

import os
import tempfile
import time
from collections.abc import AsyncIterator
from typing import Any

import httpx
import pytest_asyncio

from eaasp_l4_orchestration.api import create_app
from eaasp_l4_orchestration.db import connect, init_db


class _StubL1Client:
    """Stub L1 client for tests that don't care about real gRPC."""

    def __init__(self, runtime_id: str) -> None:
        self.runtime_id = runtime_id

    async def initialize(self, payload_dict: dict[str, Any]) -> dict[str, str]:
        sid = payload_dict.get("session_id", "mock")
        return {"session_id": sid, "runtime_id": self.runtime_id}

    async def send(self, session_id: str, content: str, message_type: str = "text"):
        yield {"chunk_type": "text_delta", "content": content}
        yield {"chunk_type": "done", "content": ""}

    async def terminate(self) -> None:
        pass

    async def close(self) -> None:
        pass


def _stub_l1_factory(runtime_id: str) -> _StubL1Client:
    return _StubL1Client(runtime_id)


@pytest_asyncio.fixture
async def tmp_db_path() -> AsyncIterator[str]:
    with tempfile.NamedTemporaryFile(suffix=".db", delete=False) as f:
        path = f.name
    await init_db(path)
    try:
        yield path
    finally:
        for suffix in ("", "-wal", "-shm"):
            p = path + suffix
            if os.path.exists(p):
                try:
                    os.unlink(p)
                except OSError:
                    pass


@pytest_asyncio.fixture
async def seed_session(tmp_db_path: str):
    """Factory fixture — inserts a minimal ``sessions`` row for tests that
    need a valid session_id without exercising the handshake."""
    created: list[str] = []

    async def _seed(session_id: str = "sess_test000001") -> str:
        db = await connect(tmp_db_path)
        try:
            await db.execute("BEGIN IMMEDIATE")
            await db.execute(
                """
                INSERT INTO sessions
                    (session_id, intent_id, skill_id, runtime_id, user_id,
                     status, payload_json, created_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                """,
                (
                    session_id,
                    None,
                    "skill.test",
                    "hermes-l1",
                    "user-test",
                    "created",
                    "{}",
                    int(time.time()),
                ),
            )
            await db.commit()
        finally:
            await db.close()
        created.append(session_id)
        return session_id

    yield _seed


@pytest_asyncio.fixture
async def l4_http_client() -> AsyncIterator[httpx.AsyncClient]:
    """An httpx AsyncClient that respx can intercept (for L2/L3 calls)."""
    async with httpx.AsyncClient(timeout=5.0) as client:
        yield client


@pytest_asyncio.fixture
async def app_client(
    tmp_db_path: str,
    l4_http_client: httpx.AsyncClient,
) -> AsyncIterator[httpx.AsyncClient]:
    """In-process ASGI client for the L4 FastAPI app.

    ``httpx.ASGITransport`` does not emit lifespan events, so we enter the
    FastAPI router lifespan context manually to hydrate ``app.state``.
    A stub L1 factory is injected so tests don't need a real gRPC runtime.
    """
    application = create_app(
        tmp_db_path,
        http_client=l4_http_client,
        l1_factory=_stub_l1_factory,
    )
    async with application.router.lifespan_context(application):
        transport = httpx.ASGITransport(app=application)
        async with httpx.AsyncClient(
            transport=transport, base_url="http://testserver"
        ) as client:
            yield client
