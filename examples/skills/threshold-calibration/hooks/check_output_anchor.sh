#!/usr/bin/env bash
# Stop hook for threshold-calibration skill.
#
# Reads the ADR-V2-006 Stop hook envelope JSON from stdin and asserts that
# `.evidence_anchor_id` (populated by grid-engine when the agent calls
# `memory_write_anchor`) is a non-empty string.
#
# Envelope shape (grid-engine HookContext::to_json):
#   {
#     "event": "Stop",
#     "session_id": "...",
#     "skill_id": "threshold-calibration",
#     "evidence_anchor_id": "anc_...",   <-- the field we check
#     ...
#   }
#
# Some older engines / runtimes nested the final assistant output under
# `.output.evidence_anchor_id`. Accept either for forward/backward compat.
set -euo pipefail

input="$(cat)"

has_top_anchor() {
  echo "$input" | jq -e '(.evidence_anchor_id // "") | length > 0' >/dev/null 2>&1
}

has_output_anchor() {
  echo "$input" | jq -e '(.output // {} | .evidence_anchor_id // "") | length > 0' >/dev/null 2>&1
}

if has_top_anchor || has_output_anchor; then
  echo '{"decision":"allow"}'
  exit 0
fi

echo '{"decision":"continue","reason":"Stop envelope missing evidence_anchor_id; agent must call memory_write_anchor and the engine must thread the returned anchor_id into the Stop hook context (grid-engine harness.rs last_evidence_anchor_id)"}'
exit 2
