# Setup

Working on Piramid: Linux, macOS, or Windows through WSL2. For running it, see
[USAGE.md](USAGE.md).

## Prerequisites

| Tool | Needed for |
|---|---|
| Rust 1.87+ with rustfmt and clippy | everything |
| [`just`](https://just.systems) | every task |
| `jq` | `scripts/check-deps.sh` |
| Docker | `just up` |
| Node 20+ | the website |
| CUDA toolkit | `--features gpu-cuda` only |
| `hf` (Hugging Face CLI) | downloading checkpoints |

```bash
git clone https://github.com/ashworks1706/piramid && cd piramid
just bootstrap    # .env, git hooks, dependencies
just doctor       # ok, warn or miss per tool
```

## Daily loop

```bash
just cli            # the console with a units view: server, site, compose, every recipe
just serve          # the server on 127.0.0.1:6333
just check          # the gate: fmt, clippy, tests, layering, website
just check-rust     # Rust only
just fmt            # format in place
```

The pre-commit hook runs the gate for the units your staged changes touch. CI runs the same
recipes. A change is done when `just check` passes; fix failures at the source rather than adding
an `#[allow]` or skipping a test.

## Feature builds

```bash
just check-gpu          # compile-check gpu-cuda, no GPU needed
just check-inference    # compile-check inference-candle
just check-features     # both, plus --all-features
```

Without a GPU, candle's kernels need `CUDA_COMPUTE_CAP` set (CI uses 80). With a host gcc newer
than 15, `NVCC_CCBIN` has to name an older compiler.

## Model tests

Generation on a real checkpoint is tested behind `#[ignore]`:

```bash
hf download Qwen/Qwen2.5-0.5B-Instruct --local-dir models/Qwen2.5-0.5B-Instruct
PIRAMID_TEST_MODEL=models/Qwen2.5-0.5B-Instruct just test-model       # CPU
PIRAMID_TEST_MODEL=models/Qwen2.5-0.5B-Instruct just test-model-gpu   # CPU and CUDA
```

Both check the model reproduces a stored reference output.

## Benchmarks

```bash
just bench        # criterion, results in target/criterion
just doc          # rustdoc, warnings are errors
just audit        # cargo-deny
```

`just bench-rag` is the end-to-end RAG benchmark. Per question it embeds, searches, prefills and
decodes once per arm, writes `target/rag_e2e.json` and prints a table. The arms are `closed-book`,
`before-prefill-http` (a separate Piramid server), `before-prefill-host` (in-process, CPU scoring)
and `before-prefill-device` (GPU scoring, needs `PIRAMID_BENCH_DEVICE=cuda:0` and
`--features gpu-cuda`).

```bash
scripts/fetch-bench-dataset.sh 200     # HotpotQA questions under target/bench

PIRAMID_BENCH_MODEL=models/Qwen2.5-0.5B-Instruct \
PIRAMID_BENCH_DATASET=target/bench/hotpotqa-dev-distractor.jsonl \
PIRAMID_BENCH_EMBEDDING='{"provider": "ollama", "model": "nomic-embed-text"}' \
just bench-rag
```

The dataset script's SHA-256 is still a placeholder, so the first run stops and prints the hash to
pin. Optional variables are listed above the recipe in the justfile.

## Website

Next.js on React, TypeScript and Tailwind; not part of the workspace build.

```bash
just web-setup      # npm ci, once
just web            # dev server on :3000
just web-preview    # build and serve what deploys; check this before pushing
just web-shots      # headless screenshots into target/screenshots
just web-frames     # regenerate the logo animation from the CLI's frames
```

`node apps/website/scripts/render-mark.mjs` regenerates the favicon and logo images.

## Pull requests

`main` takes no direct pushes. Branch, push, open a PR; review threads must be resolved before
merge. One logical change per PR, with a test for new behaviour. Commit subjects are imperative and
under 72 characters, and the body says why. A change that moves a boundary or forecloses an option
gets a record in [decisions/](decisions/).

## Bug reports

Attach the file `piramid support-bundle` writes, run with the server's configuration, and the
smallest reproduction: collection size, filter and `k` for search; model, device and request body
for generation.

## Troubleshooting

- **Port 6333 in use**: `PIRAMID__STARTUP__BIND=127.0.0.1:7333 just serve`.
- **Disk fills during builds**: `just clean` removes `target/` and `node_modules`.
- **Test data**: goes to `target/tmp/` through `CARGO_TARGET_TMPDIR`; safe to delete.
