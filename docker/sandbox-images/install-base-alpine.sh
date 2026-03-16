#!/bin/sh
# Common base packages for Alpine-based Docker images
set -euo pipefail
apk add --no-cache \
    git curl wget jq ca-certificates \
    build-base pkgconf \
    ripgrep fd tree \
    zip unzip tar gzip bash coreutils
