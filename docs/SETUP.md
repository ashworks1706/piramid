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
| `hf` (Hugging Face CLI) | downloading a model checkpoint | https://huggingface.co/docs/huggingface_hub |

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
just cli                      # the console: units, collections, config, device
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
2. a YAML or JSON file, named by `piramid serve --config` or, without that flag, by `CONFIG_FILE`
3. `PIRAMID__` environment variables
4. `piramid serve --port` and `--data-dir`, which replace the port of `startup.bind` and
   `startup.data_dir`

[`config.example.yaml`](../config.example.yaml) is the whole surface, every value at its default,
and a test asserts it stays that way. `startup.logging.config: true` logs what actually resolved.

The file has three blocks, split by when a setting takes effect. `startup:` is applied once at
boot, so changing a startup setting needs a restart, and `POST /api/config/reload` refuses a file
whose startup block differs from the running one. `runtime:` is re-read on reload, from the same
file and flags the server started with. `console:` is read when `piramid` starts with no
subcommand and opens the console.

Not every runtime setting reaches a collection that is already open. `search.parallel`, `limits`,
the WAL checkpoint thresholds (`checkpoint_frequency`, `checkpoint_interval_secs`,
`max_log_size`) and `execution` apply to open collections at once. `quantization`, `memory`,
`wal.enabled` and `wal.sync_on_write` are read when a collection opens, so a reload that changes
one of them is refused while any collection is open. `search.metric` is copied into a collection
when it is created and stored with it, so changing it affects only collections created afterwards.
`inference` is read once at startup, and a reload that changes it is refused.

Any key can also be set from the environment, spelled from its path: `runtime.wal.max_log_size`
is `PIRAMID__RUNTIME__WAL__MAX_LOG_SIZE`. Values parse as YAML, so `8`, `true` and `null` mean what
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

An embedding provider turns text into vectors. It is set under `startup.embedding` and is needed
by `/embed`, `/search/text` and retrieval in `/api/generate`. There are three providers.

`openai` speaks the OpenAI embeddings format. It covers OpenAI itself, with the key in
`OPENAI_API_KEY`, and any other server implementing the format, such as TEI, vLLM or llama.cpp, by
pointing `startup.embedding.base_url` at it and leaving `OPENAI_API_KEY` unset.

`ollama` speaks Ollama's own format. `just up ollama` starts an Ollama container beside the server.

`piramid` runs a Qwen3 embedding checkpoint inside the Piramid process. `model` is the checkpoint
directory, and `options` takes `device` (`cpu` or `cuda:N`), `dtype` and `max_tokens`. It needs a
build with `inference-candle`, and `gpu-cuda` as well for a CUDA device.

## A model and the server

Serving generation needs a build with `inference-candle`, plus `gpu-cuda` to run on a GPU, and a
Qwen2 or Qwen3 checkpoint on disk. The tests and benchmark below use Qwen2.5-0.5B-Instruct:

```bash
hf download Qwen/Qwen2.5-0.5B-Instruct --local-dir models/Qwen2.5-0.5B-Instruct
```

To run the server from source with a model, write a config file that sets
`runtime.inference.enabled: true`, `runtime.inference.model_path` and an embedding provider, as in
the [README quickstart](../README.md#quickstart), and pass the features through:

```bash
cargo run --release -p piramid --features inference-candle -- serve --config piramid.yaml
cargo run --release -p piramid --features inference-candle,gpu-cuda -- serve --config piramid.yaml
```

On the CPU, `runtime.inference.kv_cache.max_bytes` is required. On a GPU, set
`startup.hardware.profile: gpu` and `runtime.execution: gpu`; the model then loads onto
`cuda:N` at `startup.hardware.gpu.device_ordinal`. A build without `inference-candle` refuses
`runtime.inference.enabled: true` at startup. `GET /api/model` shows what loaded.

From there the path is the one a user follows. `POST /api/collections/{collection}/embed` embeds
texts with the configured provider and stores them, `POST /api/generate` with a `retrieval` block
searches a collection and answers from the passages it finds, and `POST /v1/chat/completions` serves
the same model to OpenAI clients without retrieval. Steps 5 to 7 of the
[README quickstart](../README.md#quickstart) show each request and its response. `just cli` opens
the console on the running server.

Generation on a real checkpoint has its own tests, marked ignored so the normal gate does not need
a model. `PIRAMID_TEST_MODEL` names the checkpoint directory:

```bash
PIRAMID_TEST_MODEL=models/Qwen2.5-0.5B-Instruct just test-model       # on the CPU
PIRAMID_TEST_MODEL=models/Qwen2.5-0.5B-Instruct just test-model-gpu   # on the CPU and CUDA
```

Both build in release mode and check that the model reproduces a stored reference output.

`just bench-rag` is the end-to-end RAG benchmark. For each question in a dataset it embeds,
searches, fetches passages, prefills and decodes, once per arm, writes the results as JSON to
`target/rag_e2e.json`, and prints a markdown table of them. The arms are `closed-book` (the
question alone), `before-prefill-http` (passages from a separate Piramid server),
`before-prefill-host` (an in-process collection scored on the CPU) and `before-prefill-device`
(the same, scored on the GPU). Three variables are required:

```bash
scripts/fetch-bench-dataset.sh 200     # HotpotQA questions as JSONL under target/bench

PIRAMID_BENCH_MODEL=models/Qwen2.5-0.5B-Instruct \
PIRAMID_BENCH_DATASET=target/bench/hotpotqa-dev-distractor.jsonl \
PIRAMID_BENCH_EMBEDDING='{"provider": "ollama", "model": "nomic-embed-text"}' \
just bench-rag
```

`scripts/fetch-bench-dataset.sh` checks the download against a pinned SHA-256 and stops on a
mismatch. The hash in the script is still an all-zero placeholder, so the first run stops and
prints the actual hash to verify and pin. The optional variables (`PIRAMID_BENCH_ARMS`,
`PIRAMID_BENCH_DEVICE`, `PIRAMID_BENCH_K`, `PIRAMID_BENCH_QUESTIONS` and others) are listed above
the recipe in the justfile. The `before-prefill-device` arm needs `PIRAMID_BENCH_DEVICE=cuda:0` and
`just bench-rag --features gpu-cuda`.

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

**`just: command not found`**: install from https://just.systems, or run the underlying cargo
commands directly. `just --list` shows what each recipe does.

**`check-deps: jq is required`**: install `jq`.

**Disk fills during builds**: the workspace `target/` directory grows quickly. `just clean`
removes it along with `node_modules`.

**Port 6333 already in use**: `PIRAMID__STARTUP__BIND=127.0.0.1:7333 just serve`.

**Where test data goes**: `target/tmp/`, via `CARGO_TARGET_TMPDIR`. Safe to delete.
