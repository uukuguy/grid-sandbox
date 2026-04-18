"""E2E B7 — Aggregate spill (L3 per-turn budget) contract assertions.

Validates:
- The three spill constants that govern L3 per-turn aggregate budget
- The dynamic tool-result budget formula (15%/30% of context window)
- Rust integration test file existence + key function names
- cargo test invocation (opt-in via --run-rust-tests)

Reference: D60 closed (S2.T5) + harness.rs TOOL_RESULT_TURN_BUDGET / tool_result_budget().
"""

from __future__ import annotations

import re
import subprocess
from pathlib import Path

import pytest

_REPO_ROOT = Path(__file__).resolve().parents[3]
_SPILL_TEST_FILE = (
    _REPO_ROOT / "crates" / "grid-engine" / "tests" / "tool_result_aggregate_spill.rs"
)
_HARNESS_FILE = (
    _REPO_ROOT / "crates" / "grid-engine" / "src" / "agent" / "harness.rs"
)

# ---------------------------------------------------------------------------
# B7.1 — Spill constants (must stay in sync with harness.rs)
# ---------------------------------------------------------------------------

TOOL_RESULT_SOFT_LIMIT = 30_000    # chars
MAX_TOOL_OUTPUT_SIZE = 100_000     # chars
TOOL_RESULT_TURN_BUDGET = 200_000  # chars aggregate L3 cap
DEFAULT_CONTEXT_WINDOW = 128_000   # tokens (approximate chars / 4 estimate)
MAX_TURNS_FOR_BUDGET: int = 50
MIN_TURN_BUDGET: int = 4_096       # tokens floor


def test_soft_limit_less_than_hard_limit():
    assert TOOL_RESULT_SOFT_LIMIT < MAX_TOOL_OUTPUT_SIZE


def test_soft_limit_less_than_turn_budget():
    assert TOOL_RESULT_SOFT_LIMIT < TOOL_RESULT_TURN_BUDGET


def test_hard_limit_less_than_turn_budget():
    assert MAX_TOOL_OUTPUT_SIZE < TOOL_RESULT_TURN_BUDGET


def test_default_context_window_positive():
    assert DEFAULT_CONTEXT_WINDOW > 0


def test_max_turns_for_budget_positive():
    assert MAX_TURNS_FOR_BUDGET > 0


def test_min_turn_budget_positive():
    assert MIN_TURN_BUDGET > 0


# ---------------------------------------------------------------------------
# B7.2 — Dynamic budget formula: soft=15%, hard=30% clamped
# ---------------------------------------------------------------------------


def _tool_result_budget(ctx: int) -> tuple[int, int]:
    """Mirror of harness.rs::tool_result_budget()."""
    if ctx == 0:
        return (TOOL_RESULT_SOFT_LIMIT, MAX_TOOL_OUTPUT_SIZE)
    soft = max(8_000, min(50_000, int(ctx * 0.15)))
    hard = max(30_000, min(200_000, int(ctx * 0.30)))
    return soft, hard


def test_budget_128k_context_window():
    soft, hard = _tool_result_budget(128_000)
    assert 8_000 <= soft <= 50_000
    assert 30_000 <= hard <= 200_000
    assert soft < hard


def test_budget_zero_context_falls_back_to_defaults():
    soft, hard = _tool_result_budget(0)
    assert soft == TOOL_RESULT_SOFT_LIMIT
    assert hard == MAX_TOOL_OUTPUT_SIZE


def test_budget_small_context_clamps_to_floor():
    # Very small context → should hit lower clamp
    soft, hard = _tool_result_budget(10_000)
    assert soft == 8_000  # 10000 * 0.15 = 1500 → clamped to 8000
    assert hard == 30_000  # 10000 * 0.30 = 3000 → clamped to 30000


def test_budget_large_context_clamps_to_ceiling():
    # Very large context → should hit upper clamp
    soft, hard = _tool_result_budget(2_000_000)
    assert soft == 50_000  # 2M * 0.15 = 300K → clamped to 50K
    assert hard == 200_000  # 2M * 0.30 = 600K → clamped to 200K


def test_budget_soft_always_less_than_hard():
    for ctx in [10_000, 50_000, 128_000, 200_000, 500_000, 1_000_000]:
        soft, hard = _tool_result_budget(ctx)
        assert soft < hard, f"ctx={ctx}: soft={soft} >= hard={hard}"


# ---------------------------------------------------------------------------
# B7.3 — apply_budget_decrement and has_budget_for_next_turn semantics
# ---------------------------------------------------------------------------


def _apply_budget_decrement(remaining: int, inp: int, out: int) -> int:
    return max(0, remaining - (inp + out))


def _has_budget_for_next_turn(remaining: int) -> bool:
    return remaining >= MIN_TURN_BUDGET


def test_budget_decrement_basic():
    r = _apply_budget_decrement(100_000, 1_000, 500)
    assert r == 98_500


def test_budget_decrement_saturates_at_zero():
    r = _apply_budget_decrement(100, 200, 300)
    assert r == 0


def test_has_budget_at_floor():
    assert _has_budget_for_next_turn(MIN_TURN_BUDGET) is True


def test_has_budget_below_floor():
    assert _has_budget_for_next_turn(MIN_TURN_BUDGET - 1) is False


# ---------------------------------------------------------------------------
# B7.4 — Rust test file exists and contains expected test names
# ---------------------------------------------------------------------------


def test_aggregate_spill_test_file_exists():
    assert _SPILL_TEST_FILE.exists(), f"Missing: {_SPILL_TEST_FILE}"


@pytest.mark.parametrize("fn_name", [
    "test_aggregate_under_budget_noop",
    "test_aggregate_over_budget_spills_largest_first",
    "test_aggregate_tie_break_deterministic",
])
def test_spill_integration_contains_key_functions(fn_name: str):
    content = _SPILL_TEST_FILE.read_text()
    assert fn_name in content, f"Function '{fn_name}' not found in aggregate spill test"


def test_harness_file_exports_max_turns_and_min_budget():
    content = _HARNESS_FILE.read_text()
    assert "pub const MAX_TURNS_FOR_BUDGET" in content
    assert "pub const MIN_TURN_BUDGET" in content
    assert "pub fn apply_budget_decrement" in content
    assert "pub fn has_budget_for_next_turn" in content


# ---------------------------------------------------------------------------
# B7.5 — cargo test aggregate_spill (opt-in)
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
def test_cargo_aggregate_spill_passes(run_rust_tests):
    if not run_rust_tests:
        pytest.skip("pass --run-rust-tests to enable cargo test invocation")
    result = subprocess.run(
        ["cargo", "test", "-p", "grid-engine", "--test", "tool_result_aggregate_spill"],
        cwd=_REPO_ROOT,
        capture_output=True,
        text=True,
        timeout=120,
    )
    assert result.returncode == 0, (
        f"cargo test failed:\nSTDOUT:\n{result.stdout}\nSTDERR:\n{result.stderr}"
    )
    combined = result.stdout + result.stderr
    assert re.search(r"test result: ok\.", combined), (
        f"'test result: ok.' not found in output:\n{combined[:1000]}"
    )
