"""Tests for HR onboarding example — audit hook + PII enforcement (6 tests).

These tests validate the HR example end-to-end by compiling policies,
loading them into HookExecutor, and verifying enforcement behavior.
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest

TOOLS_DIR = Path(__file__).resolve().parents[2] / "tools"
RUNTIME_DIR = Path(__file__).resolve().parents[2] / "lang" / "claude-code-runtime-python" / "src"
sys.path.insert(0, str(TOOLS_DIR / "eaasp-governance" / "src"))
sys.path.insert(0, str(RUNTIME_DIR))

from eaasp_governance.compiler import compile_yaml_to_hooks
from eaasp_governance.merger import merge_by_scope
from claude_code_runtime.hook_executor import HookExecutor

POLICIES_DIR = Path(__file__).resolve().parents[2] / "sdk" / "examples" / "hr-onboarding" / "policies"
HR_DIR = Path(__file__).resolve().parents[2] / "sdk" / "examples" / "hr-onboarding"


@pytest.fixture
def hook_executor():
    """Compile + merge HR policies into a HookExecutor."""
    enterprise_json, _ = compile_yaml_to_hooks((POLICIES_DIR / "enterprise.yaml").read_text())
    bu_json, _ = compile_yaml_to_hooks((POLICIES_DIR / "bu_hr.yaml").read_text())
    merged = merge_by_scope(managed=enterprise_json, skill=bu_json)

    executor = HookExecutor()
    executor.load_rules(merged)
    return executor


# ── Test 1: PII deny — SSN pattern ──────────────────────────

@pytest.mark.e2e
@pytest.mark.mock_llm
def test_pii_deny_ssn(hook_executor):
    """file_write with SSN pattern is denied."""
    decision, reason = hook_executor.evaluate_pre_tool_call(
        "file_write",
        json.dumps({"content": "Employee SSN: 123-45-6789"}),
    )
    assert decision == "deny"
    assert "PII" in reason


# ── Test 2: PII deny — Chinese ID number ────────────────────

@pytest.mark.e2e
@pytest.mark.mock_llm
def test_pii_deny_chinese_id(hook_executor):
    """file_write with 18-digit Chinese ID number is denied."""
    decision, reason = hook_executor.evaluate_pre_tool_call(
        "file_write",
        json.dumps({"content": "身份证号: 310101199001011234"}),
    )
    assert decision == "deny"


# ── Test 3: PII allow — clean data ──────────────────────────

@pytest.mark.e2e
@pytest.mark.mock_llm
def test_pii_allow_clean(hook_executor):
    """file_write without PII is allowed."""
    decision, _ = hook_executor.evaluate_pre_tool_call(
        "file_write",
        json.dumps({"content": "Employee ID: E2024001, Department: Engineering"}),
    )
    assert decision == "allow"


# ── Test 4: Bash tool denied ────────────────────────────────

@pytest.mark.e2e
@pytest.mark.mock_llm
def test_bash_denied(hook_executor):
    """bash tool is blocked by BU policy."""
    decision, reason = hook_executor.evaluate_pre_tool_call("bash", "ls /etc")
    assert decision == "deny"
    assert "bash" in reason.lower() or "禁止" in reason


# ── Test 5: Stop enforcement ────────────────────────────────

@pytest.mark.e2e
@pytest.mark.mock_llm
def test_stop_enforcement(hook_executor):
    """Stop hook enforces checklist completion."""
    decision, reason = hook_executor.evaluate_stop()
    assert decision == "continue"
    assert "清单" in reason or "checklist" in reason.lower()


# ── Test 6: Audit hook script runs ──────────────────────────

@pytest.mark.e2e
@pytest.mark.mock_llm
def test_audit_hook_script():
    """audit_logger.py processes tool event and returns allow."""
    audit_script = HR_DIR / "hooks" / "audit_logger.py"
    assert audit_script.exists()

    payload = json.dumps({
        "tool_name": "file_write",
        "tool_input": {"path": "/tmp/test.txt", "content": "hello"},
        "tool_output": "ok",
        "is_error": False,
        "session_id": "test-sess",
    })

    result = subprocess.run(
        [sys.executable, str(audit_script)],
        input=payload,
        capture_output=True,
        text=True,
        timeout=10,
    )
    assert result.returncode == 0

    # stdout should be JSON with allow decision
    output = json.loads(result.stdout.strip())
    assert output["decision"] == "allow"
    assert "Audit logged" in output["reason"]

    # stderr should have audit record
    audit_record = json.loads(result.stderr.strip())
    assert audit_record["event"] == "tool_execution"
    assert audit_record["tool_name"] == "file_write"
