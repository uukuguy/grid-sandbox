#!/usr/bin/env bash
# check-ccb-types-ts-sync.sh — verify that every CHUNK_TYPE_* variant defined
# in proto/eaasp/runtime/v2/common.proto has a matching identifier in
# lang/ccb-runtime-ts/src/proto/types.ts.
#
# ccb-runtime-ts uses @grpc/proto-loader (dynamic JSON messages) so most
# types never drift. The ChunkType enum is the one hand-written mirror in
# types.ts, so adding a proto variant MUST be echoed there manually. This
# script is the CI gate that catches silent drift (D149, Option B).
#
# Exit 0: every proto variant has a matching TS identifier.
# Exit 1: at least one missing / a required file is absent.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROTO_FILE="${REPO_ROOT}/proto/eaasp/runtime/v2/common.proto"
TS_FILE="${REPO_ROOT}/lang/ccb-runtime-ts/src/proto/types.ts"

for f in "$PROTO_FILE" "$TS_FILE"; do
  if [ ! -f "$f" ]; then
    echo "error: required file not found: $f" >&2
    exit 1
  fi
done

# Extract the `enum ChunkType { ... }` block from the proto, then pull the
# CHUNK_TYPE_* identifier on each non-comment line. Bounded to a single
# block so other future enums can coexist without collateral matches.
proto_names=$(
  awk '
    /^enum ChunkType[[:space:]]*\{/ { in_enum = 1; next }
    in_enum && /^\}/               { in_enum = 0; next }
    in_enum {
      line = $0
      sub(/\/\/.*/, "", line)          # strip line comments
      if (match(line, /CHUNK_TYPE_[A-Z0-9_]+/)) {
        print substr(line, RSTART, RLENGTH)
      }
    }
  ' "$PROTO_FILE"
)

if [ -z "$proto_names" ]; then
  echo "error: no CHUNK_TYPE_* variants parsed from $PROTO_FILE (enum block malformed?)" >&2
  exit 1
fi

# Extract the `export enum ChunkType { ... }` block from types.ts for matching.
ts_block=$(
  awk '
    /^export enum ChunkType[[:space:]]*\{/ { in_enum = 1; next }
    in_enum && /^\}/                        { in_enum = 0; next }
    in_enum { print }
  ' "$TS_FILE"
)

if [ -z "$ts_block" ]; then
  echo "error: could not locate 'export enum ChunkType { ... }' block in $TS_FILE" >&2
  exit 1
fi

missing=()
count=0
for name in $proto_names; do
  count=$((count + 1))
  ts_name="${name#CHUNK_TYPE_}"
  # Match `  <NAME> = ...` inside the enum block (leading spaces, then name, then `=`).
  if ! echo "$ts_block" | grep -Eq "^[[:space:]]*${ts_name}[[:space:]]*="; then
    missing+=("$name -> (expected TS: ${ts_name})")
  fi
done

if [ ${#missing[@]} -gt 0 ]; then
  echo "✗ ccb-runtime-ts types.ts is out of sync with proto ChunkType (D149):" >&2
  for m in "${missing[@]}"; do
    echo "  - missing $m" >&2
  done
  echo "" >&2
  echo "Fix: add the corresponding variant(s) to ${TS_FILE#${REPO_ROOT}/}" >&2
  echo "     (strip the CHUNK_TYPE_ prefix; keep the wire int matching the proto number)." >&2
  exit 1
fi

echo "OK: ${count} ChunkType variants in sync (proto ↔ ccb-runtime-ts/types.ts)"
