#!/bin/bash
# Ensure there is sufficient calibration context data before editing

input=$(cat)
device_id=$(echo "$input" | jq -r '.device_id // ""')
calibration_type=$(echo "$input" | jq -r '.calibration_type // ""')

# Check if required calibration context is present
data_points=$(echo "$input" | jq -r '.data_points // "" | length')

if [[ -z "$data_points" || "$data_points" -eq 0 ]]; then
  echo "ERROR: Insufficient calibration context. Data points missing."
  echo "ERROR: Please read SCADA snapshot first."
  exit 1
fi

if [[ "$data_points" -lt 10 ]]; then
  echo "WARNING: Limited data points ($data_points). Consider increasing time window." >&2
fi

echo "$input"
