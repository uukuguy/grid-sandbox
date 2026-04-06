"""gRPC RuntimeService implementation — 16-method EAASP L1 contract."""

from __future__ import annotations

import json
import logging
import time
import uuid

import grpc

from ._proto.eaasp.common.v1 import common_pb2
from ._proto.eaasp.runtime.v1 import runtime_pb2, runtime_pb2_grpc
from .config import RuntimeConfig
from .sdk_wrapper import SdkWrapper

logger = logging.getLogger(__name__)


class RuntimeServiceImpl(runtime_pb2_grpc.RuntimeServiceServicer):
    """EAASP L1 RuntimeService — Python T1 Harness."""

    def __init__(self, config: RuntimeConfig):
        self.config = config
        self.sdk = SdkWrapper(config)
        self.sessions: dict[str, dict] = {}  # session_id -> session state
        self._start_time = time.time()

    # ── 1. Health ──

    async def Health(self, request, context):
        return runtime_pb2.HealthStatus(
            healthy=True,
            runtime_id=self.config.runtime_id,
            checks={"sdk": "ok", "uptime": f"{time.time() - self._start_time:.0f}s"},
        )

    # ── 2. GetCapabilities ──

    async def GetCapabilities(self, request, context):
        return runtime_pb2.CapabilityManifest(
            runtime_id=self.config.runtime_id,
            runtime_name=self.config.runtime_name,
            tier=self.config.tier,
            model=self.config.anthropic_model_name,
            context_window=200000,
            supported_tools=["Read", "Write", "Edit", "Bash", "Glob", "Grep"],
            native_hooks=True,
            native_mcp=True,
            native_skills=True,
            requires_hook_bridge=False,
            cost=runtime_pb2.CostEstimate(
                input_cost_per_1k=0.003,
                output_cost_per_1k=0.015,
            ),
        )

    # ── 3. Initialize ──

    async def Initialize(self, request, context):
        payload = request.payload
        session_id = f"crt-{uuid.uuid4().hex[:12]}"

        self.sessions[session_id] = {
            "user_id": payload.user_id,
            "user_role": payload.user_role,
            "org_unit": payload.org_unit,
            "managed_hooks_json": payload.managed_hooks_json,
            "skills": [],
            "mcp_servers": [],
            "telemetry": [],
            "state": "active",
            "created_at": time.time(),
        }

        logger.info("Session initialized: %s (user=%s)", session_id, payload.user_id)
        return runtime_pb2.InitializeResponse(session_id=session_id)

    # ── 4. Send (streaming) ──

    async def Send(self, request, context):
        session_id = request.session_id
        if session_id not in self.sessions:
            context.set_code(grpc.StatusCode.NOT_FOUND)
            context.set_details(f"Session {session_id} not found")
            return

        message = request.message
        logger.info(
            "Send: session=%s content=%s",
            session_id,
            message.content[:50],
        )

        self.sessions[session_id]["telemetry"].append(
            {"event_type": "send", "timestamp": time.time()}
        )

        async for chunk in self.sdk.send_message(prompt=message.content):
            yield runtime_pb2.ResponseChunk(
                chunk_type=chunk.chunk_type,
                content=chunk.content,
                tool_name=chunk.tool_name,
                tool_id=chunk.tool_id,
                is_error=chunk.is_error,
            )

    # ── 5. LoadSkill ──

    async def LoadSkill(self, request, context):
        session_id = request.session_id
        if session_id not in self.sessions:
            context.set_code(grpc.StatusCode.NOT_FOUND)
            return runtime_pb2.LoadSkillResponse(
                success=False, error="session not found"
            )

        skill = request.skill
        self.sessions[session_id]["skills"].append(
            {"skill_id": skill.skill_id, "name": skill.name}
        )
        logger.info("Skill loaded: %s in session %s", skill.name, session_id)
        return runtime_pb2.LoadSkillResponse(success=True)

    # ── 6. OnToolCall ──

    async def OnToolCall(self, request, context):
        logger.info(
            "OnToolCall: session=%s tool=%s",
            request.session_id,
            request.tool_name,
        )
        # T1 Harness: hooks execute natively, always allow
        return common_pb2.HookDecision(
            decision="allow", reason="", modified_input=""
        )

    # ── 7. OnToolResult ──

    async def OnToolResult(self, request, context):
        logger.info(
            "OnToolResult: session=%s tool=%s error=%s",
            request.session_id,
            request.tool_name,
            request.is_error,
        )
        return common_pb2.HookDecision(
            decision="allow", reason="", modified_input=""
        )

    # ── 8. OnStop ──

    async def OnStop(self, request, context):
        logger.info("OnStop: session=%s", request.session_id)
        return common_pb2.StopDecision(decision="complete", feedback="")

    # ── 9. ConnectMcp ──

    async def ConnectMcp(self, request, context):
        session_id = request.session_id
        if session_id not in self.sessions:
            context.set_code(grpc.StatusCode.NOT_FOUND)
            return runtime_pb2.ConnectMcpResponse(success=False)

        connected = []
        for server in request.servers:
            self.sessions[session_id]["mcp_servers"].append(server.name)
            connected.append(server.name)
            logger.info("MCP connected: %s in session %s", server.name, session_id)

        return runtime_pb2.ConnectMcpResponse(
            success=True, connected=connected, failed=[]
        )

    # ── 10. DisconnectMcp ──

    async def DisconnectMcp(self, request, context):
        session_id = request.session_id
        if session_id in self.sessions:
            servers = self.sessions[session_id]["mcp_servers"]
            if request.server_name in servers:
                servers.remove(request.server_name)
        return runtime_pb2.DisconnectMcpResponse(success=True)

    # ── 11. EmitTelemetry ──

    async def EmitTelemetry(self, request, context):
        session_id = request.session_id
        events = []
        if session_id in self.sessions:
            for t in self.sessions[session_id].get("telemetry", []):
                events.append(
                    common_pb2.TelemetryEvent(
                        session_id=session_id,
                        runtime_id=self.config.runtime_id,
                        event_type=t.get("event_type", "unknown"),
                        timestamp=str(t.get("timestamp", "")),
                        payload_json=json.dumps(t),
                    )
                )
        return common_pb2.TelemetryBatch(events=events)

    # ── 12. GetState ──

    async def GetState(self, request, context):
        session_id = request.session_id
        if session_id not in self.sessions:
            context.set_code(grpc.StatusCode.NOT_FOUND)
            return runtime_pb2.SessionState()

        state_data = json.dumps(self.sessions[session_id]).encode()
        return runtime_pb2.SessionState(
            session_id=session_id,
            state_data=state_data,
            runtime_id=self.config.runtime_id,
            created_at=str(self.sessions[session_id].get("created_at", "")),
            state_format="python-json",
        )

    # ── 13. RestoreState ──

    async def RestoreState(self, request, context):
        try:
            state = json.loads(request.state_data)
            session_id = (
                request.session_id or f"crt-restored-{uuid.uuid4().hex[:8]}"
            )
            self.sessions[session_id] = state
            logger.info("State restored: session=%s", session_id)
            return runtime_pb2.InitializeResponse(session_id=session_id)
        except Exception as e:
            context.set_code(grpc.StatusCode.INVALID_ARGUMENT)
            context.set_details(str(e))
            return runtime_pb2.InitializeResponse(session_id="")

    # ── 14. PauseSession ──

    async def PauseSession(self, request, context):
        session_id = request.session_id
        if session_id in self.sessions:
            self.sessions[session_id]["state"] = "paused"
            return runtime_pb2.PauseResponse(success=True)
        return runtime_pb2.PauseResponse(success=False)

    # ── 15. ResumeSession ──

    async def ResumeSession(self, request, context):
        session_id = request.session_id
        if session_id in self.sessions:
            self.sessions[session_id]["state"] = "active"
            return runtime_pb2.ResumeResponse(
                success=True, session_id=session_id
            )
        context.set_code(grpc.StatusCode.NOT_FOUND)
        return runtime_pb2.ResumeResponse(success=False, session_id="")

    # ── 16. Terminate ──

    async def Terminate(self, request, context):
        session_id = request.session_id
        telemetry_batch = None

        if session_id in self.sessions:
            events = []
            for t in self.sessions[session_id].get("telemetry", []):
                events.append(
                    common_pb2.TelemetryEvent(
                        session_id=session_id,
                        runtime_id=self.config.runtime_id,
                        event_type=t.get("event_type", ""),
                        timestamp=str(t.get("timestamp", "")),
                    )
                )
            telemetry_batch = common_pb2.TelemetryBatch(events=events)
            del self.sessions[session_id]
            logger.info("Session terminated: %s", session_id)

        return runtime_pb2.TerminateResponse(
            success=True, final_telemetry=telemetry_batch
        )
