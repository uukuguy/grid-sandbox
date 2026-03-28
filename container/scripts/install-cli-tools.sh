#!/bin/bash
# install-cli-tools.sh - Install modern CLI tools from GitHub Releases
# Called during Docker build to install pre-compiled binaries.
# Supports amd64 and arm64 architectures.
set -euo pipefail

ARCH=$(dpkg --print-architecture)
case "${ARCH}" in
  amd64) RUST_ARCH="x86_64" ;;
  arm64) RUST_ARCH="aarch64" ;;
  *) echo "Unsupported architecture: ${ARCH}" >&2; exit 1 ;;
esac

# Tool versions
RIPGREP_VERSION="${RIPGREP_VERSION:-15.1.0}"
FD_VERSION="${FD_VERSION:-10.4.2}"
BAT_VERSION="${BAT_VERSION:-0.26.1}"
EZA_VERSION="${EZA_VERSION:-0.23.4}"
DUST_VERSION="${DUST_VERSION:-1.2.4}"
DELTA_VERSION="${DELTA_VERSION:-0.18.2}"
SD_VERSION="${SD_VERSION:-1.1.0}"
XH_VERSION="${XH_VERSION:-0.25.3}"
TOKEI_VERSION="${TOKEI_VERSION:-13.0.0-alpha.0}"
GH_VERSION="${GH_VERSION:-2.88.1}"

INSTALL_DIR="/usr/local/bin"
TMP_DIR=$(mktemp -d)
trap 'rm -rf "${TMP_DIR}"' EXIT

install_from_tarball() {
  local name="$1" url="$2" binary_path="$3"
  echo "Installing ${name} from ${url}..."
  curl -fsSL "${url}" -o "${TMP_DIR}/${name}.tar.gz"
  tar xzf "${TMP_DIR}/${name}.tar.gz" -C "${TMP_DIR}"
  install -m 755 "${TMP_DIR}/${binary_path}" "${INSTALL_DIR}/${name}"
  rm -f "${TMP_DIR}/${name}.tar.gz"
  echo "  OK: $(${INSTALL_DIR}/${name} --version 2>&1 | head -1)"
}

# --- ripgrep (musl for amd64, gnu for arm64 — 15.1.0 dropped x86_64-gnu) ---
if [ "${ARCH}" = "amd64" ]; then
  RG_LIBC="musl"
else
  RG_LIBC="gnu"
fi
install_from_tarball "rg" \
  "https://github.com/BurntSushi/ripgrep/releases/download/${RIPGREP_VERSION}/ripgrep-${RIPGREP_VERSION}-${RUST_ARCH}-unknown-linux-${RG_LIBC}.tar.gz" \
  "ripgrep-${RIPGREP_VERSION}-${RUST_ARCH}-unknown-linux-${RG_LIBC}/rg"

# --- fd (gnu) ---
install_from_tarball "fd" \
  "https://github.com/sharkdp/fd/releases/download/v${FD_VERSION}/fd-v${FD_VERSION}-${RUST_ARCH}-unknown-linux-gnu.tar.gz" \
  "fd-v${FD_VERSION}-${RUST_ARCH}-unknown-linux-gnu/fd"

# --- bat (gnu) ---
install_from_tarball "bat" \
  "https://github.com/sharkdp/bat/releases/download/v${BAT_VERSION}/bat-v${BAT_VERSION}-${RUST_ARCH}-unknown-linux-gnu.tar.gz" \
  "bat-v${BAT_VERSION}-${RUST_ARCH}-unknown-linux-gnu/bat"

# --- eza (gnu, flat tarball) ---
echo "Installing eza..."
curl -fsSL "https://github.com/eza-community/eza/releases/download/v${EZA_VERSION}/eza_${RUST_ARCH}-unknown-linux-gnu.tar.gz" \
  -o "${TMP_DIR}/eza.tar.gz"
tar xzf "${TMP_DIR}/eza.tar.gz" -C "${TMP_DIR}"
find "${TMP_DIR}" -name "eza" -type f | head -1 | xargs -I{} install -m 755 {} "${INSTALL_DIR}/eza"
echo "  OK: $(eza --version 2>&1 | head -1)"

# --- dust (gnu) ---
install_from_tarball "dust" \
  "https://github.com/bootandy/dust/releases/download/v${DUST_VERSION}/dust-v${DUST_VERSION}-${RUST_ARCH}-unknown-linux-gnu.tar.gz" \
  "dust-v${DUST_VERSION}-${RUST_ARCH}-unknown-linux-gnu/dust"

# --- delta (gnu) ---
install_from_tarball "delta" \
  "https://github.com/dandavison/delta/releases/download/${DELTA_VERSION}/delta-${DELTA_VERSION}-${RUST_ARCH}-unknown-linux-gnu.tar.gz" \
  "delta-${DELTA_VERSION}-${RUST_ARCH}-unknown-linux-gnu/delta"

# --- sd (musl only for arm64) ---
echo "Installing sd..."
curl -fsSL "https://github.com/chmln/sd/releases/download/v${SD_VERSION}/sd-v${SD_VERSION}-${RUST_ARCH}-unknown-linux-musl.tar.gz" \
  -o "${TMP_DIR}/sd.tar.gz"
tar xzf "${TMP_DIR}/sd.tar.gz" -C "${TMP_DIR}"
find "${TMP_DIR}" -name "sd" -type f -executable | head -1 | xargs -I{} install -m 755 {} "${INSTALL_DIR}/sd"
echo "  OK: $(sd --version 2>&1 | head -1)"

# --- xh (musl) ---
install_from_tarball "xh" \
  "https://github.com/ducaale/xh/releases/download/v${XH_VERSION}/xh-v${XH_VERSION}-${RUST_ARCH}-unknown-linux-musl.tar.gz" \
  "xh-v${XH_VERSION}-${RUST_ARCH}-unknown-linux-musl/xh"

# --- tokei (gnu, flat tarball) ---
echo "Installing tokei..."
curl -fsSL "https://github.com/XAMPPRocky/tokei/releases/download/v${TOKEI_VERSION}/tokei-${RUST_ARCH}-unknown-linux-gnu.tar.gz" \
  -o "${TMP_DIR}/tokei.tar.gz"
tar xzf "${TMP_DIR}/tokei.tar.gz" -C "${TMP_DIR}"
find "${TMP_DIR}" -name "tokei" -type f | head -1 | xargs -I{} install -m 755 {} "${INSTALL_DIR}/tokei"
echo "  OK: $(tokei --version 2>&1 | head -1)"

# --- gh (GitHub CLI) ---
echo "Installing gh..."
curl -fsSL "https://github.com/cli/cli/releases/download/v${GH_VERSION}/gh_${GH_VERSION}_linux_${ARCH}.tar.gz" \
  -o "${TMP_DIR}/gh.tar.gz"
tar xzf "${TMP_DIR}/gh.tar.gz" -C "${TMP_DIR}"
install -m 755 "${TMP_DIR}/gh_${GH_VERSION}_linux_${ARCH}/bin/gh" "${INSTALL_DIR}/gh"
echo "  OK: $(gh --version 2>&1 | head -1)"

echo ""
echo "All CLI tools installed successfully."
