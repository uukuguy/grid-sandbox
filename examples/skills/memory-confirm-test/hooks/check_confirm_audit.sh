#!/usr/bin/env bash
# Stop hook: assert at least one memory.confirm was called in this session.
# If none occurred, inject a reminder prompt (re-entry) to complete the workflow.
#
# ADR-V2-006 Stop hook contract:
#   stdin  = JSON StopHookEnvelope { session_id, skill_id, tool_calls: [...] }
#   exit 0 = Noop (let session terminate normally)
#   exit 2 = InjectAndContinue (inject message returned via stdout, re-enter loop)
#   other  = fail-open (Noop)

set -euo pipefail

envelope="$(cat)"

# Count how many memory.confirm tool calls occurred in this session
confirm_count="$(printf '%s' "$envelope" | python3 -c "
import sys, json
d = json.load(sys.stdin)
calls = d.get('tool_calls', [])
count = sum(
    1 for c in calls
    if c.get('tool_name', '') in ('memory.confirm', 'l2:memory.confirm')
)
print(count)
" 2>/dev/null || echo "0")"

if [[ "$confirm_count" -ge 1 ]]; then
  exit 0
fi

# No confirmation occurred — inject a reminder
printf '[{"role":"system","content":"[memory-confirm-test] No memory.confirm call was observed in this session. Please call memory.search to find a candidate memory entry, memory.read to inspect it, then memory.confirm with verdict=approved before proceeding to memory.write_anchor."}]\n'
exit 2
