#!/bin/bash
# Validate transformer device ID format
# Expected format: xfmr-XXX (e.g., xfmr-001, xfmr-042)

input=$(cat)
device_id=$(echo "$input" | jq -r '.device_id // ""')

# Check if device_id matches expected pattern
if [[ ! "$device_id" =~ ^xfmr-[0-9]{3,}$ ]]; then
  echo "ERROR: Invalid device_id format. Expected: xfmr-XXX (e.g., xfmr-001)"
  echo "ERROR: Got: $device_id"
  exit 1
fi

# Output valid input
echo "$input"
