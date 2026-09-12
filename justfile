# Contributor tasks. Run `just` to list them.
#
# Not the shipped CLI. Nothing here is needed to run Piramid.

set shell := ["bash", "-euo", "pipefail", "-c"]

default:
    @just --list --unsorted

# ---------- first run ----------

# Check required tools, .env, and git hooks
doctor:
    ./scripts/doctor.sh

# Create .env from the example (no-op if it exists)
env:
    @[ -f .env ] && echo ".env exists" || { cp .env.example .env && echo "created .env"; }

# Install the pre-commit hook
hooks:
    git config core.hooksPath .githooks
    @echo "hooks installed: pre-commit, pre-push"

# Everything a fresh clone needs
bootstrap: env hooks setup
    @echo "ready: 'just serve' to run, 'just check' before you commit"

# Fetch dependencies for every unit
setup:
    cargo fetch
    cd apps/website && npm ci

# ---------- the gate ----------

# Format, lint, test, and verify layering. CI and the pre-commit hook run this.
check: check-rust check-website
    @echo "all units ok"

# fmt --check, clippy -D warnings, tests, dependency direction
check-rust:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
    ./scripts/check-deps.sh

# eslint over the website
check-website:
    cd apps/website && npm run lint

# Format every unit in place
fmt:
    cargo fmt --all
    cd apps/website && npx eslint . --fix

# ---------- feature matrices ----------

# Compile-check the GPU backend without a GPU present
check-gpu:
    cargo check --workspace --features gpu-cuda --all-targets

# Compile-check the inference backend
check-inference:
    cargo check --workspace --features inference-candle --all-targets

# Every feature combination CI builds
check-features: check-gpu check-inference
    cargo check --workspace --all-features --all-targets

# ---------- run ----------

# Run the server (defaults to 127.0.0.1:6333)
serve *ARGS:
    cargo run -p piramid -- serve {{ARGS}}

# The console: units, collections and config in one terminal UI
cli:
    cargo run -p piramid

# The two subcommands that run without a terminal: serve, support-bundle
piramid *ARGS:
    cargo run -p piramid -- {{ARGS}}

# ---------- docs, benches, security ----------

# Build rustdoc for the workspace; warnings are errors
doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features

# Open the docs
doc-open: doc
    cargo doc --workspace --no-deps --open

# Criterion benchmarks
bench *ARGS:
    cargo bench --workspace {{ARGS}}

# Advisories, licences, bans, sources
audit:
    cargo deny check advisories bans licenses sources

# ---------- website ----------

# Dev server with hot reload, http://localhost:3000
web:
    cd apps/website && npm run dev

# Production build. Catches type errors and prerender failures that `just web` does not.
web-build:
    cd apps/website && npm run build

# Build and serve the production bundle. `just web` hides prerender and font problems.
web-preview: web-build
    cd apps/website && npm run start

# Install website dependencies
web-setup:
    cd apps/website && npm ci

# Regenerate the landing-page animation from the CLI's frames
web-frames:
    ./scripts/sync-ascii-frames.py

# Screenshot the production build into target/screenshots. Needs google-chrome.
web-shots:
    #!/usr/bin/env bash
    set -euo pipefail
    command -v google-chrome >/dev/null || { echo "google-chrome not found"; exit 1; }
    out="target/screenshots"
    mkdir -p "$out"
    cd apps/website && npm run build >/dev/null
    npm run start >/dev/null 2>&1 &
    server=$!
    trap 'kill $server 2>/dev/null || true' EXIT
    for _ in $(seq 30); do
      curl -sf -o /dev/null http://localhost:3000/ && break
      sleep 0.5
    done
    cd - >/dev/null
    for page in "landing:/" "blogs:/blogs" "post:/blogs/history/piramid"; do
      name="${page%%:*}"
      path="${page#*:}"
      google-chrome --headless=new --disable-gpu --hide-scrollbars \
        --window-size=1440,900 --virtual-time-budget=4000 \
        --screenshot="$out/$name.png" "http://localhost:3000$path" 2>/dev/null
      echo "  $out/$name.png"
    done

# ---------- containers ----------

up *ARGS:
    docker compose -f deploy/compose.yml up -d {{ARGS}}

down:
    docker compose -f deploy/compose.yml down

logs *ARGS:
    docker compose -f deploy/compose.yml logs -f {{ARGS}}

# Production images from GHCR (PIRAMID_IMAGE_TAG=main|<sha>)
prod-up *ARGS:
    docker compose -f deploy/compose.yml -f deploy/compose.prod.yml pull
    docker compose -f deploy/compose.yml -f deploy/compose.prod.yml up -d {{ARGS}}

prod-down:
    docker compose -f deploy/compose.yml -f deploy/compose.prod.yml down

# Build images locally
images:
    docker build -f deploy/docker/piramid.Dockerfile -t piramid .

# ---------- cleanup ----------

clean:
    cargo clean
    rm -rf apps/website/node_modules apps/website/.next
