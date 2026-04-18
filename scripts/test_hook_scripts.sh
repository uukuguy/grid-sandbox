#!/usr/bin/env bash
# D108 — unified bats runner for all skill hook scripts.
# Usage: scripts/test_hook_scripts.sh [--verbose]
#
# Discovers all *.bats files under examples/skills/*/hooks/ and runs them
# with bats. Exits non-zero if any suite fails.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BATS_FILES=()

while IFS= read -r -d '' f; do
  BATS_FILES+=("$f")
done < <(find "$REPO_ROOT/examples/skills" -name "*.bats" -print0 | sort -z)

if [ "${#BATS_FILES[@]}" -eq 0 ]; then
  echo "No .bats files found under examples/skills/" >&2
  exit 1
fi

if ! command -v bats >/dev/null 2>&1; then
  echo "ERROR: bats not found. Install via: brew install bats-core" >&2
  exit 1
fi

echo "Running ${#BATS_FILES[@]} bats suite(s)..."
exec bats "$@" "${BATS_FILES[@]}"
