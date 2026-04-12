"""Test `session list` — calls GET /v1/sessions (D41 closed)."""

from __future__ import annotations

import json
from collections.abc import Callable

import httpx
from typer.testing import CliRunner

from eaasp_cli_v2 import main as cli_main


def test_session_list(
    runner: CliRunner,
    install_mock: Callable,
) -> None:
    def handler(request: httpx.Request) -> httpx.Response:
        if request.url.path == "/v1/sessions":
            return httpx.Response(
                200,
                content=json.dumps(
                    {
                        "sessions": [
                            {
                                "session_id": "sess_abc",
                                "status": "active",
                                "skill_id": "skill.demo",
                                "runtime_id": "grid-runtime",
                                "created_at": 1700000000,
                                "closed_at": None,
                            }
                        ]
                    }
                ).encode(),
                headers={"content-type": "application/json"},
            )
        return httpx.Response(404)

    install_mock(handler)
    result = runner.invoke(cli_main.app, ["session", "list"])
    assert result.exit_code == 0, result.stdout + (result.stderr or "")
    assert "sess_abc" in result.stdout
