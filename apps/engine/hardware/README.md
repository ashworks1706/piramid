# piramid-hardware

The machine: the math, the device that runs it, and the encodings it runs over.

`compute` owns what a distance means and which strategy computes it: scalar, SIMD, parallel and,
under `gpu-cuda`, CUDA. `compute::quantization` holds the vector encodings. `gpu` owns talking to a
device: opening it, buffers, streams, compiled kernels and the device memory budget the model and
retrieval share. It knows nothing about what the math means, and `cudarc` types appear only in
`gpu/backends/`. `host` reads processor, memory and GPU use of the machine and of this process,
with `nvml-wrapper` confined to `host/nvml.rs`; a reading the platform cannot take is absent, never
zero.

A leaf crate: it depends on nothing else in the workspace, so kernels can be benchmarked on their
own and `model` can get a `Device` without reaching through retrieval math.

Adding a strategy is one file implementing `DistanceKernels` plus one arm in the registry. The
batch methods take a contiguous row-major slab and a caller-owned `out`, because that shape
uploads to a device in one copy. A slice of `Vec`s can't, and forces a per-call gather that costs
more than the kernel saves.

Dispatch never panics and never substitutes. A requested strategy that isn't available on this
machine or in this build returns `ComputeError::StrategyUnavailable`.

`unsafe` appears at `as_bytes` and `as_bytes_mut` in `gpu/buffer.rs` and in the CUDA backend in
`gpu/backends/cudarc.rs`, each block with a `// SAFETY:` note.

Part of [Piramid](https://github.com/ashworks1706/piramid). See
[`docs/ARCHITECTURE.md`](../../../docs/ARCHITECTURE.md) for how the crates fit together.
