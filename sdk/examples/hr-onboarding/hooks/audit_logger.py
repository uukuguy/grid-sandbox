#!/usr/bin/env python3
"""PostToolUse audit hook — logs all tool executions for compliance.

This hook is a PostToolUse handler that records tool execution details
to a structured audit log. Integrated via SKILL.md frontmatter hooks.

Usage as managed hook:
  event: PostToolUse
  handler_type: command
  config:
    command: "python hooks/audit_logger.py"
"""

from __future__ import annotations

import json
import sys
from datetime import datetime, timezone


def main():
    """Read tool execution event from stdin and emit audit record."""
    payload = json.loads(sys.stdin.read())

    tool_name = payload.get("tool_name", "unknown")
    tool_input = payload.get("tool_input", {})
    tool_output = payload.get("tool_output", "")
    is_error = payload.get("is_error", False)
    session_id = payload.get("session_id", "")

    # Build audit record
    audit_record = {
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "event": "tool_execution",
        "session_id": session_id,
        "tool_name": tool_name,
        "input_preview": str(tool_input)[:200],
        "output_preview": str(tool_output)[:200],
        "is_error": is_error,
        "status": "error" if is_error else "success",
    }

    # Log to stderr (structured audit log)
    print(json.dumps(audit_record, ensure_ascii=False), file=sys.stderr)

    # Return allow decision (audit hooks never block)
    result = {
        "decision": "allow",
        "reason": f"Audit logged: {tool_name}",
    }
    print(json.dumps(result))


if __name__ == "__main__":
    main()
