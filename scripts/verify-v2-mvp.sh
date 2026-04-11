#!/bin/bash
# verify-v2-mvp.sh — EAASP v2.0 Phase 0 MVP end-to-end exit gate (4b-lite).
#
# Starts the full v2 stack:
#   L2 memory-engine  :18085
#   L3 governance     :18083
#   L4 orchestration  :18084
#   skill-registry    :18081
#   grid-runtime      :50051   (skippable)
#   claude-code-runtime :50052 (skippable)
#
# Then runs scripts/verify-v2-mvp.py with 15 assertions in L4-stubbed mode
# (see ADR-V2-004). Cleans up all services via a trap on EXIT.
#
# Usage:
#   ./scripts/verify-v2-mvp.sh
#   ./scripts/verify-v2-mvp.sh --skip-build      # skip cargo build
#   ./scripts/verify-v2-mvp.sh --skip-runtimes   # skip L1 runtimes (L2/L3/L4 only)

set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# ── Port assignments (must match verify-v2-mvp.py) ─────────────────────────
L2_MEM_PORT=18085
L3_GOV_PORT=18083
L4_ORCH_PORT=18084
SKILL_REG_PORT=18081
GRID_RUNTIME_PORT=50051
CLAUDE_RUNTIME_PORT=50052

# ── Runtime flags ──────────────────────────────────────────────────────────
SKIP_BUILD=false
SKIP_RUNTIMES=false

# ── Background PIDs (for cleanup) ──────────────────────────────────────────
L2_PID=""
L3_PID=""
L4_PID=""
SKILL_REG_PID=""
GRID_PID=""
CLAUDE_PID=""

_kill_tree() {
    local name=$1
    local pid=$2
    [ -z "$pid" ] && return 0
    # Recurse into children first (uvicorn auto-spawns workers, certifier
    # forks cargo, etc.). pkill -P sends SIGTERM to direct children; lsof
    # by port is the belt-and-braces fallback for orphaned listeners.
    pkill -TERM -P "$pid" 2>/dev/null || true
    kill -TERM "$pid" 2>/dev/null && echo "  Stopped $name (PID $pid)" || true
}

_kill_port() {
    local name=$1
    local port=$2
    local stragglers
    stragglers=$(lsof -nP -iTCP:"$port" -sTCP:LISTEN -t 2>/dev/null || true)
    if [ -n "$stragglers" ]; then
        echo "  Reaping $name leftover listeners on :$port: $stragglers"
        kill -TERM $stragglers 2>/dev/null || true
        sleep 0.3
        kill -KILL $stragglers 2>/dev/null || true
    fi
}

cleanup() {
    echo ""
    echo "=== Cleaning up ==="
    _kill_tree "claude-code-runtime" "$CLAUDE_PID"
    _kill_tree "grid-runtime" "$GRID_PID"
    _kill_tree "skill-registry" "$SKILL_REG_PID"
    _kill_tree "L4 orchestration" "$L4_PID"
    _kill_tree "L3 governance" "$L3_PID"
    _kill_tree "L2 memory-engine" "$L2_PID"
    # Sweep any orphaned listeners that escaped the parent->child reap.
    _kill_port "claude-code-runtime" "$CLAUDE_RUNTIME_PORT"
    _kill_port "grid-runtime" "$GRID_RUNTIME_PORT"
    _kill_port "skill-registry" "$SKILL_REG_PORT"
    _kill_port "L4 orchestration" "$L4_ORCH_PORT"
    _kill_port "L3 governance" "$L3_GOV_PORT"
    _kill_port "L2 memory-engine" "$L2_MEM_PORT"
}
trap cleanup EXIT INT TERM

# ── Arg parsing ────────────────────────────────────────────────────────────
for arg in "$@"; do
    case "$arg" in
        --skip-build) SKIP_BUILD=true ;;
        --skip-runtimes) SKIP_RUNTIMES=true ;;
        -h|--help)
            cat <<EOF
Usage: $0 [--skip-build] [--skip-runtimes]

  --skip-build      Skip 'cargo build --release' step.
  --skip-runtimes   Skip starting grid-runtime + claude-code-runtime
                    (only L2/L3/L4/skill-registry — assertion 15 will
                    report the runtimes as unreachable).

Runs scripts/verify-v2-mvp.py with 15 assertions in L4-stubbed mode.
See ADR-V2-004 for why some assertions bypass the real L1 LLM tool-call loop.
EOF
            exit 0
            ;;
        *)
            echo "Unknown flag: $arg" >&2
            exit 1
            ;;
    esac
done

# ── Pre-flight .venv checks ────────────────────────────────────────────────
check_venv() {
    local svc_dir=$1
    local make_target=$2
    if [ ! -x "$PROJECT_ROOT/$svc_dir/.venv/bin/python" ]; then
        echo "ERROR: $svc_dir/.venv is missing. Run: make $make_target" >&2
        return 1
    fi
}

# ── Pre-flight port collision check ────────────────────────────────────────
# Without this guard, a stale uvicorn from a prior interrupted run will
# silently keep the port and the new wait_for_port "succeeds" against the
# stale process — every assertion then talks to a server with stale state.
check_port_free() {
    local port=$1
    local name=$2
    local holder
    holder=$(lsof -nP -iTCP:"$port" -sTCP:LISTEN -t 2>/dev/null | head -1 || true)
    if [ -n "$holder" ]; then
        echo "ERROR: port $port ($name) already in use by PID $holder." >&2
        echo "       Run: lsof -nP -iTCP:$port -sTCP:LISTEN, then kill the holder." >&2
        return 1
    fi
}

echo "════════════════════════════════════════════════════"
echo "  EAASP v2.0 MVP E2E Verification (4b-lite)"
echo "  Mode: L4-stubbed (ADR-V2-004)"
echo "════════════════════════════════════════════════════"
echo ""

echo "=== Pre-flight: port availability ==="
check_port_free $L2_MEM_PORT "L2 memory-engine"
check_port_free $L3_GOV_PORT "L3 governance"
check_port_free $L4_ORCH_PORT "L4 orchestration"
check_port_free $SKILL_REG_PORT "skill-registry"
if [ "$SKIP_RUNTIMES" = false ]; then
    check_port_free $GRID_RUNTIME_PORT "grid-runtime"
    check_port_free $CLAUDE_RUNTIME_PORT "claude-code-runtime"
fi
echo "  All ports free."
echo ""

echo "=== Pre-flight: Python .venv checks ==="
check_venv "tools/eaasp-l2-memory-engine" "l2-memory-setup"
check_venv "tools/eaasp-l3-governance" "l3-setup"
check_venv "tools/eaasp-l4-orchestration" "l4-setup"
check_venv "tools/eaasp-cli-v2" "cli-v2-setup"
echo "  All .venvs present."
echo ""

# ── Check ANTHROPIC_API_KEY for claude-code-runtime ────────────────────────
if [ "$SKIP_RUNTIMES" = false ] && [ -z "${ANTHROPIC_API_KEY:-}" ]; then
    if [ -f "$PROJECT_ROOT/.env" ] && grep -q '^ANTHROPIC_API_KEY=' "$PROJECT_ROOT/.env"; then
        echo "  ANTHROPIC_API_KEY not in shell env but present in .env (claude-code-runtime reads .env)"
    else
        echo "  WARNING: ANTHROPIC_API_KEY not set — forcing --skip-runtimes"
        echo "           Assertion 15 will report both L1 runtimes as unreachable."
        SKIP_RUNTIMES=true
    fi
fi

# ── Wipe stale verify-v2 SQLite state (reviewer M2) ────────────────────────
# Assertion 11 checks that session-2 P3 memory_refs are non-empty. If prior
# runs left rows in data/verify-v2-l2.db, a regression could still appear to
# satisfy the assertion via stale data. Wipe all three service DBs before
# starting so every run is deterministic.
echo "=== Wiping stale verify-v2 state ==="
rm -f "$PROJECT_ROOT/data/verify-v2-l2.db" \
      "$PROJECT_ROOT/data/verify-v2-l3.db" \
      "$PROJECT_ROOT/data/verify-v2-l4.db" \
      "$PROJECT_ROOT/data/verify-v2-l2.db-shm" \
      "$PROJECT_ROOT/data/verify-v2-l2.db-wal" \
      "$PROJECT_ROOT/data/verify-v2-l3.db-shm" \
      "$PROJECT_ROOT/data/verify-v2-l3.db-wal" \
      "$PROJECT_ROOT/data/verify-v2-l4.db-shm" \
      "$PROJECT_ROOT/data/verify-v2-l4.db-wal"
mkdir -p "$PROJECT_ROOT/data"
echo "  State wiped."
echo ""

# ── Step 1: Cargo build ────────────────────────────────────────────────────
if [ "$SKIP_BUILD" = false ]; then
    echo "=== Building Rust binaries (release) ==="
    cd "$PROJECT_ROOT"
    cargo build --release \
        -p grid-runtime \
        -p eaasp-skill-registry \
        -p eaasp-certifier 2>&1 | tail -5
    echo "  Build complete."
    echo ""
else
    echo "=== Skipping Rust build (--skip-build) ==="
    echo ""
fi

# ── wait_for_port helper ───────────────────────────────────────────────────
wait_for_port() {
    local port=$1
    local name=$2
    local max_wait=15
    local waited=0

    echo "  Waiting for $name on :${port}..."
    while ! nc -z 127.0.0.1 "$port" 2>/dev/null; do
        sleep 1
        waited=$((waited + 1))
        if [ $waited -ge $max_wait ]; then
            echo "  ERROR: $name did not start within ${max_wait}s" >&2
            return 1
        fi
    done
    echo "  $name is ready (${waited}s)"
}

# ── Step 2: Start L2 memory-engine ─────────────────────────────────────────
echo "=== Starting L2 memory-engine on :${L2_MEM_PORT} ==="
cd "$PROJECT_ROOT/tools/eaasp-l2-memory-engine"
EAASP_L2_PORT=$L2_MEM_PORT \
EAASP_L2_HOST=127.0.0.1 \
EAASP_L2_DB_PATH="$PROJECT_ROOT/data/verify-v2-l2.db" \
    .venv/bin/python -m eaasp_l2_memory_engine.main 2>&1 | sed 's/^/  [l2-mem]    /' &
L2_PID=$!
echo "  PID: $L2_PID"
wait_for_port $L2_MEM_PORT "L2 memory-engine"

# ── Step 3: Start L3 governance ────────────────────────────────────────────
echo ""
echo "=== Starting L3 governance on :${L3_GOV_PORT} ==="
cd "$PROJECT_ROOT/tools/eaasp-l3-governance"
EAASP_L3_PORT=$L3_GOV_PORT \
EAASP_L3_HOST=127.0.0.1 \
EAASP_L3_DB_PATH="$PROJECT_ROOT/data/verify-v2-l3.db" \
    .venv/bin/python -m eaasp_l3_governance.main 2>&1 | sed 's/^/  [l3-gov]    /' &
L3_PID=$!
echo "  PID: $L3_PID"
wait_for_port $L3_GOV_PORT "L3 governance"

# ── Step 4: Start L4 orchestration ─────────────────────────────────────────
echo ""
echo "=== Starting L4 orchestration on :${L4_ORCH_PORT} ==="
cd "$PROJECT_ROOT/tools/eaasp-l4-orchestration"
EAASP_L4_PORT=$L4_ORCH_PORT \
EAASP_L4_HOST=127.0.0.1 \
EAASP_L4_DB_PATH="$PROJECT_ROOT/data/verify-v2-l4.db" \
EAASP_L2_URL="http://127.0.0.1:${L2_MEM_PORT}" \
EAASP_L3_URL="http://127.0.0.1:${L3_GOV_PORT}" \
    .venv/bin/python -m eaasp_l4_orchestration.main 2>&1 | sed 's/^/  [l4-orch]   /' &
L4_PID=$!
echo "  PID: $L4_PID"
wait_for_port $L4_ORCH_PORT "L4 orchestration"

# ── Step 5: Start skill-registry (Rust) ────────────────────────────────────
echo ""
echo "=== Starting skill-registry on :${SKILL_REG_PORT} ==="
cd "$PROJECT_ROOT"
EAASP_SKILL_REGISTRY_PORT=$SKILL_REG_PORT \
EAASP_SKILL_REGISTRY_HOST=127.0.0.1 \
    ./target/release/eaasp-skill-registry \
        --data-dir "$PROJECT_ROOT/data/verify-v2-skill-registry" 2>&1 | sed 's/^/  [skill-reg] /' &
SKILL_REG_PID=$!
echo "  PID: $SKILL_REG_PID"
wait_for_port $SKILL_REG_PORT "skill-registry"

# ── Step 6 (optional): Start L1 runtimes ───────────────────────────────────
if [ "$SKIP_RUNTIMES" = false ]; then
    echo ""
    echo "=== Starting grid-runtime on :${GRID_RUNTIME_PORT} ==="
    cd "$PROJECT_ROOT"
    GRID_RUNTIME_PORT=$GRID_RUNTIME_PORT \
    RUST_LOG=grid_runtime=info \
        ./target/release/grid-runtime 2>&1 | sed 's/^/  [grid-rt]   /' &
    GRID_PID=$!
    echo "  PID: $GRID_PID"
    wait_for_port $GRID_RUNTIME_PORT "grid-runtime"

    echo ""
    echo "=== Starting claude-code-runtime on :${CLAUDE_RUNTIME_PORT} ==="
    cd "$PROJECT_ROOT/lang/claude-code-runtime-python"
    uv run python -m claude_code_runtime \
        --port "$CLAUDE_RUNTIME_PORT" \
        --env-file "$PROJECT_ROOT/.env" \
        --log-level INFO 2>&1 | sed 's/^/  [claude-rt] /' &
    CLAUDE_PID=$!
    echo "  PID: $CLAUDE_PID"
    wait_for_port $CLAUDE_RUNTIME_PORT "claude-code-runtime"
else
    echo ""
    echo "=== Skipping L1 runtimes (--skip-runtimes) ==="
fi

# ── Step 7: Run verify script ──────────────────────────────────────────────
echo ""
echo "=== Running scripts/verify-v2-mvp.py ==="
cd "$PROJECT_ROOT"

VERIFY_EXIT=0
EAASP_VERIFY_MODE=l4-stubbed \
EAASP_L2_URL="http://127.0.0.1:${L2_MEM_PORT}" \
EAASP_L3_URL="http://127.0.0.1:${L3_GOV_PORT}" \
EAASP_L4_URL="http://127.0.0.1:${L4_ORCH_PORT}" \
EAASP_SKILL_REGISTRY_URL="http://127.0.0.1:${SKILL_REG_PORT}" \
EAASP_GRID_RUNTIME_URL="http://127.0.0.1:${GRID_RUNTIME_PORT}" \
EAASP_CLAUDE_RUNTIME_URL="http://127.0.0.1:${CLAUDE_RUNTIME_PORT}" \
EAASP_SKIP_RUNTIMES="$SKIP_RUNTIMES" \
    "$PROJECT_ROOT/tools/eaasp-l4-orchestration/.venv/bin/python" \
    "$PROJECT_ROOT/scripts/verify-v2-mvp.py" || VERIFY_EXIT=$?

echo ""
echo "════════════════════════════════════════════════════"
if [ $VERIFY_EXIT -eq 0 ]; then
    echo "  PASS — all 15 assertions green"
else
    echo "  FAIL — verify-v2-mvp.py exited $VERIFY_EXIT"
fi
echo "════════════════════════════════════════════════════"

exit $VERIFY_EXIT
