"""Shared pytest fixtures — temp SQLite DB per test."""

from __future__ import annotations

import os
import tempfile
from collections.abc import AsyncIterator

import pytest_asyncio

from eaasp_l2_memory_engine.anchors import AnchorStore
from eaasp_l2_memory_engine.api import create_app
from eaasp_l2_memory_engine.db import init_db
from eaasp_l2_memory_engine.files import MemoryFileStore
from eaasp_l2_memory_engine.index import HybridIndex
from eaasp_l2_memory_engine.mcp_tools import McpToolDispatcher


@pytest_asyncio.fixture
async def db_path() -> AsyncIterator[str]:
    with tempfile.NamedTemporaryFile(suffix=".db", delete=False) as f:
        path = f.name
    await init_db(path)
    try:
        yield path
    finally:
        if os.path.exists(path):
            os.unlink(path)


@pytest_asyncio.fixture
async def anchor_store(db_path: str) -> AnchorStore:
    return AnchorStore(db_path)


@pytest_asyncio.fixture
async def file_store(db_path: str) -> MemoryFileStore:
    return MemoryFileStore(db_path)


@pytest_asyncio.fixture
async def index(db_path: str) -> HybridIndex:
    return HybridIndex(db_path)


@pytest_asyncio.fixture
async def dispatcher(db_path: str) -> McpToolDispatcher:
    return McpToolDispatcher(
        AnchorStore(db_path),
        MemoryFileStore(db_path),
        HybridIndex(db_path),
    )


@pytest_asyncio.fixture
async def app(db_path: str) -> AsyncIterator:
    from httpx import ASGITransport, AsyncClient

    application = create_app(db_path)
    transport = ASGITransport(app=application)
    async with AsyncClient(transport=transport, base_url="http://test") as client:
        yield client
