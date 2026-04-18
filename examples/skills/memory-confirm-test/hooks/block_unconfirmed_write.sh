#!/usr/bin/env bash
# PreToolUse hook: block memory.write_* if target memory_id has not been confirmed.
#
# ADR-V2-006 contract:
#   stdin  = JSON HookEnvelope  { tool_name, tool_input, session_id, skill_id, event }
#   exit 0 = allow
#   exit 2 = deny (runtime injects denial message)
#   other  = fail-open (allow, log warning)

set -euo pipefail

envelope="$(cat)"

tool_name="$(printf '%s' "$envelope" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('tool_name',''))" 2>/dev/null || echo "")"

# Only intercept memory.write_* tools
case "$tool_name" in
  memory.write_*|l2:memory.write_*)
    ;;
  *)
    exit 0
    ;;
esac

# Extract memory_id from tool_input
memory_id="$(printf '%s' "$envelope" | python3 -c "
import sys, json
d = json.load(sys.stdin)
inp = d.get('tool_input', {})
if isinstance(inp, str):
    inp = json.loads(inp)
print(inp.get('memory_id', ''))
" 2>/dev/null || echo "")"

if [[ -z "$memory_id" ]]; then
  # No memory_id in input — cannot verify; deny to enforce review-before-commit
  printf '{"decision":"deny","reason":"memory.write_* requires a memory_id that has been confirmed; no memory_id provided"}\n'
  exit 2
fi

# Check confirmation status via EAASP_CONFIRM_STORE env (optional; skip if absent)
confirm_store="${EAASP_CONFIRM_STORE:-}"
if [[ -n "$confirm_store" && -f "$confirm_store" ]]; then
  status="$(python3 -c "
import sys, json
with open('$confirm_store') as f:
    store = json.load(f)
print(store.get('$memory_id', {}).get('status', 'unknown'))
" 2>/dev/null || echo "unknown")"
  if [[ "$status" != "confirmed" ]]; then
    printf '{"decision":"deny","reason":"memory_id %s has status=%s; must be confirmed before write"}\n' "$memory_id" "$status"
    exit 2
  fi
fi

exit 0
