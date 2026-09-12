# Setup

Local development on Linux, macOS, and Windows through WSL2. On Windows use WSL2 with a Linux
distribution and run the commands below; PowerShell isn't covered.

For running published images see [`deploy/README.md`](../deploy/README.md). For CI and release
workflows see [`.github/workflows/`](../.github/workflows/).

## Prerequisites

| Tool | Needed for | Where |
|---|---|---|
| Rust 1.87+ | everything | https://rustup.rs |
| `just` | every task | https://just.systems |
| `jq` | `scripts/check-deps.sh` | your package manager |
| Docker | `just up` | https://docs.docker.com/engine/install |
| Node 20+ | the website | https://nodejs.org |
| CUDA toolkit | `--features gpu-cuda` only | https://developer.nvidia.com/cuda-downloads |

The default build is CPU-only and needs no CUDA toolkit.

```bash
rustup toolchain install stable
rustup component add rustfmt clippy
```

## Clone and bootstrap

```bash
git clone https://github.com/ashworks1706/piramid
cd piramid
just bootstrap    # creates .env, installs git hooks, fetches dependencies
just doctor       # checks every tool above
```

`just doctor` prints ok, warn, or miss per tool and exits non-zero if a required one is missing.

## Run

```bash
just cli                      # the console: units, collections, config
just serve                    # just the server, on http://127.0.0.1:6333
just piramid support-bundle   # diagnostics for a bug report
```

Check it's up:

```bash
curl -s http://localhost:6333/api/health
```

## The gate

```bash
just check          # everything
just check-rust     # fmt, clippy, tests, layering only
just fmt            # format in place
```

The pre-commit hook, installed by `just hooks`, runs the gate for whichever units your staged
changes touch. `git commit --no-verify` skips it once.

## Configuration

Settings resolve in this order, with later winning:

1. defaults in `apps/engine/core/src/config`
2. a YAML or JSON file named by `CONFIG_FILE`
3. environment variables
4. `piramid serve --config`, `--port` and `--data-dir`, which name the file, replace the port of
   `startup.bind` and replace `startup.data_dir`

[`config.example.yaml`](../config.example.yaml) is the whole surface, every value at its default,
and a test asserts it stays that way. `startup.logging.config: true` logs what actually resolved.

The file has two blocks and the split is by lifecycle: `startup:` is applied once at boot, so
changing one of those needs a restart and `POST /config/reload` refuses a file whose startup block
differs from the running one. `runtime:` is re-read on reload.

Any key can also be set from the environment, spelled from its path — `runtime.cache.max_bytes`
is `PIRAMID__RUNTIME__CACHE__MAX_BYTES`. Values parse as YAML, so `8`, `true` and `null` mean what
they do in the file. `PIRAMID_API_KEY` and `OPENAI_API_KEY` are the settings that are
environment-only, so a key never lands in a file that gets shared.

## Authentication, rate limiting and shutdown

With `PIRAMID_API_KEY` set, every route except `/api/health` and `/api/readyz` requires
`Authorization: Bearer <key>`. The default bind is `127.0.0.1:6333`, which serves without a key.
Binding anything else with no key fails at startup; set the key, or set
`startup.http.auth.allow_unauthenticated: true` to serve an open port on purpose. The console sends
`PIRAMID_API_KEY` when it is set and reports a refused key as such.

`startup.http.rate_limit` is a token bucket per client IP; a request over it gets 429 with
`Retry-After`. On SIGINT or SIGTERM the server stops accepting connections, waits up to
`startup.http.drain_timeout_secs` for in-flight requests, checkpoints every open collection, and
exits 0, or non-zero if a checkpoint failed.

An unknown key, a misspelled one, a setting in the wrong block, and a setting that is not
implemented yet all fail at startup with a message naming the key. Nothing is silently ignored.

## Feature builds

```bash
just check-gpu          # compile-check --features gpu-cuda, no GPU needed
just check-inference    # compile-check --features inference-candle
just check-features     # both, plus --all-features
```

Features are additive and off by default. `runtime.execution: gpu` on a build without `gpu-cuda`
is rejected at startup, and so is a build with the feature but no device present.

## Docs and benchmarks

```bash
just doc          # rustdoc, warnings are errors
just doc-open     # and open it
just bench        # criterion, results in target/criterion
just audit        # cargo-deny: advisories, bans, licences, sources
```

## Embeddings

Every provider is an HTTP client; Piramid does not load a model. `startup.embedding.provider:
openai` speaks the OpenAI wire format, so it covers OpenAI itself and any server implementing
it — TEI, vLLM, llama.cpp — by pointing `startup.embedding.base_url` at it and leaving
`OPENAI_API_KEY` unset.
`ollama` speaks Ollama's own format. "Local" therefore means the model runs on your machine in
another process, not inside Piramid; an in-process provider is on the roadmap for v0.4.0.

## Website

Next.js 16 on React 19, TypeScript, Tailwind 4, and MDX for the blog. It is not part of the
workspace build and ships with nothing.

```bash
just web-setup      # npm ci, once
just web            # dev server with hot reload on :3000
just web-build      # production build
just web-preview    # build and serve what actually deploys
just web-shots      # headless screenshots into target/screenshots
just check-website  # eslint
just web-frames     # regenerate the landing animation from the CLI's frames
```

`just web` runs a dev bundle, which hides prerender and font-loading problems. Check `web-preview`
before pushing.

`web-shots` needs `google-chrome`. Reading the markup is not enough to review a design; it has
already caught a stylesheet that never loaded and an animation frozen on its first frame.

## Troubleshooting

**`just: command not found`** — install from https://just.systems, or run the underlying cargo
commands directly. `just --list` shows what each recipe does.

**`check-deps: jq is required`** — install `jq`.

**Disk fills during builds** — the workspace `target/` directory grows quickly. `just clean`
removes it along with `node_modules`.

**Port 6333 already in use** — `PIRAMID__STARTUP__BIND=127.0.0.1:7333 just serve`.

**Where test data goes** — `target/tmp/`, via `CARGO_TARGET_TMPDIR`. Safe to delete.
