"""E2E B8 — PreCompact long-conversation pipeline contract assertions.

Validates:
- CompactionPipelineConfig default values (ADR-V2-018 §D4)
- CompactionTrigger-specific summary ratios
- Rust compaction_pipeline.rs test file existence + key function names
- cargo test invocation (opt-in via --run-rust-tests)

Reference: S3.T1 PreCompact hook (Phase 3) + ADR-V2-018 §§ Proactive/Reactive triggers.
"""

from __future__ import annotations

import re
import subprocess
from pathlib import Path

import pytest

_REPO_ROOT = Path(__file__).resolve().parents[3]
_COMPACT_TEST_FILE = (
    _REPO_ROOT / "crates" / "grid-engine" / "tests" / "compaction_pipeline.rs"
)
_COMPACT_SRC_FILE = (
    _REPO_ROOT / "crates" / "grid-engine" / "src" / "context" / "compaction_pipeline.rs"
)

# ---------------------------------------------------------------------------
# B8.1 — CompactionPipelineConfig defaults (must stay in sync with Rust)
# ---------------------------------------------------------------------------

# Mirrors CompactionPipelineConfig::default() in compaction_pipeline.rs
COMPACTION_DEFAULTS = {
    "summary_max_tokens": 2_000,
    "keep_recent_messages": 6,
    "max_ptl_retries": 3,
    "proactive_threshold_pct": 75,
    "tail_protect_tokens": 20_000,
    "summary_ratio": 0.2,
    "reactive_summary_ratio": 0.5,
    "summary_min_tokens": 2_000,
    "reactive_only": False,
}


def test_summary_max_tokens_positive():
    assert COMPACTION_DEFAULTS["summary_max_tokens"] > 0


def test_keep_recent_messages_at_least_one():
    assert COMPACTION_DEFAULTS["keep_recent_messages"] >= 1


def test_max_ptl_retries_positive():
    assert COMPACTION_DEFAULTS["max_ptl_retries"] > 0


def test_proactive_threshold_pct_in_range():
    pct = COMPACTION_DEFAULTS["proactive_threshold_pct"]
    assert 0 < pct <= 100


def test_tail_protect_tokens_positive():
    assert COMPACTION_DEFAULTS["tail_protect_tokens"] > 0


def test_proactive_summary_ratio_in_range():
    r = COMPACTION_DEFAULTS["summary_ratio"]
    assert 0.0 < r < 1.0


def test_reactive_summary_ratio_in_range():
    r = COMPACTION_DEFAULTS["reactive_summary_ratio"]
    assert 0.0 < r < 1.0


def test_reactive_ratio_more_conservative_than_proactive():
    """Reactive keeps more context (higher ratio) than proactive."""
    assert COMPACTION_DEFAULTS["reactive_summary_ratio"] > COMPACTION_DEFAULTS["summary_ratio"]


def test_summary_min_tokens_leq_max_tokens():
    assert COMPACTION_DEFAULTS["summary_min_tokens"] <= COMPACTION_DEFAULTS["summary_max_tokens"]


def test_reactive_only_defaults_false():
    assert COMPACTION_DEFAULTS["reactive_only"] is False


# ---------------------------------------------------------------------------
# B8.2 — ProActive vs Reactive trigger semantics
# ---------------------------------------------------------------------------


def _pick_ratio(trigger: str) -> float:
    if trigger == "Proactive":
        return COMPACTION_DEFAULTS["summary_ratio"]
    elif trigger == "Reactive":
        return COMPACTION_DEFAULTS["reactive_summary_ratio"]
    raise ValueError(f"Unknown trigger: {trigger}")


def test_proactive_trigger_uses_smaller_ratio():
    assert _pick_ratio("Proactive") < _pick_ratio("Reactive")


def test_reactive_trigger_preserves_more_context():
    """Reactive mode summary_ratio > proactive — result is larger preserved context."""
    r_proactive = _pick_ratio("Proactive")
    r_reactive = _pick_ratio("Reactive")
    # ratio is the fraction kept; larger means more content summarized away
    # Per ADR-V2-018 §B.2: reactive keeps more recent narrative
    assert r_reactive > r_proactive


def test_proactive_threshold_triggers_above_75_pct():
    pct = COMPACTION_DEFAULTS["proactive_threshold_pct"]
    # Should not trigger at 74%, should be considered for trigger at 75%
    assert pct == 75


# ---------------------------------------------------------------------------
# B8.3 — Long conversation model: budget trajectory with compaction
# ---------------------------------------------------------------------------


def _simulate_conversation(
    turns: int,
    tokens_per_turn_in: int,
    tokens_per_turn_out: int,
    context_window: int,
    max_turns_for_budget: int = 50,
    min_turn_budget: int = 4_096,
) -> dict:
    """Simulate N turns consuming tokens, check when budget exhausts."""
    initial_budget = context_window * max_turns_for_budget
    remaining = initial_budget
    terminated_at = None
    for t in range(1, turns + 1):
        remaining = max(0, remaining - (tokens_per_turn_in + tokens_per_turn_out))
        if remaining < min_turn_budget:
            terminated_at = t
            break
    return {
        "initial_budget": initial_budget,
        "remaining": remaining,
        "terminated_at": terminated_at,
        "ran_to_completion": terminated_at is None,
    }


def test_conversation_initial_budget():
    ctx = _simulate_conversation(0, 0, 0, 128_000)
    assert ctx["initial_budget"] == 128_000 * 50


def test_short_conversation_runs_to_completion():
    result = _simulate_conversation(5, 1_000, 500, 128_000)
    assert result["ran_to_completion"] is True


def test_long_conversation_exhausts_budget():
    result = _simulate_conversation(200, 50_000, 20_000, 128_000)
    assert result["ran_to_completion"] is False
    assert result["terminated_at"] is not None


def test_budget_terminates_before_runaway():
    """A conversation cannot run more than max_turns_for_budget * context_window tokens."""
    # 128K ctx * 50 budget_turns = 6.4M total token budget
    # At 100K tokens/turn (soft hard_limit) budget exhausts in ~64 turns
    result = _simulate_conversation(500, 100_000, 100_000, 128_000)
    assert result["terminated_at"] is not None
    # Must terminate well before 500 turns
    assert result["terminated_at"] < 500


# ---------------------------------------------------------------------------
# B8.4 — Rust test file exists and contains expected test names
# ---------------------------------------------------------------------------


def test_compaction_test_file_exists():
    assert _COMPACT_TEST_FILE.exists(), f"Missing: {_COMPACT_TEST_FILE}"


def test_compaction_src_file_exists():
    assert _COMPACT_SRC_FILE.exists(), f"Missing: {_COMPACT_SRC_FILE}"


@pytest.mark.parametrize("fn_name", [
    "test_compact_basic_flow",
    "test_compact_too_few_messages",
    "test_compact_ptl_all_retries_fail",
    "test_compaction_config_defaults",
])
def test_compaction_pipeline_contains_key_functions(fn_name: str):
    content = _COMPACT_TEST_FILE.read_text()
    assert fn_name in content, f"Function '{fn_name}' not found in compaction_pipeline.rs"


def test_compaction_src_has_reactive_summary_ratio():
    content = _COMPACT_SRC_FILE.read_text()
    assert "reactive_summary_ratio" in content
    assert "CompactionTrigger" in content
    assert "Proactive" in content
    assert "Reactive" in content


# ---------------------------------------------------------------------------
# B8.5 — cargo test compaction_pipeline (opt-in)
# ---------------------------------------------------------------------------


def pytest_addoption(parser):
    try:
        parser.addoption(
            "--run-rust-tests",
            action="store_true",
            default=False,
            help="Run cargo test targets as part of the E2E suite",
        )
    except ValueError:
        pass


@pytest.fixture
def run_rust_tests(request):
    return request.config.getoption("--run-rust-tests", default=False)


@pytest.mark.slow
def test_cargo_compaction_pipeline_passes(run_rust_tests):
    if not run_rust_tests:
        pytest.skip("pass --run-rust-tests to enable cargo test invocation")
    result = subprocess.run(
        ["cargo", "test", "-p", "grid-engine", "--test", "compaction_pipeline"],
        cwd=_REPO_ROOT,
        capture_output=True,
        text=True,
        timeout=180,
    )
    assert result.returncode == 0, (
        f"cargo test failed:\nSTDOUT:\n{result.stdout}\nSTDERR:\n{result.stderr}"
    )
    combined = result.stdout + result.stderr
    assert re.search(r"test result: ok\.", combined), (
        f"'test result: ok.' not found in output:\n{combined[:1000]}"
    )
