"""Mock SCADA MCP stdio server.

Exposes two tools to any MCP-capable runtime (grid-runtime, claude-code-runtime):

- `scada_read_snapshot(device_id, time_window="5m")` — returns deterministic
  telemetry (3 samples + baseline).
- `scada_write(device_id, field, value)` — always fails with a marker error.
  The threshold-calibration skill's PreToolUse hook blocks it before we even
  get here; this is belt-and-suspenders for the e2e test.

Run via stdio transport (the `mcp-scada` console script):

    uv run mock-scada

The runtime's MCP client spawns this as a subprocess and exchanges
newline-delimited JSON-RPC over stdin/stdout.
"""

from __future__ import annotations

import asyncio
import json
from typing import Any

from mcp.server import NotificationOptions, Server
from mcp.server.models import InitializationOptions
from mcp.server.stdio import stdio_server
from mcp.types import TextContent, Tool

from .snapshots import SCADA_WRITE_ERROR_MARKER, build_snapshot, snapshot_hash

import logging
import sys

logging.basicConfig(
    level=logging.INFO,
    format="[mock-scada] %(message)s",
    stream=sys.stderr,
)
_log = logging.getLogger("mock-scada")

SERVER_NAME = "mock-scada"
SERVER_VERSION = "0.1.0"

_TOOL_MANIFEST: list[Tool] = [
    Tool(
        name="scada_read_snapshot",
        description=(
            "Read the latest SCADA telemetry snapshot for a device. "
            "Returns deterministic temperature/load/dissolved-gas samples "
            "suitable for threshold calibration (read-only, safe to call)."
        ),
        inputSchema={
            "type": "object",
            "properties": {
                "device_id": {
                    "type": "string",
                    "description": "Device identifier (e.g. xfmr-042, brk-17).",
                },
                "time_window": {
                    "type": "string",
                    "description": "Lookback window, e.g. '5m', '1h'. Defaults to '5m'.",
                    "default": "5m",
                },
            },
            "required": ["device_id"],
        },
    ),
    Tool(
        name="scada_write",
        description=(
            "MUST NOT be called. Any attempt to write to SCADA from an agent "
            "is blocked by the threshold-calibration skill's PreToolUse hook. "
            "This endpoint exists only so hook denial can be exercised end-to-end."
        ),
        inputSchema={
            "type": "object",
            "properties": {
                "device_id": {"type": "string"},
                "field": {"type": "string"},
                "value": {},
            },
            "required": ["device_id", "field", "value"],
        },
    ),
]


def _handle_scada_read_snapshot(args: dict[str, Any]) -> dict[str, Any]:
    device_id = args.get("device_id")
    if not isinstance(device_id, str) or not device_id:
        raise ValueError("device_id (non-empty string) is required")
    time_window = args.get("time_window", "5m")
    if not isinstance(time_window, str) or not time_window:
        time_window = "5m"
    _log.info("scada_read_snapshot device_id=%s time_window=%s", device_id, time_window)
    snapshot = build_snapshot(device_id, time_window)
    snapshot["snapshot_hash"] = snapshot_hash(snapshot)
    return snapshot


def _handle_scada_write(args: dict[str, Any]) -> dict[str, Any]:
    _log.warning("scada_write BLOCKED device_id=%s", args.get("device_id", "?"))
    raise RuntimeError(
        f"{SCADA_WRITE_ERROR_MARKER}: scada_write is blocked; "
        f"args={json.dumps(args, sort_keys=True)}"
    )


def build_server() -> Server:
    """Build the MCP server with tool handlers wired in."""
    server: Server = Server(SERVER_NAME)

    @server.list_tools()
    async def list_tools() -> list[Tool]:
        return list(_TOOL_MANIFEST)

    @server.call_tool()
    async def call_tool(name: str, arguments: dict[str, Any]) -> list[TextContent]:
        _log.info("call_tool: %s args=%s", name, arguments)
        if name == "scada_read_snapshot":
            result = _handle_scada_read_snapshot(arguments or {})
        elif name == "scada_write":
            result = _handle_scada_write(arguments or {})
        else:
            raise ValueError(f"unknown tool: {name}")
        return [
            TextContent(
                type="text",
                text=json.dumps(result, sort_keys=True, separators=(",", ":")),
            )
        ]

    return server


async def _serve_stdio() -> None:
    _log.info("Starting mock-scada MCP server v%s (stdio transport)", SERVER_VERSION)
    server = build_server()
    init_options = InitializationOptions(
        server_name=SERVER_NAME,
        server_version=SERVER_VERSION,
        capabilities=server.get_capabilities(
            notification_options=NotificationOptions(),
            experimental_capabilities={},
        ),
    )
    async with stdio_server() as (read_stream, write_stream):
        await server.run(read_stream, write_stream, init_options)


def _serve_sse(host: str, port: int) -> None:
    """Run mock-scada as SSE transport (network MCP for container-to-container)."""
    from starlette.applications import Starlette
    from starlette.responses import Response
    from starlette.routing import Mount, Route
    from mcp.server.sse import SseServerTransport

    _log.info(
        "Starting mock-scada MCP server v%s (SSE transport on %s:%d)",
        SERVER_VERSION, host, port,
    )

    sse = SseServerTransport("/messages/")
    server = build_server()
    init_options = InitializationOptions(
        server_name=SERVER_NAME,
        server_version=SERVER_VERSION,
        capabilities=server.get_capabilities(
            notification_options=NotificationOptions(),
            experimental_capabilities={},
        ),
    )

    async def handle_sse(request):
        async with sse.connect_sse(
            request.scope, request.receive, request._send,
        ) as streams:
            await server.run(streams[0], streams[1], init_options)
        return Response()

    starlette_routes = [
        Route("/sse", endpoint=handle_sse, methods=["GET"]),
        Mount("/messages/", app=sse.handle_post_message),
    ]
    app = Starlette(routes=starlette_routes)

    import uvicorn
    uvicorn.run(app, host=host, port=port, log_level="info")


def run() -> None:
    """Console-script entry point.

    Usage:
        mock-scada                         # stdio (default)
        mock-scada --transport sse         # SSE on 0.0.0.0:8090
        mock-scada --transport sse --port 9090 --host 127.0.0.1
    """
    import argparse

    parser = argparse.ArgumentParser(description="Mock SCADA MCP server")
    parser.add_argument(
        "--transport", choices=["stdio", "sse"], default="stdio",
        help="MCP transport mode (default: stdio)",
    )
    parser.add_argument("--host", default="0.0.0.0", help="SSE bind host")
    parser.add_argument("--port", type=int, default=18090, help="SSE bind port")
    args = parser.parse_args()

    if args.transport == "sse":
        _serve_sse(args.host, args.port)
    else:
        asyncio.run(_serve_stdio())


if __name__ == "__main__":
    run()
