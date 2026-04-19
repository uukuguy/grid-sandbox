#!/usr/bin/env bash
# Phase 3 Manual Runtime Verification Runbook
# Usage: bash scripts/phase3-runtime-verification.sh [--runtime RUNTIME] [--auto]
#
# Validates the Phase 3 sign-off criteria across all 7 L1 runtimes:
#   grid / claude-code / goose / nanobot / pydantic-ai / claw-code / ccb
#
# --runtime <name>  Run a single runtime only
# --auto            Run automated checks only (no interactive prompts); needs ≥4 PASS
#
# Sign-off requires ≥4/7 runtimes PASS (or ≥2 if --runtime used for partial sign-off).
# Log written to: phase3-verification-log.txt
#
# Mirrors: scripts/phase2_5-runtime-verification.sh (Phase 2.5)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
LOG_FILE="${REPO_ROOT}/phase3-verification-log.txt"

RUNTIMES=(grid claude-code goose nanobot pydantic-ai claw-code ccb)
AUTO_MODE=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --runtime) RUNTIMES=("$2"); shift 2 ;;
    --auto)    AUTO_MODE=true; shift ;;
    *) echo "Unknown arg: $1"; exit 1 ;;
  esac
done

echo "================================================================"
echo "  EAASP v2.0 Phase 3 — Manual Runtime Verification Runbook"
echo "  $(date '+%Y-%m-%d %H:%M:%S')"
echo "================================================================"
echo ""
echo "Runtimes: ${RUNTIMES[*]}"
echo "Log file: ${LOG_FILE}"
echo ""

# ── Step A: Automated checks (always run first) ────────────────────
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  A. Automated Checks"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

AUTO_FAIL=0

echo ""
echo "[A1] cargo check (workspace) ..."
if cargo check --workspace --quiet 2>&1; then
  echo "  ✅ cargo check clean"
else
  echo "  ❌ cargo check FAILED"
  AUTO_FAIL=$((AUTO_FAIL + 1))
fi

echo ""
echo "[A2] Contract v1.1 — all 7 runtimes (pytest tests/contract/cases/) ..."
if "${REPO_ROOT}/.venv/bin/python" -m pytest tests/contract/cases/ -q --tb=line 2>&1; then
  echo "  ✅ Contract v1.1 PASS"
else
  echo "  ❌ Contract v1.1 FAILED (check above)"
  AUTO_FAIL=$((AUTO_FAIL + 1))
fi

echo ""
echo "[A3] Phase 3 E2E B1-B8 (make v2-phase3-e2e) ..."
if make -C "${REPO_ROOT}" v2-phase3-e2e 2>&1; then
  echo "  ✅ E2E B1-B8 (112 pytest) PASS"
else
  echo "  ❌ E2E B1-B8 FAILED"
  AUTO_FAIL=$((AUTO_FAIL + 1))
fi

echo ""
echo "[A4] Phase 3 Rust integration tests (make v2-phase3-e2e-rust) ..."
if make -C "${REPO_ROOT}" v2-phase3-e2e-rust 2>&1; then
  echo "  ✅ Rust E2E (aggregate spill + compaction + retry) PASS"
else
  echo "  ❌ Rust E2E FAILED"
  AUTO_FAIL=$((AUTO_FAIL + 1))
fi

echo ""
if [[ "${AUTO_FAIL}" -eq 0 ]]; then
  echo "  ✅ All automated checks PASS"
else
  echo "  ⚠️  ${AUTO_FAIL} automated check(s) FAILED — fix before proceeding"
fi

# Reset log
{
  echo "# Phase 3 Verification Log — $(date '+%Y-%m-%d %H:%M:%S')"
  echo ""
  echo "## Automated Checks"
  if [[ "${AUTO_FAIL}" -eq 0 ]]; then
    echo "A1 cargo check: PASS"
    echo "A2 contract v1.1: PASS"
    echo "A3 E2E B1-B8: PASS"
    echo "A4 Rust E2E: PASS"
  else
    echo "AUTO_FAIL=${AUTO_FAIL}"
  fi
  echo ""
} > "${LOG_FILE}"

if "${AUTO_MODE}"; then
  echo ""
  echo "  --auto mode: skipping interactive runtime sign-off."
  if [[ "${AUTO_FAIL}" -eq 0 ]]; then
    echo "  🎉 Phase 3 automated sign-off COMPLETE"
    exit 0
  else
    echo "  ⛔ Phase 3 automated checks FAILED"
    exit 1
  fi
fi

# ── Step B: Per-runtime interactive sign-off ───────────────────────
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  B. Per-Runtime Interactive Sign-off"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

PASS_COUNT=0
FAIL_COUNT=0
SKIP_COUNT=0

for rt in "${RUNTIMES[@]}"; do
  echo ""
  echo "────────────────────────────────────────────────────────────────"
  echo "  Runtime: ${rt}"
  echo "────────────────────────────────────────────────────────────────"

  case "${rt}" in
    grid)
      echo ""
      echo "[Step 1] Start grid-runtime:"
      echo "  RUST_LOG=debug cargo run -p grid-runtime -- --port 50061"
      ;;
    claude-code)
      echo ""
      echo "[Step 1] Start claude-code-runtime:"
      echo "  cd lang/claude-code-runtime-python"
      echo "  CLAUDE_CODE_RUNTIME_GRPC_ADDR=0.0.0.0:50062 \\"
      echo "    ANTHROPIC_API_KEY=\$ANTHROPIC_API_KEY \\"
      echo "    .venv/bin/python -m claude_code_runtime"
      ;;
    goose)
      echo ""
      echo "[Step 1] Start eaasp-goose-runtime (requires goose binary + container):"
      echo "  # Option A: local binary"
      echo "  GOOSE_BIN=\$(which goose) \\"
      echo "    GOOSE_RUNTIME_GRPC_ADDR=0.0.0.0:50063 \\"
      echo "    cargo run -p eaasp-goose-runtime"
      echo ""
      echo "  # Option B: Docker container"
      echo "  make goose-runtime-container-run"
      ;;
    nanobot)
      echo ""
      echo "[Step 1] Start nanobot-runtime:"
      echo "  cd lang/nanobot-runtime-python"
      echo "  OPENAI_BASE_URL=\$OPENAI_BASE_URL \\"
      echo "    OPENAI_API_KEY=\$OPENAI_API_KEY \\"
      echo "    OPENAI_MODEL_NAME=\$OPENAI_MODEL_NAME \\"
      echo "    NANOBOT_RUNTIME_PORT=50064 \\"
      echo "    .venv/bin/python -m nanobot_runtime"
      ;;
    pydantic-ai)
      echo ""
      echo "[Step 1] Start pydantic-ai-runtime:"
      echo "  cd lang/pydantic-ai-runtime-python"
      echo "  OPENAI_BASE_URL=\$OPENAI_BASE_URL \\"
      echo "    OPENAI_API_KEY=\$OPENAI_API_KEY \\"
      echo "    OPENAI_MODEL_NAME=\$OPENAI_MODEL_NAME \\"
      echo "    PYDANTIC_AI_RUNTIME_PORT=50065 \\"
      echo "    .venv/bin/python -m pydantic_ai_runtime"
      ;;
    claw-code)
      echo ""
      echo "[Step 1] Start claw-code-runtime (Rust):"
      echo "  RUST_LOG=debug cargo run -p eaasp-claw-code-runtime -- --port 50066"
      ;;
    ccb)
      echo ""
      echo "[Step 1] Start ccb-runtime (Bun/TypeScript):"
      echo "  cd lang/ccb-runtime-ts"
      echo "  CCB_RUNTIME_PORT=50067 bun run src/main.ts"
      ;;
  esac

  echo ""
  read -r -p "Press ENTER when runtime is ready (or type 'skip' to skip): " ready_input
  if [[ "${ready_input}" == "skip" ]]; then
    echo "  ⏭  Skipped ${rt}"
    echo "## ${rt}: SKIPPED" >> "${LOG_FILE}"
    SKIP_COUNT=$((SKIP_COUNT + 1))
    continue
  fi

  case "${rt}" in
    grid)        PORT=50061 ;;
    claude-code) PORT=50062 ;;
    goose)       PORT=50063 ;;
    nanobot)     PORT=50064 ;;
    pydantic-ai) PORT=50065 ;;
    claw-code)   PORT=50066 ;;
    ccb)         PORT=50067 ;;
  esac

  echo ""
  echo "[Step 2] Run contract v1.1 against live runtime:"
  echo "  python -m pytest tests/contract/cases/ --runtime=${rt} -v --tb=short"
  echo ""
  read -r -p "Press ENTER after running contract tests: "

  echo ""
  echo "[Step 3] Run skill-extraction E2E smoke:"
  echo "  python -m pytest tests/contract/contract_v1/test_e2e_smoke.py --runtime=${rt} -v"
  echo ""
  read -r -p "Press ENTER after running skill-extraction smoke: "

  echo ""
  echo "[Step 4] Verify the following checklist:"
  cat <<'CHECKLIST'

  Contract v1.1 Checklist:
  [ ] 42 cases PASS / 22 XFAIL (no unexpected FAIL)
  [ ] hook_envelope cases: PreToolUse / PostToolUse / Stop all fire correctly

  Skill-Extraction E2E Checklist:
  [ ] TOOL_CALL events received (memory_search / memory_read / write_anchor / write_file)
  [ ] PostToolUse hook fires at least once during extraction loop
  [ ] Stop hook fires at session end
  [ ] evidence_anchor_id and draft_memory_id populated in Stop hook payload
  [ ] No ERROR-level log lines in runtime output
  [ ] gRPC status OK on Initialize / Send / Terminate

  No Failure Indicators:
  [ ] No event sequence interrupt or timeout
  [ ] No gRPC UNAVAILABLE or INTERNAL status codes
  [ ] No unexpected panics or process crashes

CHECKLIST

  echo ""
  read -r -p "Sign-off for ${rt} (y=PASS / n=FAIL / s=SKIP): " signoff

  timestamp=$(date '+%Y-%m-%dT%H:%M:%S')
  case "${signoff}" in
    y|Y|yes|YES)
      echo "  ✅ PASS — ${rt}"
      echo "## ${rt}: PASS (signed off at ${timestamp})" >> "${LOG_FILE}"
      PASS_COUNT=$((PASS_COUNT + 1))
      ;;
    n|N|no|NO)
      echo "  ❌ FAIL — ${rt}"
      read -r -p "  Brief failure note: " fail_note
      echo "## ${rt}: FAIL — ${fail_note} (at ${timestamp})" >> "${LOG_FILE}"
      FAIL_COUNT=$((FAIL_COUNT + 1))
      ;;
    *)
      echo "  ⏭  SKIP — ${rt}"
      echo "## ${rt}: SKIPPED (at ${timestamp})" >> "${LOG_FILE}"
      SKIP_COUNT=$((SKIP_COUNT + 1))
      ;;
  esac
done

echo ""
echo "================================================================"
echo "  Verification Summary"
echo "================================================================"
echo "  PASS:  ${PASS_COUNT} / ${#RUNTIMES[@]}"
echo "  FAIL:  ${FAIL_COUNT}"
echo "  SKIP:  ${SKIP_COUNT}"
echo ""
echo "  Log: ${LOG_FILE}"
echo ""
cat "${LOG_FILE}"
echo ""

REQUIRED_PASS=4
if [[ "${#RUNTIMES[@]}" -lt 7 ]]; then
  REQUIRED_PASS=2
fi

if [[ "${PASS_COUNT}" -ge "${REQUIRED_PASS}" && "${FAIL_COUNT}" -eq 0 ]]; then
  echo "  🎉 Phase 3 sign-off COMPLETE (${PASS_COUNT}/${#RUNTIMES[@]} runtimes PASS)"
  exit 0
elif [[ "${FAIL_COUNT}" -gt 0 ]]; then
  echo "  ⛔ Verification FAILED — fix ${FAIL_COUNT} failure(s) before closing Phase 3"
  exit 1
else
  echo "  ⚠️  Not enough runtimes signed off (need ≥${REQUIRED_PASS} PASS, got ${PASS_COUNT})"
  exit 2
fi
