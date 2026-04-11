#!/usr/bin/env python3
"""Mock SCADA MCP server stub for the threshold-calibration skill.

S4.T1 scope: provide a parseable, callable surface with deterministic outputs.
Real MCP stdio transport is deferred to S4.T2+ (see plan Deferred D47).

Tool manifest:
  - scada_read_snapshot(device_id: str, time_window: str) -> dict
      Returns 5 deterministic samples + baseline for threshold calibration.
  - scada_write(device_id: str, field: str, value: float) -> dict
      MUST NOT be called in production. Returns an error marker; the
      threshold-calibration PreToolUse hook block_write_scada.sh should
      deny this before it reaches the server.

Usage:
  python mock-scada.py --list-tools
  python mock-scada.py --call scada_read_snapshot --device-id xfmr-042 --time-window 1h
  python mock-scada.py --call scada_write --device-id xfmr-042 --field load_pct_max --value 0.9
"""

from __future__ import annotations

import argparse
import json
import sys
from typing import Any

TOOLS: list[dict[str, Any]] = [
    {
        "name": "scada_read_snapshot",
        "description": "Read a recent telemetry snapshot for a device.",
        "input_schema": {
            "type": "object",
            "properties": {
                "device_id": {"type": "string"},
                "time_window": {"type": "string"},
            },
            "required": ["device_id", "time_window"],
        },
    },
    {
        "name": "scada_write",
        "description": "Write back a threshold (MUST be blocked by scoped hook).",
        "input_schema": {
            "type": "object",
            "properties": {
                "device_id": {"type": "string"},
                "field": {"type": "string"},
                "value": {"type": "number"},
            },
            "required": ["device_id", "field", "value"],
        },
    },
]


def scada_read_snapshot(device_id: str, time_window: str) -> dict[str, Any]:
    samples = [
        {
            "ts": f"2026-04-12T{h:02d}:00:00Z",
            "temperature_c": 65.0 + h * 0.4,
            "load_pct": 0.70 + h * 0.01,
            "doa_h2_ppm": 18.0 + h * 0.3,
        }
        for h in range(5)
    ]
    return {
        "device_id": device_id,
        "time_window": time_window,
        "samples": samples,
        "baseline": {
            "temperature_c_max": 70.0,
            "load_pct_max": 0.85,
            "doa_h2_ppm_max": 30.0,
        },
    }


def scada_write(device_id: str, field: str, value: float) -> dict[str, Any]:
    return {
        "error": "MOCK: scada_write should never be called; "
        "PreToolUse hook block_write_scada must have denied this",
        "device_id": device_id,
        "field": field,
        "value": value,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Mock SCADA MCP server (stub)")
    parser.add_argument(
        "--list-tools",
        action="store_true",
        help="Print tool manifest JSON and exit",
    )
    parser.add_argument(
        "--call",
        choices=["scada_read_snapshot", "scada_write"],
        help="Invoke a tool from the command line",
    )
    parser.add_argument("--device-id", default="xfmr-042")
    parser.add_argument("--time-window", default="1h")
    parser.add_argument("--field", default="temperature_c_max")
    parser.add_argument("--value", type=float, default=68.0)
    args = parser.parse_args()

    print(
        "[mock-scada] stub mode; real MCP stdio transport deferred (D47)",
        file=sys.stderr,
    )

    if args.list_tools:
        json.dump({"tools": TOOLS}, sys.stdout, indent=2)
        print()
        return 0

    if args.call == "scada_read_snapshot":
        json.dump(
            scada_read_snapshot(args.device_id, args.time_window),
            sys.stdout,
            indent=2,
        )
        print()
        return 0

    if args.call == "scada_write":
        json.dump(
            scada_write(args.device_id, args.field, args.value),
            sys.stderr,
            indent=2,
        )
        print(file=sys.stderr)
        return 3

    parser.print_help()
    return 0


if __name__ == "__main__":
    sys.exit(main())
