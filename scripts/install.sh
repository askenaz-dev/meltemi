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
#   4. Extract `meltemi`, `meltemid` and the two `meltemi-*-acp` adapters into a
#      directory on your PATH. Keep the adapters BESIDE the daemon: that is
#      where it looks for them.
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
# The two ACP adapters go in the same directory as the daemon, on purpose: it
# probes its own directory for them, after your PATH and the well-known paths.
binaries="meltemi meltemid meltemi-claude-acp meltemi-codex-acp"
# shellcheck disable=SC2086  # word splitting is the intent: one arg per name
tar -xzf "$tmp/$asset" -C "$INSTALL_DIR" $binaries
for name in $binaries; do
  chmod +x "$INSTALL_DIR/$name"
done

# The short alias `mel` -> meltemi.
ln -sf "$INSTALL_DIR/meltemi" "$INSTALL_DIR/mel"

echo "Installed in $INSTALL_DIR: meltemi, meltemid, the alias 'mel', and the"
echo "ACP adapters meltemi-claude-acp and meltemi-codex-acp."
echo "Ensure $INSTALL_DIR is on your PATH."
