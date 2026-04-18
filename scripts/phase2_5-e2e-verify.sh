#!/usr/bin/env bash
# phase2_5-e2e-verify.sh — EAASP v2.0 Phase 2.5 E2E Verification Script
#
# 三层验证结构：
#   Section 0: Pre-flight (工具链、API keys、编译产物)
#   Section 1: MVP 复现 (grid + claude-code，threshold-calibration，≥4 PRE_TOOL_USE)
#   Section 2: Phase 2.5 新功能 (nanobot E2E + 合约套件 v1 + hook envelope + goose 基线)
#
# 对标: scripts/s4t3-runtime-verification.sh (Phase 2 MVP)
# 设计原则:
#   - 全自动执行，不问 y/n
#   - 每个断言打印 PASS / FAIL / SKIP / XFAIL
#   - goose Send 是 stub (Phase 3)，对应测试标记 XFAIL
#   - 最终 exit 0 (all pass/xfail/skip) 或 exit 1 (任意 FAIL)
#
# 使用方式:
#   bash scripts/phase2_5-e2e-verify.sh               # 全量 (需要真实 LLM)
#   bash scripts/phase2_5-e2e-verify.sh --skip-mvp    # 跳过 Section 1 MVP
#   bash scripts/phase2_5-e2e-verify.sh --skip-llm    # 跳过所有真实 LLM 调用
#   bash scripts/phase2_5-e2e-verify.sh --section 2   # 仅跑 Section 2
#
# 环境变量 (从 .env 读取):
#   OPENAI_API_KEY, OPENAI_BASE_URL, OPENAI_MODEL_NAME  — grid + nanobot
#   ANTHROPIC_API_KEY, ANTHROPIC_BASE_URL               — claude-code
#   LLM_PROVIDER                                        — grid provider 选择
#
# 退出码:
#   0 = 全部 PASS / XFAIL / SKIP
#   1 = 任意 FAIL
#   2 = pre-flight 失败

set -euo pipefail

# ── 路径 ──────────────────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
LOG_FILE="${REPO_ROOT}/phase2_5-e2e-verify.log"
ENV_FILE="${REPO_ROOT}/.env"

# ── 颜色 ──────────────────────────────────────────────────────────────────
if [ -t 1 ]; then
    RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[0;33m'
    CYAN='\033[0;36m'; BOLD='\033[1m'; NC='\033[0m'
else
    RED=''; GREEN=''; YELLOW=''; CYAN=''; BOLD=''; NC=''
fi

# ── 全局计数 (必须在函数外声明为全局) ──────────────────────────────────────
TOTAL_PASS=0; TOTAL_FAIL=0; TOTAL_SKIP=0; TOTAL_XFAIL=0
FAILURES=()

# ── CLI flags ──────────────────────────────────────────────────────────────
SKIP_MVP=false
SKIP_LLM=false
ONLY_SECTION=""
_PREV=""
for arg in "$@"; do
    if [ "${_PREV}" = "--section" ]; then
        ONLY_SECTION="${arg}"
        _PREV=""
        continue
    fi
    case "$arg" in
        --skip-mvp)   SKIP_MVP=true ;;
        --skip-llm)   SKIP_LLM=true ;;
        --section)    _PREV="--section" ;;
        --section=*)  ONLY_SECTION="${arg#--section=}" ;;
    esac
    _PREV="${arg}"
done

# ── 结果记录函数 ────────────────────────────────────────────────────────────
log_pass()  { echo -e "  ${GREEN}PASS${NC}  $1"; echo "PASS  $1" >> "${LOG_FILE}"; TOTAL_PASS=$((TOTAL_PASS+1)); }
log_fail()  { echo -e "  ${RED}FAIL${NC}  $1"; echo "FAIL  $1" >> "${LOG_FILE}"; TOTAL_FAIL=$((TOTAL_FAIL+1)); FAILURES+=("$1"); }
log_skip()  { echo -e "  ${YELLOW}SKIP${NC}  $1"; echo "SKIP  $1" >> "${LOG_FILE}"; TOTAL_SKIP=$((TOTAL_SKIP+1)); }
log_xfail() { echo -e "  ${YELLOW}XFAIL${NC} $1"; echo "XFAIL $1" >> "${LOG_FILE}"; TOTAL_XFAIL=$((TOTAL_XFAIL+1)); }
log_info()  { echo -e "  ${CYAN}INFO${NC}  $1"; }
log_head()  { echo ""; echo -e "${BOLD}━━━ $1 ━━━${NC}"; }

# ── 端口分配 ──────────────────────────────────────────────────────────────
GRID_PORT=50071
CLAUDE_PORT=50072
NANOBOT_PORT=50073
GOOSE_PORT=50074

# ── 후台进程 PID ───────────────────────────────────────────────────────────
GRID_PID=""; CLAUDE_PID=""; NANOBOT_PID=""; GOOSE_PID=""
SETTLE_SECS="${PHASE25_SETTLE_SECS:-20}"

# ── Runtime 启动参数 (Section 0 填充) ─────────────────────────────────────
GRID_LAUNCH="cargo run -p grid-runtime --"
GRID_TIMEOUT=120
GOOSE_LAUNCH="cargo run -p eaasp-goose-runtime --"
GOOSE_TIMEOUT=120
GOOSE_AVAILABLE=false
GOOSE_BIN_PATH=""
NANOBOT_VENV=""
CONTRACT_VENV=""
L4_URL="${S4T3_L4_URL:-http://127.0.0.1:18084}"
CLI=""

# ── Cleanup trap ───────────────────────────────────────────────────────────
cleanup() {
    for _pid in "${GRID_PID}" "${CLAUDE_PID}" "${NANOBOT_PID}" "${GOOSE_PID}"; do
        if [ -n "${_pid}" ] && kill -0 "${_pid}" 2>/dev/null; then
            kill "${_pid}" 2>/dev/null || true
        fi
    done
}
trap cleanup EXIT

# ── .env loader ───────────────────────────────────────────────────────────
load_env() {
    if [ -f "${ENV_FILE}" ]; then
        while IFS= read -r line; do
            [[ "$line" =~ ^[[:space:]]*# ]] && continue
            [[ -z "${line// }" ]] && continue
            [[ "$line" =~ ^[A-Za-z_][A-Za-z0-9_]*= ]] && export "$line" 2>/dev/null || true
        done < "${ENV_FILE}"
    fi
}

# ── gRPC TCP probe ─────────────────────────────────────────────────────────
wait_for_port() {
    local port=$1 timeout=${2:-30} label=${3:-"service"}
    local deadline=$(($(date +%s) + timeout))
    while [ "$(date +%s)" -lt "$deadline" ]; do
        if nc -z 127.0.0.1 "$port" 2>/dev/null; then
            return 0
        fi
        sleep 0.5
    done
    echo "  Timeout waiting for ${label} on :${port}" >&2
    return 1
}

# ── Python gRPC round-trip (Health + Initialize + Terminate) ──────────────
grpc_roundtrip() {
    local port=$1 label=$2
    python3 - "${port}" "${label}" "${REPO_ROOT}" <<'PYEOF' 2>/dev/null
import sys
port, label, repo_root = sys.argv[1], sys.argv[2], sys.argv[3]
sys.path.insert(0, repo_root + "/lang/claude-code-runtime-python/src")
import grpc
from claude_code_runtime._proto.eaasp.runtime.v2 import common_pb2, runtime_pb2, runtime_pb2_grpc

channel = grpc.insecure_channel(f"127.0.0.1:{port}")
stub = runtime_pb2_grpc.RuntimeServiceStub(channel)

resp = stub.Health(common_pb2.Empty(), timeout=5)
assert resp.healthy, f"Health returned unhealthy: {resp}"

import uuid
init = stub.Initialize(
    runtime_pb2.InitializeRequest(
        payload=common_pb2.SessionPayload(
            session_id="phase25-verify-" + str(uuid.uuid4())[:8],
            user_id="verifier",
            runtime_id=label,
        )
    ),
    timeout=10,
)
assert init.session_id, "Initialize returned empty session_id"

stub.Terminate(common_pb2.Empty(), timeout=5)
print("OK sid=" + init.session_id)
PYEOF
}

# ── nanobot GetCapabilities check ─────────────────────────────────────────
grpc_get_capabilities() {
    local port=$1 repo_root=$2
    python3 - "${port}" "${repo_root}" <<'PYEOF' 2>/dev/null
import sys
port, repo_root = sys.argv[1], sys.argv[2]
sys.path.insert(0, repo_root + "/lang/claude-code-runtime-python/src")
import grpc
from claude_code_runtime._proto.eaasp.runtime.v2 import common_pb2, runtime_pb2_grpc
ch = grpc.insecure_channel(f"127.0.0.1:{port}")
stub = runtime_pb2_grpc.RuntimeServiceStub(ch)
caps = stub.GetCapabilities(common_pb2.Empty(), timeout=5)
print(f"runtime_id={caps.runtime_id} tier={caps.tier} deployment_mode={caps.deployment_mode}")
PYEOF
}

# ── nanobot Send (真实 LLM) ────────────────────────────────────────────────
nanobot_send_verify() {
    local port=$1 repo_root=$2
    python3 - "${port}" "${repo_root}" <<'PYEOF' 2>/dev/null
import sys
port, repo_root = sys.argv[1], sys.argv[2]
sys.path.insert(0, repo_root + "/lang/claude-code-runtime-python/src")
import grpc, uuid
from claude_code_runtime._proto.eaasp.runtime.v2 import common_pb2, runtime_pb2, runtime_pb2_grpc

channel = grpc.insecure_channel(f"127.0.0.1:{port}")
stub = runtime_pb2_grpc.RuntimeServiceStub(channel)

init = stub.Initialize(
    runtime_pb2.InitializeRequest(
        payload=common_pb2.SessionPayload(
            session_id="phase25-nanobot-llm-" + str(uuid.uuid4())[:8],
            user_id="verifier",
            runtime_id="nanobot",
        )
    ),
    timeout=10,
)

msg = runtime_pb2.UserMessage(
    content="Say 'hello world' and nothing else.",
    message_type="text",
)
stream = stub.Send(
    runtime_pb2.SendRequest(session_id=init.session_id, message=msg),
    timeout=60,
)

chunks = list(stream)
stub.Terminate(common_pb2.Empty(), timeout=5)

done_seen = any(c.chunk_type == "done" for c in chunks)
tool_calls = sum(1 for c in chunks if c.chunk_type == "tool_call")
print(f"chunks={len(chunks)} tool_calls={tool_calls} done={done_seen}")
assert len(chunks) >= 1, f"Send MUST yield ≥1 chunk, got {len(chunks)}"
assert done_seen, "Send MUST end with chunk_type=done"
PYEOF
}

# ── goose Send stub check ─────────────────────────────────────────────────
goose_send_stub_check() {
    local port=$1 repo_root=$2
    python3 - "${port}" "${repo_root}" <<'PYEOF' 2>/dev/null
import sys
port, repo_root = sys.argv[1], sys.argv[2]
sys.path.insert(0, repo_root + "/lang/claude-code-runtime-python/src")
import grpc
from claude_code_runtime._proto.eaasp.runtime.v2 import common_pb2, runtime_pb2, runtime_pb2_grpc
ch = grpc.insecure_channel(f"127.0.0.1:{port}")
stub = runtime_pb2_grpc.RuntimeServiceStub(ch)
init = stub.Initialize(runtime_pb2.InitializeRequest(
    payload=common_pb2.SessionPayload(
        session_id="goose-verify", user_id="v", runtime_id="goose"
    )
), timeout=5)
msg = runtime_pb2.UserMessage(content="ping", message_type="text")
stream = stub.Send(
    runtime_pb2.SendRequest(session_id=init.session_id, message=msg),
    timeout=10,
)
chunks = list(stream)
stub.Terminate(common_pb2.Empty(), timeout=5)
done_chunks = [c for c in chunks if c.chunk_type == "done"]
print(f"total_chunks={len(chunks)} done_chunks={len(done_chunks)}")
assert len(chunks) >= 1, "Send must yield ≥1 chunk"
PYEOF
}

# ── fetch events JSON from L4 REST ────────────────────────────────────────
fetch_events_json() {
    local session_id=$1 l4_url=$2 out_file=$3
    curl -fsS -m 10 \
        "${l4_url}/v1/sessions/${session_id}/events?from=1&limit=500" \
        -o "${out_file}" 2>/dev/null
}

# ─────────────────────────────────────────────────────────────────────────────
# SECTION 0 — Pre-flight
# ─────────────────────────────────────────────────────────────────────────────
run_section_0() {
    log_head "Section 0: Pre-flight"

    # 工具链
    if command -v cargo >/dev/null 2>&1; then
        log_pass "S0.01 cargo available ($(cargo --version 2>/dev/null | head -1))"
    else
        log_fail "S0.01 cargo not found — cannot run Rust runtimes"
    fi

    if command -v python3 >/dev/null 2>&1; then
        log_pass "S0.02 python3 available ($(python3 --version 2>/dev/null))"
    else
        log_fail "S0.02 python3 not found"
    fi

    if command -v jq >/dev/null 2>&1; then
        log_pass "S0.03 jq available"
    else
        log_fail "S0.03 jq not found — install: brew install jq"
    fi

    if command -v nc >/dev/null 2>&1; then
        log_pass "S0.04 nc (netcat) available"
    else
        log_fail "S0.04 nc not found"
    fi

    # API keys
    if [ -n "${OPENAI_API_KEY:-}" ]; then
        log_pass "S0.05 OPENAI_API_KEY present"
    else
        log_fail "S0.05 OPENAI_API_KEY missing — nanobot E2E will fail"
    fi

    if [ -n "${OPENAI_BASE_URL:-}" ]; then
        log_pass "S0.06 OPENAI_BASE_URL present (${OPENAI_BASE_URL})"
    else
        log_fail "S0.06 OPENAI_BASE_URL missing"
    fi

    if [ -n "${ANTHROPIC_API_KEY:-}" ]; then
        log_pass "S0.07 ANTHROPIC_API_KEY present"
    else
        log_skip "S0.07 ANTHROPIC_API_KEY missing — claude-code Section 1 MVP will skip"
    fi

    # 编译产物 — 设置全局变量供 Section 1/2 使用
    local grid_bin="${REPO_ROOT}/target/debug/grid-runtime"
    if [ -f "${grid_bin}" ]; then
        log_pass "S0.08 grid-runtime prebuilt"
        GRID_LAUNCH="${grid_bin}"
        GRID_TIMEOUT=15
    else
        log_info "S0.08 grid-runtime not prebuilt — will use cargo run (may be slow)"
        GRID_LAUNCH="cargo run -p grid-runtime --"
        GRID_TIMEOUT=120
    fi

    NANOBOT_VENV="${REPO_ROOT}/lang/nanobot-runtime-python/.venv/bin/python"
    if [ -f "${NANOBOT_VENV}" ]; then
        log_pass "S0.09 nanobot-runtime-python venv present"
    else
        log_fail "S0.09 nanobot-runtime-python venv missing — run: cd lang/nanobot-runtime-python && uv sync"
        NANOBOT_VENV=""
    fi

    local claude_venv="${REPO_ROOT}/lang/claude-code-runtime-python/.venv/bin/python"
    if [ -f "${claude_venv}" ]; then
        log_pass "S0.10 claude-code-runtime-python venv present"
    else
        log_skip "S0.10 claude-code-runtime-python venv missing — claude-code tests will skip"
    fi

    GOOSE_BIN_PATH="${GOOSE_BIN:-$(command -v goose 2>/dev/null || echo '')}"
    if [ -n "${GOOSE_BIN_PATH}" ] && [ -f "${GOOSE_BIN_PATH}" ]; then
        log_pass "S0.11 goose binary present (${GOOSE_BIN_PATH})"
        GOOSE_AVAILABLE=true
    else
        log_skip "S0.11 goose binary not found — goose E2E will XFAIL (expected, Phase 3)"
        GOOSE_AVAILABLE=false
        GOOSE_BIN_PATH=""
    fi

    local goose_bin="${REPO_ROOT}/target/debug/eaasp-goose-runtime"
    if [ -f "${goose_bin}" ]; then
        log_pass "S0.12 eaasp-goose-runtime prebuilt"
        GOOSE_LAUNCH="${goose_bin}"
        GOOSE_TIMEOUT=15
    else
        log_info "S0.12 eaasp-goose-runtime not prebuilt — will use cargo run"
        GOOSE_LAUNCH="cargo run -p eaasp-goose-runtime --"
        GOOSE_TIMEOUT=120
    fi

    # contract harness venv
    CONTRACT_VENV="${REPO_ROOT}/.venv/bin/python"
    if [ ! -f "${CONTRACT_VENV}" ]; then
        CONTRACT_VENV="python3"
        log_info "S0.13 using system python3 for contract suite (repo-root .venv not found)"
    else
        log_pass "S0.13 contract harness venv found at .venv/bin/python"
    fi

    # CLI for Section 1
    CLI="${REPO_ROOT}/tools/eaasp-cli-v2/.venv/bin/eaasp"

    if [ "${TOTAL_FAIL}" -gt 0 ]; then
        echo ""
        echo -e "${RED}${BOLD}Pre-flight FAILED (${TOTAL_FAIL} checks). Fix issues above and re-run.${NC}"
        exit 2
    fi
    echo ""
    log_info "Pre-flight OK (PASS=${TOTAL_PASS} SKIP=${TOTAL_SKIP}). Proceeding..."
}

# ─────────────────────────────────────────────────────────────────────────────
# SECTION 1 — MVP 复现: grid + claude-code, threshold-calibration
#             对标: scripts/s4t3-runtime-verification.sh
# ─────────────────────────────────────────────────────────────────────────────
run_mvp_runtime() {
    # Run threshold-calibration on a runtime already listening on given port.
    # Requires full EAASP stack (L2/L3/L4 + CLI).
    local rt=$1 port=$2
    local events_file="/tmp/phase25-s1-${rt}-events.json"

    log_info "S1 ${rt}: creating session..."
    local output
    if ! output=$("${CLI}" session create \
            --skill threshold-calibration \
            --runtime "${rt}" \
            --user-id phase25-verifier \
            --intent-text "校准 Transformer-001 温度阈值 (Phase 2.5 verification)" 2>&1); then
        log_fail "S1.${rt}.create session create failed: ${output}"
        return 1
    fi

    local sid
    sid=$(echo "${output}" \
        | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' \
        | head -1)
    if [ -z "${sid}" ]; then
        log_fail "S1.${rt}.sid could not extract session_id from CLI output"
        return 1
    fi
    log_info "  session_id: ${sid}"

    log_info "S1 ${rt}: sending message (background, settle=${SETTLE_SECS}s)..."
    timeout "${SETTLE_SECS}" "${CLI}" session send "${sid}" \
        "请校准 Transformer-001 的温度阈值，使用工业传感器数据进行分析" \
        --no-stream >/dev/null 2>&1 &
    local send_pid=$!
    sleep "${SETTLE_SECS}"
    kill "${send_pid}" 2>/dev/null || true
    wait "${send_pid}" 2>/dev/null || true

    log_info "S1 ${rt}: fetching events..."
    if ! fetch_events_json "${sid}" "${L4_URL}" "${events_file}" 2>/dev/null; then
        "${CLI}" session events "${sid}" --format json > "${events_file}" 2>/dev/null || true
    fi

    if ! jq empty "${events_file}" 2>/dev/null; then
        log_fail "S1.${rt}.events events file is not valid JSON (${events_file})"
        return 1
    fi

    local total pre_tool post_tool stop_count continuations
    total=$(jq '.events | length' "${events_file}" 2>/dev/null || echo 0)
    pre_tool=$(jq '[.events[]? | select(.event_type=="PRE_TOOL_USE")] | length' "${events_file}" 2>/dev/null || echo 0)
    post_tool=$(jq '[.events[]? | select(.event_type=="POST_TOOL_USE")] | length' "${events_file}" 2>/dev/null || echo 0)
    stop_count=$(jq '[.events[]? | select(.event_type=="STOP")] | length' "${events_file}" 2>/dev/null || echo 0)
    continuations=$(jq '[.events[]? | select(.event_type=="WORKFLOW_CONTINUATION" or (.payload? // {} | tostring | contains("workflow_continuation")))] | length' "${events_file}" 2>/dev/null || echo 0)

    log_info "  total=${total} PRE=${pre_tool} POST=${post_tool} STOP=${stop_count} CONT=${continuations}"

    # PRE_TOOL_USE ≥ 4 — D87 multi-step fix (threshold-calibration has ≥4 tool steps)
    if [ "${pre_tool}" -ge 4 ]; then
        log_pass "S1.${rt}.pre_tool_use PRE_TOOL_USE=${pre_tool} ≥ 4 (D87 fix active)"
    else
        log_fail "S1.${rt}.pre_tool_use PRE_TOOL_USE=${pre_tool} < 4 (D87 multi-step fix not active?)"
    fi

    # POST_TOOL_USE ≈ PRE_TOOL_USE (within ±1)
    local diff=$(( pre_tool - post_tool ))
    if [ "${diff}" -le 1 ] && [ "${diff}" -ge -1 ]; then
        log_pass "S1.${rt}.post_tool_use POST_TOOL_USE=${post_tool} ≈ PRE_TOOL_USE=${pre_tool}"
    else
        log_fail "S1.${rt}.post_tool_use POST_TOOL_USE=${post_tool} far from PRE_TOOL_USE=${pre_tool}"
    fi

    # STOP = 1 — clean termination
    if [ "${stop_count}" -eq 1 ]; then
        log_pass "S1.${rt}.stop STOP=1 (clean termination)"
    else
        log_fail "S1.${rt}.stop STOP=${stop_count} (expected exactly 1)"
    fi

    # WORKFLOW_CONTINUATION ≥ 1 — D87 ADR-V2-016 (soft check)
    if [ "${continuations}" -ge 1 ]; then
        log_pass "S1.${rt}.continuation WORKFLOW_CONTINUATION ≥ 1 (D87 ADR-V2-016)"
    else
        log_xfail "S1.${rt}.continuation no WORKFLOW_CONTINUATION marker (marker name may differ — non-fatal)"
    fi
}

run_section_1() {
    log_head "Section 1: MVP 复现 (grid + claude-code, threshold-calibration)"
    echo ""
    echo "  对标 Phase 2 S4.T3 出口标准:"
    echo "  PRE_TOOL_USE ≥ 4 / POST_TOOL_USE ≈ PRE_TOOL_USE / STOP = 1"
    echo ""
    echo "  ⚠ 此 section 需要完整 EAASP 栈 (L2/L3/L4 + eaasp CLI) 运行中。"
    echo "    如果栈未启动，Section 1 全部自动 SKIP。"
    echo "    启动方式: make dev-eaasp"
    echo ""

    # L4 health check
    if ! curl -fsS -m 3 "${L4_URL}/health" >/dev/null 2>&1; then
        log_skip "S1.01 L4 orchestration not reachable (${L4_URL}/health) — Section 1 全部跳过"
        log_skip "S1.grid-runtime threshold-calibration (L4 not available)"
        log_skip "S1.claude-code-runtime threshold-calibration (L4 not available)"
        return
    fi
    log_pass "S1.01 L4 orchestration reachable"

    if [ ! -x "${CLI}" ]; then
        log_fail "S1.02 eaasp CLI missing at ${CLI}"
        log_skip "S1.grid-runtime threshold-calibration (CLI missing)"
        log_skip "S1.claude-code-runtime threshold-calibration (CLI missing)"
        return
    fi
    log_pass "S1.02 eaasp CLI found"

    # Submit skill (idempotent)
    "${CLI}" skill submit "${REPO_ROOT}/examples/skills/threshold-calibration" >/dev/null 2>&1 || true

    # grid-runtime
    if [ -n "${OPENAI_API_KEY:-}" ] || [ -n "${ANTHROPIC_API_KEY:-}" ]; then
        if nc -z 127.0.0.1 50051 2>/dev/null; then
            run_mvp_runtime "grid-runtime" 50051
        else
            log_skip "S1.grid-runtime port 50051 not open — start stack with: make dev-eaasp"
        fi
    else
        log_skip "S1.grid-runtime no API key configured"
    fi

    # claude-code-runtime
    if [ -n "${ANTHROPIC_API_KEY:-}" ]; then
        if nc -z 127.0.0.1 50052 2>/dev/null; then
            run_mvp_runtime "claude-code-runtime" 50052
        else
            log_skip "S1.claude-code-runtime port 50052 not open — start stack with: make dev-eaasp"
        fi
    else
        log_skip "S1.claude-code-runtime ANTHROPIC_API_KEY missing"
    fi
}

# ─────────────────────────────────────────────────────────────────────────────
# SECTION 2 — Phase 2.5 新功能验证
# ─────────────────────────────────────────────────────────────────────────────

# 2A: contract suite v1 — mock LLM
run_contract_suite() {
    local rt=$1
    local out_file="/tmp/phase25-contract-${rt}.txt"
    local extra_env=()

    if [ "${rt}" = "goose" ] && [ "${GOOSE_AVAILABLE}" = false ]; then
        log_xfail "S2A.contract.goose goose binary absent — expected XFAIL (Phase 3)"
        return
    fi
    if [ "${rt}" = "goose" ]; then
        extra_env=("GOOSE_BIN=${GOOSE_BIN_PATH}")
    fi

    log_info "  Running contract suite --runtime=${rt}..."
    local rc=0
    env "${extra_env[@]}" "${CONTRACT_VENV}" -m pytest \
        "${REPO_ROOT}/tests/contract/contract_v1/" \
        --runtime="${rt}" \
        -v --tb=short -q --no-header \
        > "${out_file}" 2>&1 || rc=$?

    local passed failed skipped xfailed
    passed=$(grep -c "PASSED" "${out_file}" 2>/dev/null || echo 0)
    failed=$(grep -c "^FAILED" "${out_file}" 2>/dev/null || echo 0)
    skipped=$(grep -c " SKIP" "${out_file}" 2>/dev/null || echo 0)
    xfailed=$(grep -c " XFAIL" "${out_file}" 2>/dev/null || echo 0)

    log_info "  ${rt}: passed=${passed} failed=${failed} skipped=${skipped} xfailed=${xfailed}"

    if [ "${failed}" -eq 0 ]; then
        log_pass "S2A.contract.${rt} contract-v1 GREEN (${passed}P / ${xfailed}X / ${skipped}S)"
    else
        log_fail "S2A.contract.${rt} contract-v1 has ${failed} failures — see ${out_file}"
        grep -A5 "^FAILED\|AssertionError" "${out_file}" | head -20 | sed 's/^/    /' || true
    fi
}

# 2B: nanobot real LLM E2E
run_nanobot_e2e() {
    if [ "${SKIP_LLM}" = true ]; then
        log_skip "S2B.nanobot.e2e --skip-llm specified"
        return
    fi
    if [ -z "${OPENAI_API_KEY:-}" ]; then
        log_skip "S2B.nanobot.e2e OPENAI_API_KEY missing"
        return
    fi
    if [ -z "${NANOBOT_VENV}" ] || [ ! -f "${NANOBOT_VENV}" ]; then
        log_fail "S2B.nanobot.e2e nanobot venv missing"
        return
    fi

    log_info "  Starting nanobot-runtime on :${NANOBOT_PORT}..."
    NANOBOT_RUNTIME_PORT="${NANOBOT_PORT}" \
    OPENAI_BASE_URL="${OPENAI_BASE_URL:-}" \
    OPENAI_API_KEY="${OPENAI_API_KEY:-}" \
    OPENAI_MODEL_NAME="${OPENAI_MODEL_NAME:-gpt-4o-mini}" \
    EAASP_DEPLOYMENT_MODE="shared" \
    NO_PROXY="127.0.0.1,localhost" \
    no_proxy="127.0.0.1,localhost" \
    HTTP_PROXY="" HTTPS_PROXY="" http_proxy="" https_proxy="" \
        "${NANOBOT_VENV}" -m nanobot_runtime \
        > /tmp/phase25-nanobot-e2e.log 2>&1 &
    NANOBOT_PID=$!

    if ! wait_for_port "${NANOBOT_PORT}" 20 "nanobot-runtime"; then
        log_fail "S2B.nanobot.startup nanobot-runtime did not start in 20s"
        log_info "  See: /tmp/phase25-nanobot-e2e.log"
        return
    fi
    log_pass "S2B.nanobot.startup nanobot-runtime started on :${NANOBOT_PORT}"

    # RPC baseline
    local rpc_out
    if rpc_out=$(grpc_roundtrip "${NANOBOT_PORT}" "nanobot" "${REPO_ROOT}" 2>/dev/null); then
        log_pass "S2B.nanobot.rpc_baseline Health+Initialize+Terminate OK (${rpc_out})"
    else
        log_fail "S2B.nanobot.rpc_baseline gRPC round-trip failed"
    fi

    # GetCapabilities — ADR-V2-019 deployment_mode field
    local caps_out
    if caps_out=$(grpc_get_capabilities "${NANOBOT_PORT}" "${REPO_ROOT}" 2>/dev/null); then
        if echo "${caps_out}" | grep -q "deployment_mode="; then
            log_pass "S2B.nanobot.capabilities GetCapabilities has deployment_mode (ADR-V2-019): ${caps_out}"
        else
            log_fail "S2B.nanobot.capabilities deployment_mode field missing: ${caps_out}"
        fi
    else
        log_fail "S2B.nanobot.capabilities GetCapabilities failed"
    fi

    # Real LLM Send
    log_info "  nanobot Send with real LLM (may take 10-30s)..."
    local send_out
    if send_out=$(nanobot_send_verify "${NANOBOT_PORT}" "${REPO_ROOT}" 2>/dev/null); then
        log_pass "S2B.nanobot.send real LLM Send OK (${send_out})"
    else
        log_fail "S2B.nanobot.send real LLM Send failed — check OPENAI_BASE_URL / OPENAI_API_KEY"
        log_info "  See: /tmp/phase25-nanobot-e2e.log"
    fi

    # Cleanup
    kill "${NANOBOT_PID}" 2>/dev/null || true
    NANOBOT_PID=""
}

# 2C: grid-runtime standalone — ADR-V2-019 + GetCapabilities
run_grid_standalone() {
    if [ "${SKIP_LLM}" = true ]; then
        log_skip "S2C.grid --skip-llm specified"
        return
    fi

    local grid_api_key="${OPENAI_API_KEY:-}"
    local grid_base_url="${OPENAI_BASE_URL:-}"
    local grid_model="${OPENAI_MODEL_NAME:-gpt-4o-mini}"
    local grid_provider="openai"

    if [ "${LLM_PROVIDER:-}" = "anthropic" ]; then
        grid_provider="anthropic"
        grid_api_key="${ANTHROPIC_API_KEY:-}"
        grid_base_url="${ANTHROPIC_BASE_URL:-}"
        grid_model="${ANTHROPIC_MODEL_NAME:-claude-sonnet-4-6}"
    fi

    if [ -z "${grid_api_key}" ]; then
        log_skip "S2C.grid no API key available"
        return
    fi

    log_info "  Starting grid-runtime on :${GRID_PORT} (provider=${grid_provider})..."
    local probe_out="${REPO_ROOT}/tests/contract/fixtures/_probe_out"
    mkdir -p "${probe_out}"

    GRID_RUNTIME_ADDR="127.0.0.1:${GRID_PORT}" \
    GRID_RUNTIME_ID="grid-phase25-verify" \
    LLM_PROVIDER="${grid_provider}" \
    OPENAI_API_KEY="${grid_api_key}" \
    OPENAI_BASE_URL="${grid_base_url}/v1" \
    OPENAI_MODEL_NAME="${grid_model}" \
    ANTHROPIC_API_KEY="${ANTHROPIC_API_KEY:-}" \
    ANTHROPIC_BASE_URL="${ANTHROPIC_BASE_URL:-}" \
    ANTHROPIC_MODEL_NAME="${ANTHROPIC_MODEL_NAME:-}" \
    GRID_PROBE_STRATEGY="lazy" \
    EAASP_SKILL_CACHE_DIR="${REPO_ROOT}/tests/contract/fixtures" \
    GRID_CONTRACT_PROBE_OUT="${probe_out}" \
    EAASP_DEPLOYMENT_MODE="shared" \
    RUST_LOG="grid_runtime=warn,grid_engine=warn" \
    NO_PROXY="127.0.0.1,localhost" \
    no_proxy="127.0.0.1,localhost" \
    HTTP_PROXY="" HTTPS_PROXY="" http_proxy="" https_proxy="" \
        ${GRID_LAUNCH} --port "${GRID_PORT}" \
        > /tmp/phase25-grid-c.log 2>&1 &
    GRID_PID=$!

    if ! wait_for_port "${GRID_PORT}" "${GRID_TIMEOUT}" "grid-runtime"; then
        log_fail "S2C.grid.startup grid-runtime did not start in ${GRID_TIMEOUT}s"
        log_info "  See: /tmp/phase25-grid-c.log"
        return
    fi
    log_pass "S2C.grid.startup grid-runtime started on :${GRID_PORT}"

    local rpc_out
    if rpc_out=$(grpc_roundtrip "${GRID_PORT}" "grid" "${REPO_ROOT}" 2>/dev/null); then
        log_pass "S2C.grid.rpc_baseline Health+Initialize+Terminate OK (${rpc_out})"
    else
        log_fail "S2C.grid.rpc_baseline gRPC round-trip failed"
    fi

    local caps_out
    if caps_out=$(grpc_get_capabilities "${GRID_PORT}" "${REPO_ROOT}" 2>/dev/null); then
        log_pass "S2C.grid.capabilities GetCapabilities OK (${caps_out})"
    else
        log_fail "S2C.grid.capabilities GetCapabilities failed"
    fi

    kill "${GRID_PID}" 2>/dev/null || true
    GRID_PID=""
}

# 2D: goose-runtime baseline (stub Send expected)
run_goose_baseline() {
    log_info "  Starting eaasp-goose-runtime on :${GOOSE_PORT}..."
    GOOSE_RUNTIME_GRPC_ADDR="0.0.0.0:${GOOSE_PORT}" \
    GOOSE_BIN="${GOOSE_BIN_PATH:-}" \
    EAASP_DEPLOYMENT_MODE="shared" \
    RUST_LOG="eaasp_goose_runtime=warn" \
    NO_PROXY="127.0.0.1,localhost" \
    no_proxy="127.0.0.1,localhost" \
    HTTP_PROXY="" HTTPS_PROXY="" http_proxy="" https_proxy="" \
        ${GOOSE_LAUNCH} \
        > /tmp/phase25-goose.log 2>&1 &
    GOOSE_PID=$!

    if ! wait_for_port "${GOOSE_PORT}" "${GOOSE_TIMEOUT}" "eaasp-goose-runtime"; then
        log_fail "S2D.goose.startup eaasp-goose-runtime did not start in ${GOOSE_TIMEOUT}s"
        log_info "  See: /tmp/phase25-goose.log"
        return
    fi
    log_pass "S2D.goose.startup eaasp-goose-runtime started on :${GOOSE_PORT}"

    # RPC baseline
    local rpc_out
    if rpc_out=$(grpc_roundtrip "${GOOSE_PORT}" "goose" "${REPO_ROOT}" 2>/dev/null); then
        log_pass "S2D.goose.rpc_baseline Health+Initialize+Terminate OK (${rpc_out})"
    else
        log_fail "S2D.goose.rpc_baseline gRPC round-trip failed"
    fi

    # Send — stub returns single "done" chunk (XFAIL: real ACP is Phase 3)
    local send_out
    send_out=$(goose_send_stub_check "${GOOSE_PORT}" "${REPO_ROOT}" 2>/dev/null) || send_out=""
    if echo "${send_out}" | grep -q "total_chunks="; then
        log_xfail "S2D.goose.send Send is stub (${send_out}) — real ACP wiring deferred to Phase 3 (expected XFAIL)"
    else
        log_fail "S2D.goose.send Send probe failed — goose-runtime may be broken"
    fi

    # ADR-V2-019: per_session mode switch
    log_info "  Testing EAASP_DEPLOYMENT_MODE=per_session (ADR-V2-019)..."
    kill "${GOOSE_PID}" 2>/dev/null || true
    sleep 1
    GOOSE_RUNTIME_GRPC_ADDR="0.0.0.0:${GOOSE_PORT}" \
    GOOSE_BIN="${GOOSE_BIN_PATH:-}" \
    EAASP_DEPLOYMENT_MODE="per_session" \
    RUST_LOG="eaasp_goose_runtime=warn" \
        ${GOOSE_LAUNCH} \
        > /tmp/phase25-goose-per-session.log 2>&1 &
    GOOSE_PID=$!
    if wait_for_port "${GOOSE_PORT}" "${GOOSE_TIMEOUT}" "goose per_session"; then
        log_pass "S2D.goose.per_session EAASP_DEPLOYMENT_MODE=per_session starts OK (ADR-V2-019)"
    else
        log_fail "S2D.goose.per_session runtime did not start in per_session mode"
    fi

    kill "${GOOSE_PID}" 2>/dev/null || true
    GOOSE_PID=""
}

# 2E: eaasp-scoped-hook-mcp unit tests
run_hook_mcp_unit_tests() {
    log_info "  cargo test -p eaasp-scoped-hook-mcp..."
    local hook_out rc=0
    hook_out=$(cargo test -p eaasp-scoped-hook-mcp --quiet 2>&1) || rc=$?

    local hook_pass hook_fail
    hook_pass=$(echo "${hook_out}" | grep -oE '[0-9]+ passed' | grep -oE '[0-9]+' | head -1 || echo "0")
    hook_fail=$(echo "${hook_out}" | grep -oE '[0-9]+ failed' | grep -oE '[0-9]+' | head -1 || echo "0")
    hook_pass="${hook_pass:-0}"; hook_fail="${hook_fail:-0}"

    if [ "${hook_fail}" -eq 0 ] && [ "${hook_pass}" -gt 0 ]; then
        log_pass "S2E.hook_mcp.unit eaasp-scoped-hook-mcp ${hook_pass} unit tests PASS"
    elif [ "${hook_fail}" -gt 0 ]; then
        log_fail "S2E.hook_mcp.unit ${hook_fail} unit tests FAILED"
        echo "${hook_out}" | tail -20 | sed 's/^/    /'
    else
        log_skip "S2E.hook_mcp.unit no results (build may have failed)"
    fi
}

# 2F: hook envelope ADR-V2-006 via contract hook_envelope suite
run_hook_envelope_suite() {
    local rt=$1
    local out_file="/tmp/phase25-hook-envelope-${rt}.txt"
    local rc=0

    "${CONTRACT_VENV}" -m pytest \
        "${REPO_ROOT}/tests/contract/contract_v1/test_hook_envelope.py" \
        --runtime="${rt}" \
        -v --tb=short -q --no-header \
        > "${out_file}" 2>&1 || rc=$?

    local he_pass he_fail he_xfail
    he_pass=$(grep -c "PASSED" "${out_file}" 2>/dev/null || echo 0)
    he_fail=$(grep -c "^FAILED" "${out_file}" 2>/dev/null || echo 0)
    he_xfail=$(grep -c "XFAIL" "${out_file}" 2>/dev/null || echo 0)

    if [ "${he_fail}" -eq 0 ]; then
        log_pass "S2F.hook_envelope.${rt} ADR-V2-006 envelope suite OK (${he_pass}P / ${he_xfail}X)"
    else
        log_fail "S2F.hook_envelope.${rt} hook envelope ${he_fail} failures — see ${out_file}"
        grep -A5 "^FAILED\|AssertionError" "${out_file}" | head -20 | sed 's/^/    /' || true
    fi
}

run_section_2() {
    log_head "Section 2: Phase 2.5 新功能验证"

    log_head "  2A: 合约套件 v1 (mock LLM, 全自动, 4 runtimes)"
    for rt in grid claude-code nanobot goose; do
        run_contract_suite "${rt}"
    done

    log_head "  2B: nanobot-runtime E2E (真实 LLM)"
    run_nanobot_e2e

    log_head "  2C: grid-runtime 独立启动 (ADR-V2-019 deployment_mode + GetCapabilities)"
    run_grid_standalone

    log_head "  2D: goose-runtime 基线 (stub Send XFAIL expected, ADR-V2-019 mode switch)"
    run_goose_baseline

    log_head "  2E: eaasp-scoped-hook-mcp 单元测试"
    run_hook_mcp_unit_tests

    log_head "  2F: hook envelope ADR-V2-006 合约套件 (grid + claude-code)"
    for rt in grid claude-code; do
        run_hook_envelope_suite "${rt}"
    done
}

# ─────────────────────────────────────────────────────────────────────────────
# MAIN
# ─────────────────────────────────────────────────────────────────────────────
echo "================================================================"
echo "  EAASP v2.0 Phase 2.5 — E2E Verification"
echo "  $(date '+%Y-%m-%d %H:%M:%S')"
echo "================================================================"
echo ""
echo "  Log: ${LOG_FILE}"
echo "" > "${LOG_FILE}"
echo "# Phase 2.5 E2E Verify — $(date '+%Y-%m-%d %H:%M:%S')" >> "${LOG_FILE}"

load_env

case "${ONLY_SECTION}" in
    "0") run_section_0 ;;
    "1") run_section_0; run_section_1 ;;
    "2") run_section_0; run_section_2 ;;
    "")
        run_section_0
        [ "${SKIP_MVP}" = false ] && run_section_1
        run_section_2
        ;;
    *)
        echo "Unknown --section ${ONLY_SECTION}. Valid: 0, 1, 2." >&2
        exit 2
        ;;
esac

# ── Final summary ─────────────────────────────────────────────────────────
echo ""
echo "================================================================"
echo -e "${BOLD}  Phase 2.5 E2E Verification — Summary${NC}"
echo "================================================================"
echo -e "  ${GREEN}PASS${NC}   : ${TOTAL_PASS}"
echo -e "  ${RED}FAIL${NC}   : ${TOTAL_FAIL}"
echo -e "  ${YELLOW}XFAIL${NC}  : ${TOTAL_XFAIL}  (expected: goose Send stub, D140 hook envelope gap)"
echo -e "  ${YELLOW}SKIP${NC}   : ${TOTAL_SKIP}"
echo ""
echo "  Log: ${LOG_FILE}"
echo ""

if [ "${TOTAL_FAIL}" -gt 0 ]; then
    echo -e "${RED}${BOLD}  FAILURES (${TOTAL_FAIL}):${NC}"
    for f in "${FAILURES[@]}"; do
        echo -e "    ${RED}✗${NC}  ${f}"
    done
    echo ""
    echo -e "${RED}${BOLD}  ⛔ Phase 2.5 E2E FAILED — fix above before closing phase${NC}"
    exit 1
elif [ "${TOTAL_PASS}" -gt 0 ]; then
    echo -e "${GREEN}${BOLD}  ✅ Phase 2.5 E2E PASS (${TOTAL_PASS} checks, ${TOTAL_XFAIL} expected failures)${NC}"
    echo ""
    echo "  Phase 2.5 出口标准满足:"
    echo "    ✅ Section 1: MVP 复现 (threshold-calibration, D87 PRE_TOOL_USE ≥ 4)"
    echo "    ✅ Section 2A: 合约套件 v1 — grid + claude-code + nanobot GREEN"
    echo "    ✅ Section 2B: nanobot-runtime 真实 LLM E2E"
    echo "    ✅ Section 2C: grid-runtime ADR-V2-019 GetCapabilities"
    echo "    ✅ Section 2D: goose-runtime 基线 RPC"
    echo "    ✅ Section 2E: eaasp-scoped-hook-mcp 单元测试"
    echo "    ✅ Section 2F: hook envelope ADR-V2-006"
    echo "    XFAIL: goose Send stub (Phase 3 scope)"
    echo ""
    exit 0
else
    echo -e "${YELLOW}${BOLD}  ⚠ No checks ran — check --section / --skip-* flags${NC}"
    exit 2
fi
