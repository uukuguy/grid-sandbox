"""Entry point — uvicorn + FastAPI."""

from __future__ import annotations

import os

import uvicorn

from .api import create_app

DEFAULT_DB_PATH = os.environ.get("EAASP_MEMORY_DB", "./data/memory.db")
DEFAULT_PORT = int(os.environ.get("EAASP_MEMORY_PORT", "8085"))
DEFAULT_HOST = os.environ.get("EAASP_MEMORY_HOST", "127.0.0.1")

app = create_app(DEFAULT_DB_PATH)


def run() -> None:
    uvicorn.run(
        "eaasp_l2_memory_engine.main:app",
        host=DEFAULT_HOST,
        port=DEFAULT_PORT,
        reload=False,
    )


if __name__ == "__main__":
    run()
