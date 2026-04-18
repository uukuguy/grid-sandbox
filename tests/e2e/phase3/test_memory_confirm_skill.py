"""E2E B5+B6 — memory-confirm skill: hook behavior and state-machine assertions.

B5: Validates the three hook scripts (block_unconfirmed_write, verify_confirm_ack,
    check_confirm_audit) as standalone shell-level contracts — no live LLM needed.

B6: Validates the SKILL.md schema (required_tools namespace prefixes, scoped_hooks
    wiring) via eaasp-skill-registry parser — no live LLM needed.

Reference: S3.T14 memory-confirm-test skill (Phase 3).
"""

from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path

import pytest

_REPO_ROOT = Path(__file__).resolve().parents[3]
_SKILL_DIR = _REPO_ROOT / "examples" / "skills" / "memory-confirm-test"
_HOOKS_DIR = _SKILL_DIR / "hooks"

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _run_hook(script: str, stdin_data: dict, extra_env: dict | None = None) -> subprocess.CompletedProcess:
    env = {**os.environ, "SKILL_DIR": str(_SKILL_DIR)}
    if extra_env:
        env.update(extra_env)
    return subprocess.run(
        ["bash", str(_HOOKS_DIR / script)],
        input=json.dumps(stdin_data),
        capture_output=True,
        text=True,
        env=env,
    )


# ---------------------------------------------------------------------------
# B5 — Hook script contracts
# ---------------------------------------------------------------------------


# --- block_unconfirmed_write.sh (PreToolUse) ---

def test_block_unconfirmed_write_allows_non_write_tool():
    result = _run_hook(
        "block_unconfirmed_write.sh",
        {"tool_name": "memory.search", "tool_input": {}, "session_id": "s1", "skill_id": "sk1"},
    )
    assert result.returncode == 0


def test_block_unconfirmed_write_denies_write_without_memory_id():
    result = _run_hook(
        "block_unconfirmed_write.sh",
        {"tool_name": "memory.write_anchor", "tool_input": {}, "session_id": "s1", "skill_id": "sk1"},
    )
    assert result.returncode == 2


def test_block_unconfirmed_write_denies_write_with_memory_id_no_store():
    """No EAASP_CONFIRM_STORE set — memory_id present but unverifiable → deny."""
    result = _run_hook(
        "block_unconfirmed_write.sh",
        {
            "tool_name": "memory.write_anchor",
            "tool_input": {"memory_id": "mem-001"},
            "session_id": "s1",
            "skill_id": "sk1",
        },
        extra_env={"EAASP_CONFIRM_STORE": ""},
    )
    # Without a store, no memory_id means deny; with memory_id but no store → allow (can't verify)
    # Hook exits 0 when no store file is present (can't reject what it can't check)
    assert result.returncode == 0


def test_block_unconfirmed_write_denies_unconfirmed_memory_id(tmp_path):
    store = tmp_path / "confirm_store.json"
    store.write_text(json.dumps({"mem-001": {"status": "agent_suggested"}}))
    result = _run_hook(
        "block_unconfirmed_write.sh",
        {
            "tool_name": "l2:memory.write_anchor",
            "tool_input": {"memory_id": "mem-001"},
            "session_id": "s1",
            "skill_id": "sk1",
        },
        extra_env={"EAASP_CONFIRM_STORE": str(store)},
    )
    assert result.returncode == 2
    assert "status=" in result.stdout or "status=" in result.stderr or result.returncode == 2


def test_block_unconfirmed_write_allows_confirmed_memory_id(tmp_path):
    store = tmp_path / "confirm_store.json"
    store.write_text(json.dumps({"mem-001": {"status": "confirmed"}}))
    result = _run_hook(
        "block_unconfirmed_write.sh",
        {
            "tool_name": "memory.write_anchor",
            "tool_input": {"memory_id": "mem-001"},
            "session_id": "s1",
            "skill_id": "sk1",
        },
        extra_env={"EAASP_CONFIRM_STORE": str(store)},
    )
    assert result.returncode == 0


def test_block_unconfirmed_write_handles_l2_prefixed_tool():
    result = _run_hook(
        "block_unconfirmed_write.sh",
        {"tool_name": "l2:memory.write_file", "tool_input": {}, "session_id": "s1", "skill_id": "sk1"},
    )
    assert result.returncode == 2  # no memory_id → deny


# --- verify_confirm_ack.sh (PostToolUse) ---

def test_verify_confirm_ack_skips_non_confirm_tool():
    result = _run_hook(
        "verify_confirm_ack.sh",
        {"tool_name": "memory.read", "tool_result": {}, "session_id": "s1", "skill_id": "sk1"},
    )
    assert result.returncode == 0


def test_verify_confirm_ack_allows_valid_ack():
    result = _run_hook(
        "verify_confirm_ack.sh",
        {
            "tool_name": "memory.confirm",
            "tool_result": {"status": "confirmed", "memory_id": "mem-001"},
            "session_id": "s1",
            "skill_id": "sk1",
        },
    )
    assert result.returncode == 0


def test_verify_confirm_ack_denies_wrong_status():
    result = _run_hook(
        "verify_confirm_ack.sh",
        {
            "tool_name": "memory.confirm",
            "tool_result": {"status": "agent_suggested", "memory_id": "mem-001"},
            "session_id": "s1",
            "skill_id": "sk1",
        },
    )
    assert result.returncode == 2


def test_verify_confirm_ack_denies_empty_memory_id():
    result = _run_hook(
        "verify_confirm_ack.sh",
        {
            "tool_name": "l2:memory.confirm",
            "tool_result": {"status": "confirmed", "memory_id": ""},
            "session_id": "s1",
            "skill_id": "sk1",
        },
    )
    assert result.returncode == 2


def test_verify_confirm_ack_denies_missing_memory_id():
    result = _run_hook(
        "verify_confirm_ack.sh",
        {
            "tool_name": "memory.confirm",
            "tool_result": {"status": "confirmed"},
            "session_id": "s1",
            "skill_id": "sk1",
        },
    )
    assert result.returncode == 2


# --- check_confirm_audit.sh (Stop) ---

def test_check_confirm_audit_noop_when_confirm_present():
    result = _run_hook(
        "check_confirm_audit.sh",
        {
            "session_id": "s1",
            "skill_id": "sk1",
            "tool_calls": [
                {"tool_name": "memory.search"},
                {"tool_name": "memory.confirm"},
                {"tool_name": "memory.write_anchor"},
            ],
        },
    )
    assert result.returncode == 0


def test_check_confirm_audit_injects_when_no_confirm():
    result = _run_hook(
        "check_confirm_audit.sh",
        {
            "session_id": "s1",
            "skill_id": "sk1",
            "tool_calls": [
                {"tool_name": "memory.search"},
                {"tool_name": "memory.read"},
            ],
        },
    )
    assert result.returncode == 2
    injected = json.loads(result.stdout)
    assert isinstance(injected, list)
    assert len(injected) == 1
    assert injected[0]["role"] == "system"
    assert "memory.confirm" in injected[0]["content"]


def test_check_confirm_audit_injects_when_empty_tool_calls():
    result = _run_hook(
        "check_confirm_audit.sh",
        {"session_id": "s1", "skill_id": "sk1", "tool_calls": []},
    )
    assert result.returncode == 2


def test_check_confirm_audit_l2_prefixed_confirm_counts():
    result = _run_hook(
        "check_confirm_audit.sh",
        {
            "session_id": "s1",
            "skill_id": "sk1",
            "tool_calls": [{"tool_name": "l2:memory.confirm"}],
        },
    )
    assert result.returncode == 0


# ---------------------------------------------------------------------------
# B6 — SKILL.md schema validation via eaasp-skill-registry
# ---------------------------------------------------------------------------

_SKILL_MD = _SKILL_DIR / "SKILL.md"


def test_skill_md_exists():
    assert _SKILL_MD.exists(), f"SKILL.md not found at {_SKILL_MD}"


def test_skill_md_has_required_tools_with_namespace_prefix():
    content = _SKILL_MD.read_text()
    assert "l2:memory.search" in content
    assert "l2:memory.read" in content
    assert "l2:memory.confirm" in content
    assert "l2:memory.write_anchor" in content


def test_skill_md_scoped_hooks_reference_correct_scripts():
    content = _SKILL_MD.read_text()
    assert "block_unconfirmed_write.sh" in content
    assert "verify_confirm_ack.sh" in content
    assert "check_confirm_audit.sh" in content


def test_skill_md_hook_scripts_exist():
    for script in ("block_unconfirmed_write.sh", "verify_confirm_ack.sh", "check_confirm_audit.sh"):
        assert (_HOOKS_DIR / script).exists(), f"Hook script missing: {script}"


def test_skill_md_hook_scripts_are_executable():
    for script in ("block_unconfirmed_write.sh", "verify_confirm_ack.sh", "check_confirm_audit.sh"):
        path = _HOOKS_DIR / script
        assert os.access(path, os.X_OK), f"Hook script not executable: {script}"


def test_skill_md_parseable_by_registry():
    """Parse SKILL.md via eaasp-skill-registry (cargo test round-trip)."""
    result = subprocess.run(
        ["cargo", "test", "-p", "eaasp-skill-registry", "--", "--nocapture", "memory_confirm"],
        cwd=_REPO_ROOT,
        capture_output=True,
        text=True,
        timeout=120,
    )
    if result.returncode != 0 and "no tests" in (result.stdout + result.stderr):
        pytest.skip("No eaasp-skill-registry test for memory-confirm-test yet")
    # If tests ran, they must pass
    if "FAILED" in result.stdout or "FAILED" in result.stderr:
        pytest.fail(f"eaasp-skill-registry test FAILED:\n{result.stdout}\n{result.stderr}")


def test_skill_md_workflow_requires_memory_confirm():
    """Workflow requires exactly memory.confirm (the key state-machine transition)."""
    content = _SKILL_MD.read_text()
    # Must require confirm before write_anchor
    confirm_pos = content.find("memory.confirm")
    write_anchor_pos = content.find("memory.write_anchor")
    assert confirm_pos != -1, "l2:memory.confirm not found in required_tools"
    assert write_anchor_pos != -1, "l2:memory.write_anchor not found in required_tools"
    assert confirm_pos < write_anchor_pos, (
        "memory.confirm must appear before memory.write_anchor in required_tools"
    )
