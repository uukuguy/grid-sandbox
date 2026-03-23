#!/bin/bash
# install-python-packages.sh - Install Python packages from requirements files
# Usage: install-python-packages.sh <requirements-file> [requirements-file2 ...]
# Called during Docker build.
set -euo pipefail

if [ $# -eq 0 ]; then
  echo "Usage: $0 <requirements-file> [requirements-file2 ...]" >&2
  exit 1
fi

for req_file in "$@"; do
  if [ ! -f "${req_file}" ]; then
    echo "Warning: ${req_file} not found, skipping." >&2
    continue
  fi
  echo "Installing packages from ${req_file}..."
  pip3 install --no-cache-dir --break-system-packages -r "${req_file}"
  echo ""
done

echo "Python packages installed:"
pip3 list --format=columns 2>/dev/null | tail -n +3 | wc -l
echo " packages total."
