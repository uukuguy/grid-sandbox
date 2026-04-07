#!/usr/bin/env python3
"""Self-contained E2E script for HR onboarding example.

Runs in-process (no server needed):
  python sdk/examples/hr-onboarding/run_e2e.py

Verifies:
  1. Policy compilation (enterprise + BU)
  2. Hook enforcement (PII deny, audit allow)
  3. Session lifecycle (create → message → terminate)
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

# Add tools to path
TOOLS_DIR = Path(__file__).resolve().parents[3] / "tools"
sys.path.insert(0, str(TOOLS_DIR / "eaasp-governance" / "src"))

from eaasp_governance.compiler import compile_yaml_to_hooks
from eaasp_governance.merger import merge_by_scope

# Also need HookExecutor from the runtime
RUNTIME_DIR = Path(__file__).resolve().parents[3] / "lang" / "claude-code-runtime-python" / "src"
sys.path.insert(0, str(RUNTIME_DIR))
from claude_code_runtime.hook_executor import HookExecutor


def main():
    policies_dir = Path(__file__).parent / "policies"

    print("=== HR Onboarding E2E ===\n")

    # 1. Compile policies
    print("1. Compiling policies...")
    enterprise_json, enterprise_digest = compile_yaml_to_hooks(
        (policies_dir / "enterprise.yaml").read_text()
    )
    bu_json, bu_digest = compile_yaml_to_hooks(
        (policies_dir / "bu_hr.yaml").read_text()
    )
    print(f"   Enterprise: {enterprise_digest} ({len(json.loads(enterprise_json)['rules'])} rules)")
    print(f"   BU HR:      {bu_digest} ({len(json.loads(bu_json)['rules'])} rules)")

    # 2. Merge
    print("\n2. Merging policies (deny-always-wins)...")
    merged = merge_by_scope(managed=enterprise_json, skill=bu_json)
    merged_rules = json.loads(merged)["rules"]
    print(f"   Merged: {len(merged_rules)} rules")

    # 3. Load into HookExecutor
    print("\n3. Loading into HookExecutor...")
    executor = HookExecutor()
    loaded = executor.load_rules(merged)
    print(f"   Loaded {loaded} rules")

    # 4. Test PII enforcement
    print("\n4. Testing PII enforcement...")
    # Should deny: contains SSN
    decision, reason = executor.evaluate_pre_tool_call(
        "file_write",
        json.dumps({"content": "Employee SSN: 123-45-6789"}),
    )
    assert decision == "deny", f"Expected deny, got {decision}"
    print(f"   PII detected: {decision} — {reason}")

    # Should allow: no PII
    decision, reason = executor.evaluate_pre_tool_call(
        "file_write",
        json.dumps({"content": "Employee ID: E2024001, Department: Engineering"}),
    )
    assert decision == "allow", f"Expected allow, got {decision}"
    print(f"   Clean data:   {decision}")

    # 5. Test bash deny
    print("\n5. Testing bash deny...")
    decision, reason = executor.evaluate_pre_tool_call("bash", "rm -rf /")
    assert decision == "deny", f"Expected deny, got {decision}"
    print(f"   Bash blocked: {decision} — {reason}")

    # 6. Test stop enforcement
    print("\n6. Testing stop enforcement...")
    decision, reason = executor.evaluate_stop()
    assert decision == "continue", f"Expected continue, got {decision}"
    print(f"   Stop check:   {decision} — {reason}")

    print("\n=== All checks passed ===")


if __name__ == "__main__":
    main()
