"""MCP tool dispatcher tests — 6 tools happy path + error codes."""

from __future__ import annotations

import pytest

from eaasp_l2_memory_engine.mcp_tools import McpToolDispatcher, ToolError


pytestmark = pytest.mark.asyncio


async def test_write_anchor_tool(dispatcher: McpToolDispatcher) -> None:
    out = await dispatcher.invoke(
        "memory_write_anchor",
        {
            "event_id": "evt1",
            "session_id": "sess1",
            "type": "tool_result",
            "metadata": {"k": "v"},
        },
    )
    assert out["anchor_id"].startswith("anc_")


async def test_write_file_and_read(dispatcher: McpToolDispatcher) -> None:
    written = await dispatcher.invoke(
        "memory_write_file",
        {
            "scope": "user:alice",
            "category": "threshold",
            "content": "salary_floor=50000",
        },
    )
    memory_id = written["memory_id"]

    read = await dispatcher.invoke("memory_read", {"memory_id": memory_id})
    assert read["memory_id"] == memory_id
    assert read["content"] == "salary_floor=50000"


async def test_search_tool(dispatcher: McpToolDispatcher) -> None:
    await dispatcher.invoke(
        "memory_write_file",
        {
            "scope": "s",
            "category": "c",
            "content": "the quick brown fox jumps",
        },
    )
    out = await dispatcher.invoke("memory_search", {"query": "brown"})
    assert len(out["hits"]) == 1


async def test_list_tool(dispatcher: McpToolDispatcher) -> None:
    for i in range(3):
        await dispatcher.invoke(
            "memory_write_file",
            {"scope": "s", "category": "c", "content": f"entry {i}"},
        )
    out = await dispatcher.invoke("memory_list", {"scope": "s"})
    assert len(out["memories"]) == 3


async def test_archive_tool(dispatcher: McpToolDispatcher) -> None:
    written = await dispatcher.invoke(
        "memory_write_file",
        {"scope": "s", "category": "c", "content": "x"},
    )
    out = await dispatcher.invoke(
        "memory_archive", {"memory_id": written["memory_id"]}
    )
    assert out["status"] == "archived"


async def test_unknown_tool_raises(dispatcher: McpToolDispatcher) -> None:
    with pytest.raises(ToolError) as exc:
        await dispatcher.invoke("memory_nonexistent", {})
    assert exc.value.code == "unknown_tool"


async def test_missing_arg_raises(dispatcher: McpToolDispatcher) -> None:
    with pytest.raises(ToolError) as exc:
        await dispatcher.invoke("memory_read", {})
    assert exc.value.code == "missing_arg"


async def test_read_not_found_raises(dispatcher: McpToolDispatcher) -> None:
    with pytest.raises(ToolError) as exc:
        await dispatcher.invoke("memory_read", {"memory_id": "mem_missing"})
    assert exc.value.code == "not_found"
