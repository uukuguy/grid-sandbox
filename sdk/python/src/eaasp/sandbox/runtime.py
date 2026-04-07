"""RuntimeSandbox — gRPC direct connection to L1 Runtime.

Connects to a running grid-runtime (or claude-code-runtime) via gRPC,
using the EAASP Runtime Service protocol (proto eaasp.runtime.v1).

Requires optional dependency: ``pip install eaasp-sdk[grpc]``
"""

from __future__ import annotations

import asyncio
import logging
from collections.abc import AsyncIterator
from urllib.parse import urlparse

import yaml

from eaasp.models.message import ResponseChunk, UserMessage
from eaasp.models.session import SessionConfig
from eaasp.models.skill import Skill
from eaasp.sandbox.base import (
    HookFiredEvent,
    SandboxAdapter,
    SandboxError,
    TelemetrySummary,
)

logger = logging.getLogger(__name__)


def _parse_endpoint(endpoint: str) -> tuple[str, int]:
    """Parse ``grpc://host:port`` into (host, port).

    Also accepts plain ``host:port`` without scheme.
    """
    if "://" in endpoint:
        parsed = urlparse(endpoint)
        host = parsed.hostname or "localhost"
        port = parsed.port or 50051
    else:
        parts = endpoint.rsplit(":", 1)
        host = parts[0] if parts[0] else "localhost"
        port = int(parts[1]) if len(parts) > 1 else 50051
    return host, port


def _build_session_payload(
    config: SessionConfig, skill: Skill
) -> dict:
    """Build the gRPC InitializeRequest payload dict.

    Returns a dict matching proto SessionPayload fields, suitable for
    constructing the protobuf message.
    """
    return {
        "user_id": config.user_id,
        "user_role": config.user_role,
        "org_unit": config.org_unit,
        "managed_hooks_json": config.managed_hooks_json or "",
        "quotas": dict(config.quotas),
        "context": {
            **dict(config.context),
            "skill_name": skill.frontmatter.name,
            "skill_type": skill.frontmatter.skill_type,
        },
        "hook_bridge_url": config.hook_bridge_url or "",
        "telemetry_endpoint": config.telemetry_endpoint or "",
    }


def _build_skill_content(skill: Skill) -> dict:
    """Build the gRPC SkillContent dict from a Skill model."""
    return {
        "skill_id": skill.frontmatter.name,
        "name": skill.frontmatter.name,
        "frontmatter_yaml": yaml.dump(
            skill.frontmatter.model_dump(mode="json"), default_flow_style=False
        ),
        "prose": skill.prose,
    }


def _build_send_request(session_id: str, message: UserMessage) -> dict:
    """Build the gRPC SendRequest dict."""
    return {
        "session_id": session_id,
        "message": {
            "content": message.content,
            "message_type": message.message_type,
            "metadata": dict(message.metadata),
        },
    }


def _proto_chunk_to_response(proto_chunk) -> ResponseChunk:
    """Convert a proto ResponseChunk to SDK ResponseChunk.

    Works with both real proto objects and dict-like mocks.
    """
    if hasattr(proto_chunk, "chunk_type"):
        chunk_type = proto_chunk.chunk_type
        content = proto_chunk.content
        tool_name = proto_chunk.tool_name or None
        tool_id = proto_chunk.tool_id or None
        is_error = proto_chunk.is_error
    else:
        chunk_type = proto_chunk.get("chunk_type", "text_delta")
        content = proto_chunk.get("content", "")
        tool_name = proto_chunk.get("tool_name") or None
        tool_id = proto_chunk.get("tool_id") or None
        is_error = proto_chunk.get("is_error", False)

    # Normalize chunk_type to SDK enum values
    valid_types = {"text_delta", "tool_start", "tool_result", "thinking", "done", "error"}
    if chunk_type not in valid_types:
        chunk_type = "text_delta"

    return ResponseChunk(
        chunk_type=chunk_type,
        content=content,
        tool_name=tool_name,
        tool_id=tool_id,
        is_error=is_error,
    )


class RuntimeSandbox(SandboxAdapter):
    """Sandbox adapter connecting to a gRPC L1 Runtime.

    Usage::

        sandbox = RuntimeSandbox("grpc://localhost:50051")
        session_id = await sandbox.initialize(skill, config)
        async for chunk in sandbox.send(UserMessage(content="hello")):
            print(chunk.content)
        summary = await sandbox.terminate()

    Requires ``grpcio`` to be installed.
    """

    def __init__(self, endpoint: str, timeout: float = 120.0) -> None:
        self._endpoint = endpoint
        self._timeout = timeout
        self._host, self._port = _parse_endpoint(endpoint)
        self._channel = None
        self._stub = None
        self._session_id: str = ""
        self._skill_name: str = ""
        self._chunks_collected: list[ResponseChunk] = []
        self._hooks_collected: list[HookFiredEvent] = []

    def _ensure_grpc(self):
        """Lazily import grpc and proto stubs. Raises SandboxError if unavailable."""
        try:
            import grpc  # noqa: F811

            if grpc is None:
                raise ImportError("grpc module is None")
        except (ImportError, ModuleNotFoundError):
            raise SandboxError(
                "grpcio is required for RuntimeSandbox. "
                "Install with: pip install eaasp-sdk[grpc]"
            )
        return grpc

    def _create_channel(self):
        """Create an insecure gRPC channel."""
        grpc = self._ensure_grpc()
        target = f"{self._host}:{self._port}"
        self._channel = grpc.insecure_channel(target)
        return self._channel

    def _get_stub(self):
        """Get or create the RuntimeService stub.

        Uses dynamic proto loading to avoid hard dependency on generated stubs.
        Falls back to a lightweight wrapper if stubs not available.
        """
        if self._stub is not None:
            return self._stub

        channel = self._create_channel()

        # Try importing the generated stubs
        try:
            from claude_code_runtime._proto.eaasp.runtime.v1 import (
                runtime_pb2_grpc,
            )

            self._stub = runtime_pb2_grpc.RuntimeServiceStub(channel)
        except ImportError:
            # Stubs not available — use a lightweight gRPC wrapper
            self._stub = _LightweightRuntimeStub(channel)

        return self._stub

    async def initialize(
        self, skill: Skill, config: SessionConfig | None = None
    ) -> str:
        """Connect to gRPC runtime and initialize a session."""
        cfg = config or SessionConfig()
        self._skill_name = skill.frontmatter.name

        payload = _build_session_payload(cfg, skill)
        skill_content = _build_skill_content(skill)

        try:
            stub = self._get_stub()

            # Run gRPC calls in executor (grpcio is sync by default)
            loop = asyncio.get_event_loop()

            # Initialize session
            init_response = await loop.run_in_executor(
                None, lambda: stub.Initialize(_make_init_request(payload))
            )
            self._session_id = init_response.session_id

            # Load skill
            await loop.run_in_executor(
                None,
                lambda: stub.LoadSkill(
                    _make_load_skill_request(self._session_id, skill_content)
                ),
            )

        except Exception as e:
            raise SandboxError(f"gRPC initialization failed: {e}") from e

        return self._session_id

    async def send(self, message: UserMessage) -> AsyncIterator[ResponseChunk]:
        """Send a message and yield response chunks from gRPC stream."""
        if not self._session_id:
            raise SandboxError("Sandbox not initialized. Call initialize() first.")

        stub = self._get_stub()
        send_dict = _build_send_request(self._session_id, message)

        try:
            loop = asyncio.get_event_loop()

            # Get the server-streaming response iterator
            response_iterator = await loop.run_in_executor(
                None, lambda: stub.Send(_make_send_request(send_dict))
            )

            # Iterate over response chunks
            for proto_chunk in response_iterator:
                chunk = _proto_chunk_to_response(proto_chunk)
                self._chunks_collected.append(chunk)
                yield chunk
                if chunk.chunk_type == "done":
                    break

        except Exception as e:
            error_chunk = ResponseChunk(
                chunk_type="error", content=str(e), is_error=True
            )
            self._chunks_collected.append(error_chunk)
            yield error_chunk

    async def terminate(self) -> TelemetrySummary:
        """Terminate the gRPC session and return telemetry."""
        if self._session_id and self._stub is not None:
            try:
                loop = asyncio.get_event_loop()
                term_response = await loop.run_in_executor(
                    None,
                    lambda: self._stub.Terminate(
                        _make_terminate_request(self._session_id)
                    ),
                )
                # Extract telemetry from TerminateResponse if available
                if hasattr(term_response, "final_telemetry") and term_response.final_telemetry:
                    # Map proto TelemetryBatch → TelemetrySummary
                    pass  # Telemetry extraction deferred to real integration
            except Exception as e:
                logger.warning("Failed to terminate gRPC session: %s", e)

        summary = self._build_telemetry()

        # Close channel
        if self._channel is not None:
            try:
                self._channel.close()
            except Exception:
                pass
            self._channel = None
            self._stub = None

        return summary

    async def validate_skill(self, skill: Skill) -> bool:
        """Validate a skill by calling LoadSkill on the runtime."""
        if not self._session_id:
            return False

        try:
            stub = self._get_stub()
            skill_content = _build_skill_content(skill)
            loop = asyncio.get_event_loop()
            response = await loop.run_in_executor(
                None,
                lambda: stub.LoadSkill(
                    _make_load_skill_request(self._session_id, skill_content)
                ),
            )
            return response.success
        except Exception:
            return False

    def _build_telemetry(self) -> TelemetrySummary:
        """Build telemetry from collected chunks."""
        tools = []
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
            hooks_fired=self._hooks_collected,
            skill_loaded=self._skill_name,
            completed_normally=completed,
        )


# ---------------------------------------------------------------------------
# Proto message construction helpers (avoid hard import of generated stubs)
# ---------------------------------------------------------------------------


class _SimpleMessage:
    """Lightweight proto-like message for when generated stubs are unavailable."""

    def __init__(self, **kwargs):
        for k, v in kwargs.items():
            setattr(self, k, v)

    def SerializeToString(self) -> bytes:
        raise NotImplementedError("Lightweight stub requires real proto stubs for serialization")


def _make_init_request(payload: dict) -> object:
    """Create an InitializeRequest proto message."""
    try:
        from claude_code_runtime._proto.eaasp.runtime.v1 import runtime_pb2

        session_payload = runtime_pb2.SessionPayload(
            user_id=payload["user_id"],
            user_role=payload["user_role"],
            org_unit=payload["org_unit"],
            managed_hooks_json=payload["managed_hooks_json"],
            hook_bridge_url=payload["hook_bridge_url"],
            telemetry_endpoint=payload["telemetry_endpoint"],
        )
        for k, v in payload.get("quotas", {}).items():
            session_payload.quotas[k] = v
        for k, v in payload.get("context", {}).items():
            session_payload.context[k] = v

        return runtime_pb2.InitializeRequest(payload=session_payload)
    except ImportError:
        return _SimpleMessage(**payload)


def _make_load_skill_request(session_id: str, skill_content: dict) -> object:
    """Create a LoadSkillRequest proto message."""
    try:
        from claude_code_runtime._proto.eaasp.runtime.v1 import runtime_pb2

        sc = runtime_pb2.SkillContent(
            skill_id=skill_content["skill_id"],
            name=skill_content["name"],
            frontmatter_yaml=skill_content["frontmatter_yaml"],
            prose=skill_content["prose"],
        )
        return runtime_pb2.LoadSkillRequest(session_id=session_id, skill=sc)
    except ImportError:
        return _SimpleMessage(session_id=session_id, skill=skill_content)


def _make_send_request(send_dict: dict) -> object:
    """Create a SendRequest proto message."""
    try:
        from claude_code_runtime._proto.eaasp.runtime.v1 import runtime_pb2

        msg = runtime_pb2.UserMessage(
            content=send_dict["message"]["content"],
            message_type=send_dict["message"]["message_type"],
        )
        for k, v in send_dict["message"].get("metadata", {}).items():
            msg.metadata[k] = v

        return runtime_pb2.SendRequest(
            session_id=send_dict["session_id"], message=msg
        )
    except ImportError:
        return _SimpleMessage(**send_dict)


def _make_terminate_request(session_id: str) -> object:
    """Create a TerminateRequest proto message."""
    try:
        from claude_code_runtime._proto.eaasp.runtime.v1 import runtime_pb2

        return runtime_pb2.TerminateRequest(session_id=session_id)
    except ImportError:
        return _SimpleMessage(session_id=session_id)


class _LightweightRuntimeStub:
    """Fallback stub when generated proto stubs are not available.

    Raises descriptive errors — real usage requires proto stubs.
    """

    def __init__(self, channel):
        self._channel = channel

    def Initialize(self, request):
        raise SandboxError(
            "Proto stubs not available. Install claude-code-runtime or compile "
            "proto stubs to use RuntimeSandbox with a real gRPC server."
        )

    def Send(self, request):
        raise SandboxError("Proto stubs not available.")

    def LoadSkill(self, request):
        raise SandboxError("Proto stubs not available.")

    def Terminate(self, request):
        raise SandboxError("Proto stubs not available.")
