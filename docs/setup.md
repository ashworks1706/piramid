# Setup

This guide covers the full local setup for Piramid on Linux, macOS, and Windows through WSL2.
On Windows, use WSL2 with a Linux distro such as Ubuntu and run the same commands shown here.
This document does not cover PowerShell.

## Prerequisites

You will need Git, Rust, Node.js, and Python 3.10+.
Install Docker if you want the containerized workflow.

For Rust development, install the stable toolchain plus the standard formatting and linting components:

```bash
rustup toolchain install stable
rustup component add rustfmt clippy
```

Clone the repository and enter it:

```bash
git clone https://github.com/ashworks1706/piramid
cd piramid
```

## Main Rust App

Use this when you want the server and CLI running from source:

```bash
cargo run -- init
cargo run -- serve --data-dir ./data
```

For the same checks used by CI:

```bash
./scripts/check.sh
```

If you want local push protection, enable the hook once:

```bash
git config core.hooksPath .githooks
```

## Docker

Use Docker when you want a containerized server without installing the Rust toolchain:

```bash
docker build -t piramid .
docker run --rm -p 6333:6333 -v piramid-data:/app/data piramid
```

The repo also includes compose files:

```bash
docker compose up --build
```

## Website

The website lives in `website/` and uses Next.js:

```bash
cd website
npm ci
npm run dev
```

Useful checks:

```bash
npm run lint
npm run build
```

## Python SDK

The Python package lives in `sdk/pip/` and uses `pyproject.toml`:

```bash
cd sdk/pip
python -m venv .venv
source .venv/bin/activate
pip install -e .
```

## JavaScript Package

The npm package lives in `sdk/npm/`.
If you are working on that package directly, use the same Node.js toolchain as the website and install dependencies in that folder.

## Final Check

Before pushing, run the repo checks that cover Rust and the website:

```bash
./scripts/check.sh
./scripts/check-website.sh
```

If the pre-push hook is enabled, it will run both automatically.