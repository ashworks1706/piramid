---
name: kernel-authoring
description: Add a compute strategy or GPU kernel to Piramid. Use when implementing a DistanceKernels strategy, writing CUDA kernels, touching compute or gpu, or working on vector memory layout and device transfer.
---

# kernel-authoring

How to add compute strategies and kernels without breaking the boundaries that make them swappable.

Read `docs/decisions/README.md` first — the compute dispatch, device/math split and vector
layout decisions are summarised there, and they explain why the shapes below are what they are.

## The boundary

`gpu` owns the device: contexts, buffers, streams, module loading. No math semantics.
`compute` owns the math: what cosine means, and which strategy runs it.

Both are leaf crates, depending on nothing else in the workspace. Don't add a dependency to either;
`scripts/check-deps.sh` will reject it, and the reason is that `inference` needs a device too and
must not reach through retrieval math to get one.

Vendor SDK types like `cudarc` appear only in `gpu/src/backends/`. If `cudarc::` shows up
anywhere else, the abstraction has leaked.

## Adding a CPU strategy

One file in `compute/src/strategies/`, one arm in `for_mode` in `strategies/mod.rs`. Nothing
else in the workspace changes. If you find yourself editing a third file, stop and reconsider.

1. Implement all of `DistanceKernels`: `mode`, `name`, `is_available`, the four pairwise methods
   and the three batch methods. The batch methods have no default; each validates with
   `check_batch_shape` and scores the slab with the strategy's own row kernel, hoisting whatever
   depends only on the query (the query norm, for cosine) out of the row loop.
2. Never delegate to another strategy. A strategy that cannot compute something is unavailable,
   not quietly served by a different one.
3. Add a variant to `ExecutionMode` and a string in `as_str`. serde is the only parser.
4. Add a parity test against `ScalarStrategy` — same inputs, results within `1e-5`.
5. Add a criterion bench against `ScalarStrategy`. A strategy with no measurement is a guess.

`is_available` has to be honest. `for_mode` refuses an unavailable strategy with
`StrategyUnavailable`, and a backend that lies produces wrong answers instead of an error.

## Adding a GPU kernel

The batch signature is the contract:

```rust
fn cosine_batch(&self, query: &[f32], candidates: &[f32], dim: usize, out: &mut [f32])
    -> ComputeResult<()>;
```

`candidates` is a contiguous row-major slab. Don't change this to `&[Vec<f32>]` — that's what it
used to be and it's unusable, because each row is a separate allocation, so every call would gather
into a staging buffer and the gather costs more than the kernel saves. `out` is caller-owned so the
buffer survives across queries and can be pinned later.

Order of work:

1. Device runtime first, kernels second. `Device`, `DeviceBuffer`, and `Stream` have to round-trip
   — allocate, upload, download, compare — before any kernel is written. A kernel debugged on top
   of broken transfer is unfixable.
2. Kernel source in `gpu/src/kernels/<family>.cu`, with a typed launch wrapper beside it
   in `<family>.rs`. The wrapper owns launch geometry and argument binding, not device lifetime.
3. Add `compute/src/strategies/cuda.rs` implementing the whole trait on the device, and replace
   the `Gpu` arm in `for_mode`, which refuses the mode until then. How a single-pair call is
   served is decided when that file lands; it does not silently run a CPU strategy.
4. `is_available` is a real device probe.

Keep data resident. The point of `DeviceBuffer` is that a candidate slab uploads once and gets
reused. If your benchmark uploads per query, you're measuring PCIe and CPU will win.

## Benchmarking

Always against the scalar reference, and always report both:

- Correctness: max absolute deviation from `ScalarStrategy`.
- Throughput: with the transfer cost included, if the data isn't already resident. A number that
  excludes the upload isn't the number the query path sees.

State the vector count, dimension, and whether data was device-resident. A speedup without those
can't be checked.

## Traps

- Measuring a GPU path against a scattered CPU baseline. Make `cache::VectorStore` contiguous first, or
  you'll credit the layout win to the device.
- `#[allow(unsafe_code)]` outside `gpu`. The security workflow fails on a new site.
- Panicking in a kernel. An unavailable strategy is a `ComputeError::StrategyUnavailable`; dispatch
  never panics and never substitutes another strategy.
- Adding a match arm instead of a file. If new hardware means editing existing dispatch code, the
  registry pattern has been broken.
