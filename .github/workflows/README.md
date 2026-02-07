# GitHub Actions Workflows

This directory contains CI/CD workflows for the Piramid project.

## Workflows

### 🦀 `rust.yml` - Rust CI Pipeline
**Triggers:** Push/PR to `main` and `dev` branches

**Jobs:**
- **Test Suite** - Runs tests on Ubuntu & macOS with stable & beta Rust
  - Runs tests with all features
  - Runs tests with no default features
  - Uses cargo caching for faster builds
  
- **Format Check** - Ensures code follows rustfmt standards
  
- **Clippy** - Lints code with clippy
  - Treats all warnings as errors
  
- **Build** - Cross-platform build verification (Linux, macOS, Windows)
  - Uploads artifacts for each platform
  
- **Benchmark** - Runs performance benchmarks on main branch
  - Stores benchmark history

### 📊 `dashboard.yml` - Dashboard CI Pipeline
**Triggers:** Push/PR to `main` and `dev` branches (only when dashboard files change)

**Jobs:**
- **Lint & Type Check** - ESLint + TypeScript validation
- **Build** - Next.js build and static export
  - Uploads build artifacts
- **Test** - Unit tests (disabled until tests are added)

### 🐳 `docker.yml` - Docker Build & Publish
**Triggers:** 
- Push to `main` branch
- Version tags (`v*`)
- Pull requests

**Jobs:**
- **Build and Push** - Multi-architecture Docker images
  - Builds for `linux/amd64` and `linux/arm64`
  - Publishes to GitHub Container Registry (ghcr.io)
  - Generates SBOM (Software Bill of Materials)
  - Uses layer caching for faster builds

**Image Tags:**
- `latest` - Latest main branch
- `v1.2.3` - Semver tags
- `v1.2` - Major.minor tags
- `v1` - Major version tags
- `main-abc1234` - Commit SHA tags

### 🔒 `security.yml` - Security Audits
**Triggers:**
- Push/PR to `main` branch
- Weekly schedule (Mondays at 9 AM UTC)

**Jobs:**
- **Cargo Audit** - Rust dependency vulnerability scanning
- **NPM Audit** - JavaScript dependency vulnerability scanning
- **Dependency Review** - GitHub's dependency review (PR only)

### 🚀 `release.yml` - Release Automation
**Triggers:** Version tags (`v*`)

**Jobs:**
- **Create Release** - Creates GitHub release
- **Build Release** - Cross-compilation for multiple platforms:
  - Linux: x86_64, aarch64
  - macOS: x86_64 (Intel), aarch64 (Apple Silicon)
  - Windows: x86_64
- **Publish Crate** - Publishes to crates.io (requires `CARGO_REGISTRY_TOKEN` secret)

## Required Secrets

### For Release Publishing
- `CARGO_REGISTRY_TOKEN` - Token from crates.io for publishing
  - Get it from: https://crates.io/settings/tokens
  - Add to: Repository Settings → Secrets → Actions

### For Docker Publishing
- `GITHUB_TOKEN` - Automatically provided by GitHub Actions
  - No setup required

## Permissions

The workflows require the following permissions:
- **Read:** contents, packages
- **Write:** packages (for Docker publishing)

These are configured in each workflow file.

## Caching

All workflows use GitHub Actions caching to speed up builds:
- **Rust:** Cargo registry, git dependencies, and build artifacts
- **Dashboard:** npm dependencies
- **Docker:** Build layers

## Local Testing

### Test Rust CI Locally
```bash
# Run all tests
cargo test --verbose --all-features

# Check formatting
cargo fmt --all -- --check

# Run clippy
cargo clippy --all-targets --all-features -- -D warnings

# Build release
cargo build --release
```

### Test Dashboard CI Locally
```bash
cd dashboard

# Install dependencies
npm ci

# Lint
npm run lint

# Type check
npx tsc --noEmit

# Build
npm run build
```

### Test Docker Build Locally
```bash
# Build for current platform
docker build -t piramid:local .

# Build multi-platform (requires buildx)
docker buildx build --platform linux/amd64,linux/arm64 -t piramid:local .
```

## Monitoring

- **Workflow runs:** https://github.com/YOUR_USERNAME/Piramid/actions
- **Docker images:** https://github.com/YOUR_USERNAME/Piramid/pkgs/container/piramid
- **Security alerts:** https://github.com/YOUR_USERNAME/Piramid/security

## Badge Examples

Add these to your README.md:

```markdown
![Rust CI](https://github.com/YOUR_USERNAME/Piramid/workflows/Rust%20CI/badge.svg)
![Dashboard CI](https://github.com/YOUR_USERNAME/Piramid/workflows/Dashboard%20CI/badge.svg)
![Docker Build](https://github.com/YOUR_USERNAME/Piramid/workflows/Docker%20Build%20%26%20Publish/badge.svg)
![Security Audit](https://github.com/YOUR_USERNAME/Piramid/workflows/Security%20Audit/badge.svg)
```
