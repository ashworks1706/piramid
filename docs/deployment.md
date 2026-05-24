# Deployment

This document covers how to run Piramid from published artifacts and how those artifacts are produced.

For local development setup, use `docs/setup.md`. For CI and release internals, use `docs/devops.md`.

## Published Artifacts

A release can publish three kinds of artifacts:

- the Rust crate on crates.io
- Docker images on GitHub Container Registry
- platform binaries attached to the GitHub release

All three are built from the same release tag.

## Run From crates.io

Install the latest published crate:

```bash
cargo install piramid
piramid init
piramid serve --data-dir ./data
```

Install a specific version:

```bash
cargo install piramid --version 0.2.0
```

This path is useful when you want a local binary without Docker.

## Run From Docker

Pull the versioned image:

```bash
docker pull ghcr.io/ashworks1706/piramid:v0.2.0
```

Run the server with persistent storage:

```bash
docker run --rm \
  -p 6333:6333 \
  -v piramid-data:/app/data \
  ghcr.io/ashworks1706/piramid:v0.2.0
```

The container stores data under `/app/data`.

Use a bind mount if you want to control the host path directly:

```bash
docker run --rm \
  -p 6333:6333 \
  -v "$PWD/data:/app/data" \
  ghcr.io/ashworks1706/piramid:v0.2.0
```

## Docker Tags

Release images are pushed with these tags:

```text
vX.Y.Z
X.Y.Z
latest
sha-*
```

Use the explicit version tag for real deployments. `latest` is convenient for quick testing, but it moves every release.

## Configuration

The same runtime configuration works in Docker and non-Docker deployments.

Common container settings:

```bash
docker run --rm \
  -p 6333:6333 \
  -v piramid-data:/app/data \
  -e RUST_LOG=info \
  -e DATA_DIR=/app/data \
  -e CACHE_MAX_BYTES=536870912 \
  ghcr.io/ashworks1706/piramid:v0.2.0
```

For embedding providers, pass provider-specific environment variables:

```bash
docker run --rm \
  -p 6333:6333 \
  -v piramid-data:/app/data \
  -e EMBEDDING_PROVIDER=openai \
  -e OPENAI_API_KEY=sk-... \
  ghcr.io/ashworks1706/piramid:v0.2.0
```

Avoid baking secrets into images or config files. Pass secrets through environment variables or your deployment platform's secret manager.

## Compose

For local container runs from source:

```bash
docker compose up --build
```

For published images, point compose at the GHCR image instead of building locally.

Example service shape:

```yaml
services:
  piramid:
    image: ghcr.io/ashworks1706/piramid:v0.2.0
    ports:
      - "6333:6333"
    volumes:
      - piramid-data:/app/data
    environment:
      DATA_DIR: /app/data
      RUST_LOG: info

volumes:
  piramid-data:
```

## Health Checks

The Docker image has a health check against:

```text
http://localhost:6333/api/health
```

Useful runtime endpoints:

```text
/healthz
/readyz
/api/health
/api/metrics
```

Use readiness for traffic routing and health for process/container supervision.

## How Images Are Pushed

Docker images are not pushed on normal commits.

The release workflow pushes images only after:

- release checks pass
- the crate package verifies
- all release binaries build

This prevents a broken platform build from producing a public Docker release.

Images are pushed by GitHub Actions using `GITHUB_TOKEN` with package write permission.

## How Crates Are Pushed

The crate is pushed by GitHub Actions using:

```text
CARGO_REGISTRY_TOKEN
```

The token must be configured in GitHub repository secrets, or in the `crates-io` environment if that environment is used.

Once a version is published to crates.io, it cannot be overwritten. If a release fails after crates.io publish, bump the version and publish a follow-up patch.

## Rollback

For Docker deployments, rollback by using the previous version tag:

```bash
docker pull ghcr.io/ashworks1706/piramid:v0.1.1
```

For crates.io installs, install a specific older version:

```bash
cargo install piramid --version 0.1.1 --force
```

For data compatibility, always test the target version against a copy of production data before changing the running server.
