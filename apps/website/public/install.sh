#!/bin/sh
# Installs the piramid binary for this machine.
# Usage: curl -fsSL https://piramiddb.com/install.sh | sh
# PIRAMID_VERSION selects a release tag, PIRAMID_INSTALL_DIR the target directory.

set -eu

REPO="ashworks1706/piramid-sdk"
VERSION="${PIRAMID_VERSION:-latest}"
INSTALL_DIR="${PIRAMID_INSTALL_DIR:-$HOME/.local/bin}"

fail() {
  printf 'piramid install: %s\n' "$1" >&2
  exit 1
}

case "$(uname -s)" in
  Linux) os=linux ;;
  Darwin) os=macos ;;
  *) fail "unsupported OS $(uname -s); on Windows download piramid-windows-amd64.zip from https://github.com/$REPO/releases or use Docker" ;;
esac

case "$(uname -m)" in
  x86_64 | amd64) arch=amd64 ;;
  aarch64 | arm64) arch=arm64 ;;
  *) fail "unsupported architecture $(uname -m)" ;;
esac

asset="piramid-$os-$arch.tar.gz"
if [ "$VERSION" = latest ]; then
  url="https://github.com/$REPO/releases/latest/download/$asset"
else
  url="https://github.com/$REPO/releases/download/$VERSION/$asset"
fi

if command -v curl >/dev/null 2>&1; then
  fetch() { curl -fsSL "$1" -o "$2"; }
elif command -v wget >/dev/null 2>&1; then
  fetch() { wget -qO "$2" "$1"; }
else
  fail "needs curl or wget"
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

printf 'Downloading %s\n' "$url"
fetch "$url" "$tmp/$asset" || fail "download failed: $url"
tar -xzf "$tmp/$asset" -C "$tmp"

mkdir -p "$INSTALL_DIR"
mv "$tmp/piramid" "$INSTALL_DIR/piramid"
chmod +x "$INSTALL_DIR/piramid"

printf 'Installed piramid to %s\n' "$INSTALL_DIR/piramid"
case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *) printf 'Add it to PATH: export PATH="%s:$PATH"\n' "$INSTALL_DIR" ;;
esac
printf 'Run: piramid serve\n'
