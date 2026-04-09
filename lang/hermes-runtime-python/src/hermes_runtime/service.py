"""gRPC RuntimeService — EAASP L1 16-method contract for hermes-agent."""

import json
import logging
import time

import grpc

from hermes_runtime._fix_proto_imports import fix as _fix_proto_imports

_fix_proto_imports()

from eaasp.common.v1 import common_pb2  # noqa: E402
from eaasp.runtime.v1 import runtime_pb2, runtime_pb2_grpc  # noqa: E402

from hermes_runtime.adapter import HermesAdapter
from hermes_runtime.config import HermesRuntimeConfig
from hermes_runtime.mapper import chunk_to_proto
from hermes_runtime.session import SessionManager
from hermes_runtime.telemetry import TelemetryCollector

logger = logging.getLogger(__name__)


class RuntimeServiceImpl(runtime_pb2_grpc.RuntimeServiceServicer):
    """EAASP L1 RuntimeService — Hermes Agent T2 Aligned."""

    def __init__(self, config: HermesRuntimeConfig):
        self.config = config
        self.adapter = HermesAdapter(config)
        self.session_mgr = SessionManager()
        self._telemetry: dict[str, TelemetryCollector] = {}
        self._start_time = time.time()

    def _get_or_404(self, session_id: str, context):
        session = self.session_mgr.get(session_id)
        if session is None:
            context.set_code(grpc.StatusCode.NOT_FOUND)
            context.set_details(f"Session {session_id} not found")
        return session

    # ── 1. Health ──

    async def Health(self, request, context):
        return runtime_pb2.HealthStatus(
            healthy=True,
            runtime_id=self.config.runtime_id,
            checks={
                "hermes": "ok",
                "sessions": str(self.session_mgr.count),
                "uptime": f"{time.time() - self._start_time:.0f}s",
            },
        )

    # ── 2. GetCapabilities ──

    async def GetCapabilities(self, request, context):
        return runtime_pb2.CapabilityManifest(
            runtime_id=self.config.runtime_id,
            runtime_name=self.config.runtime_name,
            tier=self.config.tier,
            model=self.config.hermes_model,
            context_window=200000,
            supported_tools=[
                "terminal", "read_file", "write_file", "patch", "search_files",
                "web_search", "web_extract", "browser_navigate", "execute_code",
                "delegate_task", "memory", "todo", "skills_list", "skill_view",
            ],
            native_hooks=False,  # T2 — uses HookBridge
            native_mcp=True,     # hermes has native MCP
            native_skills=True,  # hermes has native skills
            requires_hook_bridge=True,
            deployment_mode=self.config.deployment_mode,
            cost=runtime_pb2.CostEstimate(
                input_cost_per_1k=0.0,
                output_cost_per_1k=0.0,
            ),
        )

    # ── 3. Initialize ──

    async def Initialize(self, request, context):
        payload = request.payload
        session = self.session_mgr.create(
            user_id=payload.user_id,
            user_role=payload.user_role,
            org_unit=payload.org_unit,
            managed_hooks_json=payload.managed_hooks_json,
            context=dict(payload.context) if payload.context else {},
            hook_bridge_url=payload.hook_bridge_url or self.config.hook_bridge_url,
            telemetry_endpoint=payload.telemetry_endpoint,
        )
        sid = session.session_id

        # Create AIAgent instance for this session
        try:
            self.adapter.create_agent(sid)
        except Exception as e:
            logger.error("Failed to create AIAgent for %s: %s", sid, e)
            context.set_code(grpc.StatusCode.INTERNAL)
            context.set_details(str(e))
            self.session_mgr.terminate(sid)
            return runtime_pb2.InitializeResponse(session_id="")

        self._telemetry[sid] = TelemetryCollector(
            session_id=sid,
            runtime_id=self.config.runtime_id,
            user_id=payload.user_id,
        )
        self._telemetry[sid].record("session_start")

        logger.info(
            "Session initialized: %s (user=%s, model=%s)",
            sid, payload.user_id, self.config.hermes_model,
        )
        return runtime_pb2.InitializeResponse(session_id=sid)

    # ── 4. Send (streaming) ──

    async def Send(self, request, context):
        session = self._get_or_404(request.session_id, context)
        if session is None:
            return

        sid = session.session_id
        message = request.message
        logger.info("Send: session=%s content=%s", sid, message.content[:80])

        tc = self._telemetry.get(sid)
        if tc:
            tc.record("send", payload={"content_len": len(message.content)})

        for chunk in self.adapter.send_message(
            session_id=sid,
            content=message.content,
            conversation_history=session.conversation_history,
        ):
            yield chunk_to_proto(**chunk)

        # Persist conversation entry
        session.conversation_history.append({"role": "user", "content": message.content})

    # ── 5. LoadSkill ──

    async def LoadSkill(self, request, context):
        session = self._get_or_404(request.session_id, context)
        if session is None:
            return runtime_pb2.LoadSkillResponse(success=False, error="session not found")

        skill = request.skill
        session.skills.append({"skill_id": skill.skill_id, "name": skill.name})

        tc = self._telemetry.get(session.session_id)
        if tc:
            tc.record("skill_loaded", payload={"skill_id": skill.skill_id})

        return runtime_pb2.LoadSkillResponse(success=True)

    # ── 6. OnToolCall ──

    async def OnToolCall(self, request, context):
        # T2: 治理拦截已在 governance_plugin monkey-patch 中完成
        return common_pb2.HookDecision(decision="allow", reason="", modified_input="")

    # ── 7. OnToolResult ──

    async def OnToolResult(self, request, context):
        return common_pb2.HookDecision(decision="allow", reason="", modified_input="")

    # ── 8. OnStop ──

    async def OnStop(self, request, context):
        return common_pb2.StopDecision(decision="complete", feedback="")

    # ── 9. ConnectMcp ──

    async def ConnectMcp(self, request, context):
        session = self._get_or_404(request.session_id, context)
        if session is None:
            return runtime_pb2.ConnectMcpResponse(success=False)
        connected = [s.name for s in request.servers]
        session.mcp_servers.extend(connected)
        return runtime_pb2.ConnectMcpResponse(success=True, connected=connected, failed=[])

    # ── 10. DisconnectMcp ──

    async def DisconnectMcp(self, request, context):
        session = self.session_mgr.get(request.session_id)
        if session and request.server_name in session.mcp_servers:
            session.mcp_servers.remove(request.server_name)
        return runtime_pb2.DisconnectMcpResponse(success=True)

    # ── 11. EmitTelemetry ──

    async def EmitTelemetry(self, request, context):
        tc = self._telemetry.get(request.session_id)
        if tc:
            tc.peek()  # acknowledge entries exist
        return common_pb2.TelemetryBatch(events=[])

    # ── 12. GetState ──

    async def GetState(self, request, context):
        session = self._get_or_404(request.session_id, context)
        if session is None:
            return runtime_pb2.SessionState()
        state_data = json.dumps({
            "session_id": session.session_id,
            "user_id": session.user_id,
            "conversation_history": session.conversation_history,
            "skills": session.skills,
        }).encode()
        return runtime_pb2.SessionState(
            session_id=session.session_id,
            state_data=state_data,
            runtime_id=self.config.runtime_id,
            created_at=session.created_at,
            state_format="hermes-json-v1",
        )

    # ── 13. RestoreState ──

    async def RestoreState(self, request, context):
        try:
            data = json.loads(request.state_data)
            session = self.session_mgr.restore(data)
            self.adapter.create_agent(session.session_id)
            sid = session.session_id
            self._telemetry[sid] = TelemetryCollector(
                session_id=sid,
                runtime_id=self.config.runtime_id,
                user_id=session.user_id,
            )
            return runtime_pb2.InitializeResponse(session_id=sid)
        except Exception as e:
            context.set_code(grpc.StatusCode.INVALID_ARGUMENT)
            context.set_details(str(e))
            return runtime_pb2.InitializeResponse(session_id="")

    # ── 14. PauseSession ──

    async def PauseSession(self, request, context):
        success = self.session_mgr.pause(request.session_id)
        return runtime_pb2.PauseResponse(success=success)

    # ── 15. ResumeSession ──

    async def ResumeSession(self, request, context):
        success = self.session_mgr.resume(request.session_id)
        if success:
            return runtime_pb2.ResumeResponse(success=True, session_id=request.session_id)
        context.set_code(grpc.StatusCode.NOT_FOUND)
        return runtime_pb2.ResumeResponse(success=False, session_id="")

    # ── 16. Terminate ──

    async def Terminate(self, request, context):
        sid = request.session_id
        tc = self._telemetry.pop(sid, None)
        if tc:
            tc.record("session_end")
            tc.flush()
        self.adapter.remove_agent(sid)
        session = self.session_mgr.terminate(sid)
        return runtime_pb2.TerminateResponse(success=session is not None)
