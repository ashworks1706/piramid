#!/usr/bin/env sh
set -eu

missing_tools=""

if ! command -v cargo >/dev/null 2>&1; then
    missing_tools="${missing_tools} cargo"
fi

if ! cargo fmt --version >/dev/null 2>&1; then
    missing_tools="${missing_tools} rustfmt"
fi

if ! cargo clippy --version >/dev/null 2>&1; then
    missing_tools="${missing_tools} clippy"
fi

if [ -n "$missing_tools" ]; then
    echo "Missing Rust dev tools:${missing_tools}" >&2
    echo "" >&2
    if ! command -v rustup >/dev/null 2>&1; then
        echo "Install Rust with rustup first: https://rustup.rs/" >&2
    fi
    echo "  rustup component add rustfmt clippy" >&2
    exit 1
fi

cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
