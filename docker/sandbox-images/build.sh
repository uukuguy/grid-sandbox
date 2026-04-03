#!/bin/bash
# Build grid-sandbox Docker images
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TAG_PREFIX="grid-sandbox"
VERSION="1.0"

IMAGES=(python rust nodejs bash general swebench)

build_image() {
    local name="$1"
    local dockerfile="Dockerfile.${name}"
    local tag="${TAG_PREFIX}/${name}:${VERSION}"

    echo "Building ${tag}..."
    docker build -t "${tag}" -f "${SCRIPT_DIR}/${dockerfile}" "${SCRIPT_DIR}"
    echo "Done: ${tag}"
}

if [ $# -eq 0 ] || [ "$1" = "all" ]; then
    for img in "${IMAGES[@]}"; do
        build_image "$img"
    done
    echo ""
    echo "All images built successfully."
else
    for arg in "$@"; do
        if [[ " ${IMAGES[*]} " =~ " ${arg} " ]]; then
            build_image "$arg"
        else
            echo "Unknown image: ${arg}"
            echo "Available: ${IMAGES[*]}"
            exit 1
        fi
    done
fi
