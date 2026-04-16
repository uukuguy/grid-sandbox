"""CLI entry — gRPC server for nanobot-runtime (W2.T4)."""
from __future__ import annotations

import argparse
import asyncio
import logging
import os

from grpc import aio

from nanobot_runtime._proto.eaasp.runtime.v2 import runtime_pb2_grpc
from nanobot_runtime.service import NanobotRuntimeService


async def _serve(port: int) -> None:
    server = aio.server()
    runtime_pb2_grpc.add_RuntimeServiceServicer_to_server(
        NanobotRuntimeService(), server
    )
    addr = f"0.0.0.0:{port}"
    server.add_insecure_port(addr)
    await server.start()
    logging.info("nanobot-runtime gRPC server listening on %s", addr)
    await server.wait_for_termination()


def main() -> None:
    parser = argparse.ArgumentParser(description="nanobot-runtime gRPC server")
    parser.add_argument(
        "--port",
        type=int,
        default=int(os.environ.get("NANOBOT_RUNTIME_PORT", "50054")),
    )
    parser.add_argument("--log-level", default="INFO")
    args = parser.parse_args()
    logging.basicConfig(level=args.log_level.upper())
    asyncio.run(_serve(args.port))


if __name__ == "__main__":
    main()
