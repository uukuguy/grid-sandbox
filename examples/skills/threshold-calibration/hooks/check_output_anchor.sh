#!/usr/bin/env bash
# Stop hook for threshold-calibration skill.
# Reads {output: {...}} from stdin, requires evidence_anchor_id to be non-null.
set -euo pipefail

input="$(cat)"

if echo "$input" | jq -e '(.output // {}) | has("evidence_anchor_id") and (.evidence_anchor_id != null) and (.evidence_anchor_id != "")' >/dev/null 2>&1; then
  echo '{"decision":"allow"}'
  exit 0
fi

echo '{"decision":"continue","reason":"Output missing evidence_anchor_id; agent must reference the scada_snapshot anchor written via memory_write_anchor"}'
exit 2
