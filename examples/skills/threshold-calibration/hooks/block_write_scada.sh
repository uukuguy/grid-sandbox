#!/usr/bin/env bash
# PreToolUse hook for threshold-calibration skill.
# Reads a JSON envelope {tool_name, ...} from stdin, denies scada_write*.
set -euo pipefail

input="$(cat)"
tool="$(echo "$input" | jq -r '.tool_name // ""')"

case "$tool" in
  scada_write|scada_write_*)
    echo '{"decision":"deny","reason":"SCADA write not allowed in threshold-calibration skill"}'
    exit 2
    ;;
  *)
    echo '{"decision":"allow"}'
    exit 0
    ;;
esac
