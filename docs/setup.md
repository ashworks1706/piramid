# Setup

This guide covers the full local setup for Piramid on Linux, macOS, and Windows through WSL2.
On Windows, use WSL2 with a Linux distro such as Ubuntu and run the same commands shown here.
This document does not cover PowerShell.

## Prerequisites

You will need Git, Rust, Node.js, Python 3.10+, and Docker if you plan to use containers.

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

## Rust App

### From crates.io

```bash
cargo install piramid
piramid init
piramid serve --data-dir ./data
```

### From source

```bash
cargo run -- init
cargo run -- serve --data-dir ./data
```

The server defaults to `http://0.0.0.0:6333`.
Data is stored under `~/.piramid` by default; set `DATA_DIR` to override it.

## Configuration

Use `piramid.yaml` and environment variables when you need to tune the server.

```bash
piramid init --path piramid.yaml
piramid serve --config piramid.yaml
```

Common overrides:

```bash
PORT=7000 DATA_DIR=~/piramid-data piramid serve
CONFIG_FILE=~/piramid/piramid.yaml piramid serve
EMBEDDING_PROVIDER=openai OPENAI_API_KEY=sk-...
```

Key environment variables:

```bash
PORT=6333
DATA_DIR=/app/data
CONFIG_FILE=./piramid.yaml

EMBEDDING_PROVIDER=openai|local
EMBEDDING_MODEL=text-embedding-3-small
OPENAI_API_KEY=sk-...
EMBEDDING_BASE_URL=http://localhost:11434
EMBEDDING_TIMEOUT_SECS=15

DISK_MIN_FREE_BYTES=1073741824
DISK_READONLY_ON_LOW_SPACE=true
CACHE_MAX_BYTES=536870912
```

Minimal YAML sample:

```yaml
index:
  type: Auto
  metric: Cosine
  mode: Auto
search:
  filter_overfetch: 10
wal:
  enabled: true
  checkpoint_frequency: 1000
memory:
  use_mmap: true
limits:
  max_vectors: null
  max_bytes: null
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

## SDKs

### Python

The Python package lives in `sdk/pip/` and uses `pyproject.toml`:

```bash
cd sdk/pip
python -m venv .venv
source .venv/bin/activate
pip install -e .
```

### JavaScript

The npm package lives in `sdk/npm/`.
Use the same Node.js toolchain as the website and install dependencies in that folder.

## Development Checks

Run the same checks used by CI:

```bash
./scripts/check.sh
./scripts/check-website.sh
```

If you want local push protection, enable the hook once:

```bash
git config core.hooksPath .githooks
```

If the pre-push hook is enabled, it will run both checks automatically.