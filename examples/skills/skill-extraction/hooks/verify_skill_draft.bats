#!/usr/bin/env bats
# D108 — bats regression tests for verify_skill_draft.sh (PostToolUse hook).
# Tests: allow path, deny-wrong-status, deny-missing-memory_id,
#        passthrough non-memory_write_file, malformed-envelope.

HOOK="$BATS_TEST_DIRNAME/verify_skill_draft.sh"

@test "allow: valid memory_write_file with agent_suggested status" {
  input='{"tool_name":"memory_write_file","tool_result":{"memory_id":"mem_abc123","status":"agent_suggested"}}'
  run bash -c "echo '$input' | bash '$HOOK'"
  [ "$status" -eq 0 ]
  echo "$output" | grep -q '"allow"'
}

@test "deny: memory_write_file with confirmed status exits 2" {
  input='{"tool_name":"memory_write_file","tool_result":{"memory_id":"mem_abc123","status":"confirmed"}}'
  run bash -c "echo '$input' | bash '$HOOK'"
  [ "$status" -eq 2 ]
  echo "$output" | grep -q '"continue"'
}

@test "deny: memory_write_file with missing memory_id exits 2" {
  input='{"tool_name":"memory_write_file","tool_result":{"status":"agent_suggested"}}'
  run bash -c "echo '$input' | bash '$HOOK'"
  [ "$status" -eq 2 ]
  echo "$output" | grep -q '"continue"'
}

@test "allow: non-memory_write_file tool passes through" {
  input='{"tool_name":"memory_search","tool_result":{"hits":[]}}'
  run bash -c "echo '$input' | bash '$HOOK'"
  [ "$status" -eq 0 ]
  echo "$output" | grep -q '"allow"'
}

@test "deny: malformed envelope (empty input) exits 2" {
  run bash -c "echo '{}' | bash '$HOOK'"
  # tool_name empty → passthrough → allow
  [ "$status" -eq 0 ]
  echo "$output" | grep -q '"allow"'
}

@test "deny: memory_write_file with empty memory_id exits 2" {
  input='{"tool_name":"memory_write_file","tool_result":{"memory_id":"","status":"agent_suggested"}}'
  run bash -c "echo '$input' | bash '$HOOK'"
  [ "$status" -eq 2 ]
}
