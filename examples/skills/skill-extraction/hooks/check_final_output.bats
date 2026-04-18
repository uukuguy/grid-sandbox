#!/usr/bin/env bats
# D108 — bats regression tests for check_final_output.sh (Stop hook).
# Tests: allow with top-level fields, allow via .output.* fallback,
#        deny missing both, deny missing one, empty envelope.

HOOK="$BATS_TEST_DIRNAME/check_final_output.sh"

@test "allow: top-level draft_memory_id and evidence_anchor_id" {
  input='{"event":"Stop","draft_memory_id":"mem_abc","evidence_anchor_id":"anc_xyz"}'
  run bash -c "echo '$input' | bash '$HOOK'"
  [ "$status" -eq 0 ]
  echo "$output" | grep -q '"allow"'
}

@test "allow: nested under .output fallback" {
  input='{"event":"Stop","output":{"draft_memory_id":"mem_abc","evidence_anchor_id":"anc_xyz"}}'
  run bash -c "echo '$input' | bash '$HOOK'"
  [ "$status" -eq 0 ]
  echo "$output" | grep -q '"allow"'
}

@test "deny: missing both fields exits 2" {
  input='{"event":"Stop"}'
  run bash -c "echo '$input' | bash '$HOOK'"
  [ "$status" -eq 2 ]
  echo "$output" | grep -q '"continue"'
}

@test "deny: missing evidence_anchor_id exits 2" {
  input='{"event":"Stop","draft_memory_id":"mem_abc"}'
  run bash -c "echo '$input' | bash '$HOOK'"
  [ "$status" -eq 2 ]
}

@test "deny: missing draft_memory_id exits 2" {
  input='{"event":"Stop","evidence_anchor_id":"anc_xyz"}'
  run bash -c "echo '$input' | bash '$HOOK'"
  [ "$status" -eq 2 ]
}

@test "deny: empty string fields exit 2" {
  input='{"event":"Stop","draft_memory_id":"","evidence_anchor_id":""}'
  run bash -c "echo '$input' | bash '$HOOK'"
  [ "$status" -eq 2 ]
}
