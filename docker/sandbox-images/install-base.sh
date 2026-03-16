#!/bin/bash
# Common base packages for Debian/Ubuntu-based Docker images
set -euo pipefail
apt-get update && apt-get install -y --no-install-recommends \
    git curl wget jq ca-certificates \
    build-essential pkg-config \
    ripgrep fd-find tree \
    unzip zip tar gzip \
    && rm -rf /var/lib/apt/lists/*
