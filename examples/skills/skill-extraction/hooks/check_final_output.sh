#!/usr/bin/env bash
# Stop hook for skill-extraction meta-skill.
#
# Asserts the Stop envelope carries both `draft_memory_id` and
# `evidence_anchor_id` — grid-engine populates these after `memory_write_file`
# and `memory_write_anchor` tool calls respectively. See
# `crates/grid-engine/src/agent/harness.rs` `last_draft_memory_id` /
# `last_evidence_anchor_id`.
#
# Envelope shape (ADR-V2-006 §2, HookContext::to_json):
#   {
#     "event": "Stop",
#     "skill_id": "skill-extraction",
#     "draft_memory_id": "mem_...",
#     "evidence_anchor_id": "anc_...",
#     ...
#   }
#
# Older engines nested the final assistant output under `.output.*`.
# Accept either for forward/backward compat.
set -euo pipefail

input="$(cat)"

has_top_both() {
  echo "$input" | jq -e \
    '(.draft_memory_id // "" | length > 0)
     and (.evidence_anchor_id // "" | length > 0)' >/dev/null 2>&1
}

has_output_both() {
  echo "$input" | jq -e \
    '(.output // {} | .draft_memory_id // "" | length > 0)
     and (.output // {} | .evidence_anchor_id // "" | length > 0)' >/dev/null 2>&1
}

if has_top_both || has_output_both; then
  echo '{"decision":"allow"}'
  exit 0
fi

echo '{"decision":"continue","reason":"Stop envelope missing draft_memory_id and/or evidence_anchor_id; skill must call memory_write_file + memory_write_anchor, and the engine must thread the returned IDs into the Stop hook context (grid-engine harness.rs)"}'
exit 2
