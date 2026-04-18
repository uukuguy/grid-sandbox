#!/bin/bash
# Validate calibration type

input=$(cat)
calibration_type=$(echo "$input" | jq -r '.calibration_type // ""')

# Check if calibration_type is supported
case "$calibration_type" in
  temperature|load|dissolved-gas)
    echo "$input"
    exit 0
    ;;
  *)
    echo "ERROR: Unsupported calibration_type: $calibration_type"
    echo "ERROR: Supported types: temperature, load, dissolved-gas"
    exit 1
    ;;
esac
