#!/usr/bin/env bash
# Check that a fresh clone has everything it needs. Run with just doctor.
set -uo pipefail
cd "$(git rev-parse --show-toplevel)"

status=0
ok()   { printf '  \033[32mok\033[0m    %s\n' "$1"; }
warn() { printf '  \033[33mwarn\033[0m  %s\n' "$1"; }
bad()  { printf '  \033[31mmiss\033[0m  %s\n' "$1"; status=1; }

echo "required"
command -v cargo >/dev/null && ok "cargo $(cargo --version | cut -d' ' -f2)" || bad "cargo — install via https://rustup.rs"
cargo fmt --version   >/dev/null 2>&1 && ok "rustfmt" || bad "rustfmt — rustup component add rustfmt"
cargo clippy --version >/dev/null 2>&1 && ok "clippy"  || bad "clippy — rustup component add clippy"
command -v just >/dev/null && ok "just"  || bad "just — https://just.systems"
command -v jq   >/dev/null && ok "jq"    || bad "jq — needed by scripts/check-deps.sh"

echo
echo "optional"
command -v docker >/dev/null && ok "docker" || warn "docker — needed for 'just up'"
command -v node   >/dev/null && ok "node $(node --version)" || warn "node — needed for the website"
command -v cargo-deny >/dev/null && ok "cargo-deny" || warn "cargo-deny — 'just audit' (cargo install --locked cargo-deny)"
command -v nvcc >/dev/null && ok "nvcc $(nvcc --version | grep -oP 'release \K[0-9.]+')" \
  || warn "nvcc — only needed to build with --features gpu-cuda"

echo
echo "repo"
[ -f .env ] && ok ".env present" || warn ".env missing — run 'just env'"
[ "$(git config core.hooksPath)" = ".githooks" ] && ok "git hooks installed" || warn "hooks not installed — run 'just hooks'"

echo
[ $status -eq 0 ] && echo "ready." || echo "missing required tools — see above."
exit $status
