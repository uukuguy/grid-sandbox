#!/usr/bin/env bats
# D108 — bats regression tests for block_write_scada.sh (PreToolUse hook).
# Tests: deny scada_write, deny scada_write_*, allow other tools, empty input.

HOOK="$BATS_TEST_DIRNAME/block_write_scada.sh"

@test "deny: scada_write exits 2" {
  input='{"tool_name":"scada_write","args":{}}'
  run bash -c "echo '$input' | bash '$HOOK'"
  [ "$status" -eq 2 ]
  echo "$output" | grep -q '"deny"'
}

@test "deny: scada_write_threshold exits 2" {
  input='{"tool_name":"scada_write_threshold","args":{}}'
  run bash -c "echo '$input' | bash '$HOOK'"
  [ "$status" -eq 2 ]
  echo "$output" | grep -q '"deny"'
}

@test "allow: scada_read_snapshot passes through" {
  input='{"tool_name":"scada_read_snapshot","args":{}}'
  run bash -c "echo '$input' | bash '$HOOK'"
  [ "$status" -eq 0 ]
  echo "$output" | grep -q '"allow"'
}

@test "allow: memory_search passes through" {
  input='{"tool_name":"memory_search","args":{}}'
  run bash -c "echo '$input' | bash '$HOOK'"
  [ "$status" -eq 0 ]
  echo "$output" | grep -q '"allow"'
}

@test "allow: empty tool_name passes through" {
  input='{}'
  run bash -c "echo '$input' | bash '$HOOK'"
  [ "$status" -eq 0 ]
  echo "$output" | grep -q '"allow"'
}
