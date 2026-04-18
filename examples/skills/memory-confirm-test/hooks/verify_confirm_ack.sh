#!/usr/bin/env bash
# PostToolUse hook: verify that memory.confirm returned a valid ack.
#
# ADR-V2-006 contract:
#   stdin  = JSON HookEnvelope  { tool_name, tool_result, session_id, skill_id, event }
#   exit 0 = allow
#   exit 2 = deny (malformed or wrong status)
#   other  = fail-open

set -euo pipefail

envelope="$(cat)"

tool_name="$(printf '%s' "$envelope" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('tool_name',''))" 2>/dev/null || echo "")"

# Only validate memory.confirm results
case "$tool_name" in
  memory.confirm|l2:memory.confirm)
    ;;
  *)
    exit 0
    ;;
esac

# Validate result: must contain status=="confirmed" and non-empty memory_id
result="$(printf '%s' "$envelope" | python3 -c "
import sys, json
d = json.load(sys.stdin)
result = d.get('tool_result', {})
if isinstance(result, str):
    result = json.loads(result)
status = result.get('status', '')
mid = result.get('memory_id', '')
if status == 'confirmed' and mid:
    print('ok')
else:
    print('fail:status=%s,memory_id=%s' % (status, mid))
" 2>/dev/null || echo "fail:parse_error")"

if [[ "$result" == "ok" ]]; then
  exit 0
fi

printf '{"decision":"deny","reason":"memory.confirm ack invalid: %s"}\n' "$result"
exit 2
