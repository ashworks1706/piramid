# Decisions

The decisions behind the shape, compressed into one file. Eleven separate records used to live here;
they were written while the tree was still moving and described a layout that no longer exists, so
they were folded into this summary.

`docs/ARCHITECTURE.md` says what the shape *is*. This says why it was chosen and what was ruled out.
From here, a change that moves a boundary or forecloses an option gets a new numbered record beside
this file.

**One binary, layers as crates.** Rust checks a dependency rule across crate boundaries and never
inside one, so the layering is only real if each layer is a crate. `apps/cli` links them back into
the single executable users install.

**`apps/engine/` for the library tree.** `crates/` names Rust's compilation model rather than the
product. Everything authored lives under `apps/`, the SDKs included, and reaching a crate is two
levels with no grouping folders. A folder hierarchy that doesn't match the dependency rule is a
second model to keep in your head.

**Storage, search and collections share one crate.** Separating them is a cycle: a collection is
built on search, search reads storage, and storage holds a collection's bytes. The resident vectors
and metadata a collection keeps in memory live in the same crate for the same reason.

**Strategy-first compute dispatch.** One file per strategy, each implementing the whole
`DistanceKernels` trait. Adding hardware is a new file, not a new match arm in every kernel.

**`gpu` owns the device, `compute` owns the math.** Two subsystems need a device, and neither should
depend on the other to allocate memory.

**Commit to the fusion seam, not to a mechanism.** Picking chunked cross-attention or
residual-stream gating now would bake one answer into the driver before any of them has been
measured. `NoopRetrievalHook` is the control arm.

**Errors carry a kind, not a status code.** Every error crossing a crate boundary must be reachable
from `PiramidError`, so nothing has to be stringified to travel.

**One name, one meaning.** Folders are named for what they hold, never for a Rust construct, so there
is no `types/` and no `utils/`. Repeating a word is fine when it means the same thing at each layer and a bug
when it doesn't.

**Open standards only.** Prometheus and OTLP are protocols; a vendor's product is not. Telemetry
points at an endpoint the operator supplies and integrates with nothing by name.

**Nothing is silently ignored.** This is the rule the tree has broken most often: five settings once
shipped with no reader, one of them `wal.sync_on_write`, which the docs described as the durability
knob while the WAL never called `fsync`. A setting the build cannot honour is a startup error naming
the key.
