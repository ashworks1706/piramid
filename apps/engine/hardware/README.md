# piramid-hardware

The machine: the math, the device that runs it, and the encodings it runs over.

`compute` owns what a distance means and which strategy computes it. `gpu` owns talking to a
device — contexts, buffers, streams, module loading — and nothing about what the math means; vendor
SDK types like `cudarc` appear only in `gpu/backends/`. `quantization` owns the encodings
both score over, beside them rather than under either. `host` reads processor and memory use of
the machine and of this process; a reading the platform cannot take is absent, never zero.

A leaf crate: it depends on nothing else in the workspace, so kernels can be benchmarked on their
own and `model` can get a `Device` without reaching through retrieval math.

Adding a strategy is one file implementing `DistanceKernels` plus one arm in the registry. The
batch methods take a contiguous row-major slab and a caller-owned `out`, because that shape
uploads to a device in one copy. A slice of `Vec`s can't, and forces a per-call gather that costs
more than the kernel saves.

Dispatch never panics and never substitutes. A requested strategy that isn't available on this
machine or in this build returns `ComputeError::StrategyUnavailable`.

`unsafe` appears at two functions in `gpu/buffer.rs`, each with a `// SAFETY:` note.

Part of [Piramid](https://github.com/ashworks1706/piramid). See
[`docs/ARCHITECTURE.md`](../../../docs/ARCHITECTURE.md) for how the crates fit together.
