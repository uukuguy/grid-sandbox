#!/usr/bin/env bats
# D108 — bats regression tests for check_output_anchor.sh (Stop hook).
# Tests: allow top-level, allow .output.* fallback, deny missing, deny empty.

HOOK="$BATS_TEST_DIRNAME/check_output_anchor.sh"

@test "allow: top-level evidence_anchor_id present" {
  input='{"event":"Stop","evidence_anchor_id":"anc_abc123"}'
  run bash -c "echo '$input' | bash '$HOOK'"
  [ "$status" -eq 0 ]
  echo "$output" | grep -q '"allow"'
}

@test "allow: nested under .output fallback" {
  input='{"event":"Stop","output":{"evidence_anchor_id":"anc_abc123"}}'
  run bash -c "echo '$input' | bash '$HOOK'"
  [ "$status" -eq 0 ]
  echo "$output" | grep -q '"allow"'
}

@test "deny: missing evidence_anchor_id exits 2" {
  input='{"event":"Stop"}'
  run bash -c "echo '$input' | bash '$HOOK'"
  [ "$status" -eq 2 ]
  echo "$output" | grep -q '"continue"'
}

@test "deny: empty evidence_anchor_id exits 2" {
  input='{"event":"Stop","evidence_anchor_id":""}'
  run bash -c "echo '$input' | bash '$HOOK'"
  [ "$status" -eq 2 ]
}

@test "deny: null evidence_anchor_id exits 2" {
  input='{"event":"Stop","evidence_anchor_id":null}'
  run bash -c "echo '$input' | bash '$HOOK'"
  [ "$status" -eq 2 ]
}
