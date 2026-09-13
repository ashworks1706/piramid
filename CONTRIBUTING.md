# Contributing

## Setup

```bash
git clone https://github.com/ashworks1706/piramid && cd piramid
just bootstrap   # .env, git hooks, dependencies
just doctor      # check tooling
```

You need Rust (stable, 1.87 or newer), [`just`](https://just.systems), and `jq`. Docker, Node, and
a CUDA toolkit are optional; the default build is CPU-only and needs no vendor toolchain.

## The gate

```bash
just check
```

That runs `cargo fmt --check`, `clippy -D warnings`, the tests, `scripts/check-deps.sh`, and the
website lint. CI and the pre-commit hook run the same recipes, so if it passes locally it passes
in CI.

A change isn't done until it does. Fix failures at the source rather than adding an `#[allow]` or
skipping a test. If a lint is genuinely wrong for one case, the allow goes on the smallest
possible scope with a one-line reason.

## Before writing code

Read [AGENTS.md](AGENTS.md) for the layout and rules, and
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the crate boundaries and the three seams.

The one thing worth internalizing: a crate may depend on one below it in the dependency rule and
never the reverse. `scripts/check-deps.sh` enforces this. If your change needs a new edge, that's
a design conversation, so open an issue first.

## Where code goes

| If it is… | It belongs in |
|---|---|
| HTTP-specific | `apps/engine/serving/src/http` |
| A user-facing operation | `apps/engine/serving/src/services` |
| One collection's state | `apps/engine/database` |
| Bytes, mmap, WAL, sidecars, the manifest | `apps/engine/database/src/storage` |
| Vectors and metadata held in memory for search | `apps/engine/database/src/resident` |
| Scoring, filtering and ranking a query | `apps/engine/database/src/search` |
| Distance math or backend dispatch | `apps/engine/hardware` |
| Device memory, streams, kernels | `apps/engine/hardware/src/gpu` |
| Model execution | `apps/engine/model` |
| Shared vocabulary | `apps/engine/core` |

## Changing retrieval

Search is one exact scan. It scores the query against every stored vector, then keeps the best `k`
hits that pass the metadata filter. The scan lives in `apps/engine/database/src/search`, and the
vectors it reads come through the `VectorReader` trait in
`apps/engine/database/src/storage/vectors`. A change to how hits are scored, filtered or ranked
goes in `search`. A change that makes the scan faster on some hardware goes in a compute strategy,
described next. A change to where retrieval enters generation goes behind the `RetrievalHook`
trait in `apps/engine/model/src/fusion`, with any implementation that queries a collection in its
own crate.

Search stays an exact scan, and an approximate index is out of scope. The full list is in the Out
of scope section of [docs/ROADMAP.md](docs/ROADMAP.md).

## Adding a compute strategy

One file in `apps/engine/hardware/src/compute/strategies/` implementing `DistanceKernels`, and one
arm in the registry in `strategies/mod.rs`. Nothing else changes; that's what the trait is for.

The batch methods take a contiguous row-major slab and a caller-owned `out`. Don't change that to
`&[Vec<f32>]`, because scattered rows can't be uploaded to a device without a per-call gather that
costs more than the kernel saves.

New strategies need a parity test against `ScalarStrategy` and a bench against it.

## Good first issues

`docs/ROADMAP.md` has the open todos, and its Unscheduled section lists smaller known problems.
Backfilling doc comments so `missing_docs` can move from `warn` to `deny` in every crate, as it
already is in `hardware`, is self-contained.

## Commits and PRs

`main` is protected: it takes no direct pushes, so every change arrives as a pull request. Branch
from `main`, push the branch, open a PR. Force-pushing and deleting `main` are blocked, and review
threads must be resolved before merge.

Imperative subject under 72 characters. The body explains why, not what the diff already shows.
One logical change per PR, and new behaviour comes with a test. A change that moves a boundary or
forecloses an option gets a numbered record in `docs/decisions/`.

## Reporting bugs

Run `piramid support-bundle` with the same configuration as the server, and attach the file it
writes. It collects the version, platform, build, hardware, inference and embedding settings, the
resolved configuration and collection state in one pass, with credentials redacted. Read it before
sharing.

Add the smallest reproduction you can manage. For a search bug, the collection size, the filter and
`k` matter. For a generation bug, include the model, the device and the request body.
