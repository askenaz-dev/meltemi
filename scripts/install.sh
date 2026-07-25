#!/bin/sh
# SPDX-License-Identifier: Apache-2.0
#
# Meltemi installer (macOS/Linux). Short and auditable on purpose: read it
# before running it, and verify its published hash. There is no blind pipe to a
# shell — the script verifies the release checksum and refuses on a mismatch.
#
# Manual equivalent (if you prefer not to run this):
#   1. Download the release archive for your platform and its SHA256SUMS.
#   2. Verify: `sha256sum --check SHA256SUMS` (or `shasum -a 256 --check`).
#   3. Verify the signature of SHA256SUMS with the published signing key.
#   4. Extract `meltemi` and `meltemid` into a directory on your PATH.
#   5. Create the alias: `ln -s meltemi <dir>/mel`.
#
# Usage:
#   MELTEMI_VERSION=v0.1.0 sh install.sh [install-dir]
# Default install dir: $HOME/.local/bin

set -eu

VERSION="${MELTEMI_VERSION:-latest}"
INSTALL_DIR="${1:-$HOME/.local/bin}"
# Canonical download base — declared once in docs/release.md and verified by
# the site lint; override only for a local mirror while testing.
BASE_URL="${MELTEMI_BASE_URL:-https://github.com/askenaz-dev/meltemi/releases}"

# The two shapes the host serves: the version-free redirector for the latest
# release, and the tagged path for a pinned one. `latest` is NOT a tag, so
# asking for `download/latest/<asset>` is a 404 — the mistake this guards.
if [ "$VERSION" = "latest" ]; then
  asset_base="$BASE_URL/latest/download"
else
  asset_base="$BASE_URL/download/$VERSION"
fi

os="$(uname -s)"
case "$os" in
  Linux)  asset="meltemi-Linux.tar.gz" ;;
  Darwin) asset="meltemi-macOS.tar.gz" ;;
  *) echo "unsupported OS: $os (use the Windows installer or install manually)"; exit 1 ;;
esac

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

echo "Downloading $asset ($VERSION)..."
curl -fsSL "$asset_base/$asset" -o "$tmp/$asset"
curl -fsSL "$asset_base/SHA256SUMS" -o "$tmp/SHA256SUMS"

echo "Verifying checksum..."
( cd "$tmp" && grep " $asset\$" SHA256SUMS | { command -v sha256sum >/dev/null && sha256sum --check - || shasum -a 256 --check -; } )

echo "Installing to $INSTALL_DIR..."
mkdir -p "$INSTALL_DIR"
tar -xzf "$tmp/$asset" -C "$INSTALL_DIR" meltemi meltemid
chmod +x "$INSTALL_DIR/meltemi" "$INSTALL_DIR/meltemid"

# The short alias `mel` -> meltemi.
ln -sf "$INSTALL_DIR/meltemi" "$INSTALL_DIR/mel"

echo "Installed: meltemi, meltemid, and the alias 'mel' in $INSTALL_DIR"
echo "Ensure $INSTALL_DIR is on your PATH."
