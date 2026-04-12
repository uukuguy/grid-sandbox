"""Tests for P4 scoped hook loading and evaluation in RuntimeServiceImpl.

S3.T2 — Verifies that frontmatter_hooks from SkillInstructions (P4) are
loaded into the per-session HookExecutor during Initialize and correctly
evaluated by OnToolCall / OnToolResult / OnStop.
"""

import pytest

from claude_code_runtime._proto.eaasp.runtime.v2 import (
    common_pb2,
    runtime_pb2,
)
from claude_code_runtime.config import RuntimeConfig
from claude_code_runtime.service import RuntimeServiceImpl


class FakeContext:
    """Minimal fake gRPC context for unit tests."""

    def __init__(self):
        self.code = None
        self.details = None

    def set_code(self, code):
        self.code = code

    def set_details(self, details):
        self.details = details


@pytest.fixture
def config():
    return RuntimeConfig(
        grpc_port=50099,
        runtime_id="test-scoped-hooks",
        runtime_name="Test Scoped Hooks",
        anthropic_model_name="test-model",
    )


@pytest.fixture
def service(config):
    return RuntimeServiceImpl(config)


@pytest.fixture
def ctx():
    return FakeContext()


def _payload_with_scoped_hooks(
    user_id: str = "hook-user",
    scoped_hooks: list[dict] | None = None,
) -> common_pb2.SessionPayload:
    """Build a SessionPayload with P4 SkillInstructions carrying scoped hooks."""
    skill = common_pb2.SkillInstructions(
        skill_id="threshold-calibration",
        name="Threshold Calibration",
        content="Calibrate transformer thresholds",
    )
    if scoped_hooks:
        for h in scoped_hooks:
            skill.frontmatter_hooks.append(common_pb2.ScopedHook(**h))

    payload = common_pb2.SessionPayload(
        user_id=user_id,
        skill_instructions=skill,
    )
    return payload


# ── Initialize loads scoped hooks ────────────────────────────────


@pytest.mark.asyncio
async def test_initialize_loads_scoped_hooks(service, ctx):
    """Scoped hooks from P4 must be loaded into HookExecutor during Initialize."""
    payload = _payload_with_scoped_hooks(
        scoped_hooks=[
            {
                "hook_id": "block-scada-write",
                "hook_type": "PreToolUse",
                "condition": "scada_write*",
                "action": "exit 2",
                "precedence": 0,
            },
            {
                "hook_id": "check-output",
                "hook_type": "Stop",
                "condition": "",
                "action": "echo ok",
                "precedence": 10,
            },
        ]
    )
    req = runtime_pb2.InitializeRequest(payload=payload)
    resp = await service.Initialize(req, ctx)

    assert resp.session_id
    # Verify HookExecutor has rules loaded (P4 scoped hooks)
    hook_exe = service._hooks.get(resp.session_id)
    assert hook_exe is not None
    # 2 scoped hooks should have been loaded
    assert hook_exe.rule_count >= 2


@pytest.mark.asyncio
async def test_initialize_without_scoped_hooks_still_works(service, ctx):
    """Empty frontmatter_hooks must not cause errors."""
    payload = _payload_with_scoped_hooks(scoped_hooks=[])
    req = runtime_pb2.InitializeRequest(payload=payload)
    resp = await service.Initialize(req, ctx)
    assert resp.session_id


@pytest.mark.asyncio
async def test_initialize_without_skill_instructions(service, ctx):
    """No skill_instructions at all must not cause errors."""
    payload = common_pb2.SessionPayload(user_id="no-skill-user")
    req = runtime_pb2.InitializeRequest(payload=payload)
    resp = await service.Initialize(req, ctx)
    assert resp.session_id


# ── OnToolCall with scoped deny hook ─────────────────────────────


@pytest.mark.asyncio
async def test_on_tool_call_deny_matching_scoped_hook(service, ctx):
    """A scoped hook with condition=scada_write* + action=exit 2 must deny scada_write_temperature."""
    payload = _payload_with_scoped_hooks(
        scoped_hooks=[
            {
                "hook_id": "block-scada-write",
                "hook_type": "PreToolUse",
                "condition": "scada_write*",
                "action": "exit 2",
                "precedence": 0,
            },
        ]
    )
    req = runtime_pb2.InitializeRequest(payload=payload)
    resp = await service.Initialize(req, ctx)
    sid = resp.session_id

    # Call OnToolCall with a matching tool name
    tool_req = runtime_pb2.ToolCallEvent(
        session_id=sid,
        tool_name="scada_write_temperature",
        tool_id="t1",
        input_json='{"value": 80}',
    )
    ack = await service.OnToolCall(tool_req, ctx)
    assert ack.decision == "deny"


@pytest.mark.asyncio
async def test_on_tool_call_allow_non_matching_scoped_hook(service, ctx):
    """A scoped hook with condition=scada_write* must NOT deny scada_read_snapshot."""
    payload = _payload_with_scoped_hooks(
        scoped_hooks=[
            {
                "hook_id": "block-scada-write",
                "hook_type": "PreToolUse",
                "condition": "scada_write*",
                "action": "exit 2",
                "precedence": 0,
            },
        ]
    )
    req = runtime_pb2.InitializeRequest(payload=payload)
    resp = await service.Initialize(req, ctx)
    sid = resp.session_id

    # Call OnToolCall with a NON-matching tool name
    tool_req = runtime_pb2.ToolCallEvent(
        session_id=sid,
        tool_name="scada_read_snapshot",
        tool_id="t2",
        input_json="{}",
    )
    ack = await service.OnToolCall(tool_req, ctx)
    assert ack.decision == "allow"


# ── OnStop with scoped hook ──────────────────────────────────────


@pytest.mark.asyncio
async def test_on_stop_deny_scoped_hook(service, ctx):
    """A scoped Stop hook with action containing 'deny' must force-continue."""
    payload = _payload_with_scoped_hooks(
        scoped_hooks=[
            {
                "hook_id": "force-continue",
                "hook_type": "Stop",
                "condition": "",
                "action": "deny: missing anchor",
                "precedence": 0,
            },
        ]
    )
    req = runtime_pb2.InitializeRequest(payload=payload)
    resp = await service.Initialize(req, ctx)
    sid = resp.session_id

    stop_req = runtime_pb2.StopEvent(session_id=sid)
    ack = await service.OnStop(stop_req, ctx)
    # deny on stop → force continue
    assert ack.decision == "deny"


@pytest.mark.asyncio
async def test_on_stop_allow_scoped_hook(service, ctx):
    """A scoped Stop hook with allow action must allow stop (complete)."""
    payload = _payload_with_scoped_hooks(
        scoped_hooks=[
            {
                "hook_id": "allow-stop",
                "hook_type": "Stop",
                "condition": "",
                "action": "echo ok",
                "precedence": 0,
            },
        ]
    )
    req = runtime_pb2.InitializeRequest(payload=payload)
    resp = await service.Initialize(req, ctx)
    sid = resp.session_id

    stop_req = runtime_pb2.StopEvent(session_id=sid)
    ack = await service.OnStop(stop_req, ctx)
    assert ack.decision == "allow"


# ── _scoped_hooks_to_rules static method ─────────────────────────


def test_scoped_hooks_to_rules_glob_conversion():
    """Condition glob 'scada_write*' must map to regex '^scada_write.*'."""
    rules = RuntimeServiceImpl._scoped_hooks_to_rules(
        [
            _mock_scoped_hook(
                hook_id="h1",
                hook_type="PreToolUse",
                condition="scada_write*",
                action="exit 2",
            )
        ]
    )
    assert len(rules) == 1
    assert rules[0]["tool_pattern"] == "^scada_write.*"
    assert rules[0]["action"] == "deny"
    assert rules[0]["hook_type"] == "pre_tool_call"


def test_scoped_hooks_to_rules_exact_condition():
    """Exact condition 'bash' must map to regex '^bash$'."""
    rules = RuntimeServiceImpl._scoped_hooks_to_rules(
        [
            _mock_scoped_hook(
                hook_id="h2",
                hook_type="PostToolUse",
                condition="bash",
                action="echo ok",
            )
        ]
    )
    assert len(rules) == 1
    assert rules[0]["tool_pattern"] == "^bash$"
    assert rules[0]["action"] == "allow"
    assert rules[0]["hook_type"] == "post_tool_result"


def test_scoped_hooks_to_rules_empty_condition():
    """Empty condition must map to empty tool_pattern (match all)."""
    rules = RuntimeServiceImpl._scoped_hooks_to_rules(
        [
            _mock_scoped_hook(
                hook_id="h3",
                hook_type="Stop",
                condition="",
                action="exit 2",
            )
        ]
    )
    assert len(rules) == 1
    assert rules[0]["tool_pattern"] == ""
    assert rules[0]["hook_type"] == "stop"


def test_scoped_hooks_to_rules_wildcard_condition():
    """Wildcard '*' must map to empty tool_pattern (match all)."""
    rules = RuntimeServiceImpl._scoped_hooks_to_rules(
        [
            _mock_scoped_hook(
                hook_id="h4",
                hook_type="PreToolUse",
                condition="*",
                action="echo ok",
            )
        ]
    )
    assert len(rules) == 1
    # '*' → ends with '*', prefix is empty → '^.*'
    assert rules[0]["tool_pattern"] == "^.*"


# ── helpers ──────────────────────────────────────────────────────


class _MockScopedHook:
    """Minimal mock of proto ScopedHook for unit-testing _scoped_hooks_to_rules."""

    def __init__(self, hook_id="", hook_type="", condition="", action="", precedence=0):
        self.hook_id = hook_id
        self.hook_type = hook_type
        self.condition = condition
        self.action = action
        self.precedence = precedence


def _mock_scoped_hook(**kwargs) -> _MockScopedHook:
    return _MockScopedHook(**kwargs)
