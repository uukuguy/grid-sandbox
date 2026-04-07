"""GridCliSandbox — local sandbox via grid CLI subprocess.

Runs the `grid` binary in a subprocess, passing Skill content and session config
through temporary files, and parses stdout for response chunks and telemetry.
"""

from __future__ import annotations

import asyncio
import json
import shutil
import tempfile
from collections.abc import AsyncIterator
from pathlib import Path

from eaasp.models.message import ResponseChunk, UserMessage
from eaasp.models.session import SessionConfig
from eaasp.models.skill import Skill
from eaasp.sandbox.base import (
    HookFiredEvent,
    SandboxAdapter,
    SandboxError,
    TelemetrySummary,
)


class GridCliSandbox(SandboxAdapter):
    """Sandbox adapter that delegates to the local `grid` CLI binary.

    Usage::

        sandbox = GridCliSandbox(grid_bin="grid")
        session_id = await sandbox.initialize(skill, config)
        async for chunk in sandbox.send(UserMessage(content="hello")):
            print(chunk.content)
        summary = await sandbox.terminate()
    """

    def __init__(self, grid_bin: str = "grid", timeout: float = 120.0) -> None:
        self._grid_bin = grid_bin
        self._timeout = timeout
        self._process: asyncio.subprocess.Process | None = None
        self._tmpdir: tempfile.TemporaryDirectory | None = None
        self._session_id: str = ""
        self._skill_name: str = ""
        self._chunks_collected: list[ResponseChunk] = []

    def _check_binary(self) -> str:
        """Verify the grid binary is available. Returns resolved path."""
        resolved = shutil.which(self._grid_bin)
        if resolved is None:
            raise SandboxError(
                f"Grid binary '{self._grid_bin}' not found in PATH. "
                "Install with `make build-cli` or set grid_bin to the full path."
            )
        return resolved

    async def initialize(
        self, skill: Skill, config: SessionConfig | None = None
    ) -> str:
        """Write skill + config to temp files and start the grid subprocess."""
        binary = self._check_binary()

        self._tmpdir = tempfile.TemporaryDirectory(prefix="eaasp-sandbox-")
        tmpdir = Path(self._tmpdir.name)

        # Write SKILL.md
        skill_path = tmpdir / "SKILL.md"
        skill_path.write_text(skill.to_skill_md(), encoding="utf-8")
        self._skill_name = skill.frontmatter.name

        # Write session config as JSON
        config_path = tmpdir / "session_config.json"
        cfg = config or SessionConfig()
        config_path.write_text(cfg.model_dump_json(indent=2), encoding="utf-8")

        # Build command — uses create_subprocess_exec (no shell, safe from injection)
        cmd = [
            binary,
            "session",
            "create",
            "--skill",
            str(skill_path),
            "--config",
            str(config_path),
            "--json-output",
        ]

        try:
            self._process = await asyncio.create_subprocess_exec(
                *cmd,
                stdin=asyncio.subprocess.PIPE,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
        except OSError as e:
            raise SandboxError(f"Failed to start grid process: {e}") from e

        # Read initial output for session_id
        self._session_id = f"sandbox-{id(self._process)}"
        return self._session_id

    async def send(self, message: UserMessage) -> AsyncIterator[ResponseChunk]:
        """Send a message to the grid subprocess and yield response chunks."""
        if self._process is None or self._process.stdin is None:
            raise SandboxError("Sandbox not initialized. Call initialize() first.")

        # Write message as JSON line to stdin
        msg_json = message.model_dump_json() + "\n"
        self._process.stdin.write(msg_json.encode("utf-8"))
        await self._process.stdin.drain()

        # Read response lines from stdout
        assert self._process.stdout is not None
        while True:
            try:
                line = await asyncio.wait_for(
                    self._process.stdout.readline(), timeout=self._timeout
                )
            except TimeoutError:
                raise SandboxError(
                    f"Timeout waiting for grid response after {self._timeout}s"
                )

            if not line:
                break

            text = line.decode("utf-8").strip()
            if not text:
                continue

            chunk = self._parse_output_line(text)
            if chunk is not None:
                self._chunks_collected.append(chunk)
                yield chunk
                if chunk.chunk_type == "done":
                    break

    async def terminate(self) -> TelemetrySummary:
        """Terminate the subprocess and return telemetry summary."""
        if self._process is not None:
            if self._process.stdin is not None:
                self._process.stdin.close()
            try:
                await asyncio.wait_for(self._process.wait(), timeout=10.0)
            except TimeoutError:
                self._process.kill()
                await self._process.wait()

        summary = self._build_telemetry()

        # Cleanup temp dir
        if self._tmpdir is not None:
            self._tmpdir.cleanup()
            self._tmpdir = None

        self._process = None
        return summary

    def _parse_output_line(self, line: str) -> ResponseChunk | None:
        """Parse a JSON output line into a ResponseChunk."""
        try:
            data = json.loads(line)
        except json.JSONDecodeError:
            # Non-JSON line — treat as text delta
            return ResponseChunk(chunk_type="text_delta", content=line)

        chunk_type = data.get("type", data.get("chunk_type", "text_delta"))
        return ResponseChunk(
            chunk_type=chunk_type,
            content=data.get("content", data.get("text", "")),
            tool_name=data.get("tool_name"),
            tool_id=data.get("tool_id"),
            is_error=data.get("is_error", False),
        )

    def _build_telemetry(self) -> TelemetrySummary:
        """Build telemetry summary from collected chunks."""
        tools = []
        hooks: list[HookFiredEvent] = []
        completed = False

        for chunk in self._chunks_collected:
            if chunk.chunk_type == "tool_start" and chunk.tool_name:
                tools.append(chunk.tool_name)
            elif chunk.chunk_type == "done":
                completed = True

        return TelemetrySummary(
            session_id=self._session_id,
            total_turns=len(
                [c for c in self._chunks_collected if c.chunk_type == "text_delta"]
            ),
            tools_called=tools,
            hooks_fired=hooks,
            skill_loaded=self._skill_name,
            completed_normally=completed,
        )

    async def validate_skill(self, skill: Skill) -> bool:
        """Validate skill by calling grid with --validate flag."""
        try:
            binary = self._check_binary()
        except SandboxError:
            return False

        tmpdir = tempfile.TemporaryDirectory(prefix="eaasp-validate-")
        try:
            skill_path = Path(tmpdir.name) / "SKILL.md"
            skill_path.write_text(skill.to_skill_md(), encoding="utf-8")

            proc = await asyncio.create_subprocess_exec(
                binary,
                "eval",
                "config",
                "--validate",
                str(skill_path),
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
            _, stderr = await asyncio.wait_for(proc.communicate(), timeout=30.0)
            return proc.returncode == 0
        except (OSError, TimeoutError):
            return False
        finally:
            tmpdir.cleanup()
