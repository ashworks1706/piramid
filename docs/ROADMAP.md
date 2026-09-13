# Roadmap

## v0.3.0: a retrieval path worth measuring

- [ ] keep a collection's vectors resident on the device across queries, so the exact scan does not
      upload them on every call, and measure it against per-call upload
- [ ] quantize the resident vectors on the device, with recall against the unquantized exact scan
      reported alongside the speedup
- [ ] implement the quantization levels or drop them; until then every quantization key is refused
      at its default

## v0.4.0: the integrated baseline

One model, one GPU, batch size one, no HTTP.

- [x] prove the model runtime and our device runtime can share one device with no host round trip
- [x] run a model on the same device retrieval uses
- [ ] the first real `RetrievalHook` implementation, in its own crate
- [ ] an end-to-end benchmark: embed, search, fetch, prefill, decode, reporting TTFT,
      tokens/sec and recall
- [ ] report TTFT, tokens/sec and retrieval-hook latency as metrics, graphed in the device view
- [ ] measure the configurations that matter against it, with retrieval-before-prefill as control
- [ ] publish the result

## v0.5.0: `piramid serve`

Co-located RAG with unmodified models. Also the baseline v0.6 is measured against.

- [x] embed in-process, reusing the device already held, beside the existing providers
- [ ] cut the website copy down to what the runtime does by then

## v0.6.0: retrieval during generation

- [ ] hook retrieval into the decoder layers of a forked model
- [ ] run retrieval on its own stream, overlapped with model compute
- [ ] retrieve at block boundaries, measured against the v0.5 baseline at equal token budget
- [ ] take tokenization off the hot path, and reuse document KV state where that is sound
- [ ] a scheduler dividing the device between document vectors, weights and KV under load, with that
      split shown in the device view

## v0.7.0+: beyond one model

- [ ] fusion on models Piramid has not forked
- [ ] fuse retrieval and attention into fewer launches, once profiling says it is worth it
- [ ] a layout for document vectors co-designed for the attention access pattern
- [ ] half precision end to end, no upcasting on the hot path
- [ ] retrieval over the model's own KV history rather than external documents
- [ ] block-diffusion decoding, where a block is the retrieval unit

## Unscheduled

- [x] the `auto` hardware profile means the same as `cpu-only` until something detects a GPU
- [ ] behind a reverse proxy every client shares one rate-limit bucket; forwarded headers are not
      read
- [ ] `/api/readyz` is unauthenticated and names the data directory and every collection
- [ ] requests still running when the drain timeout expires continue after shutdown begins
- [ ] bincode is unmaintained upstream at every version (RUSTSEC-2025-0141); moving off it is a
      format change
- [ ] the embedding token total counts a provider that reports no usage as zero tokens
- [ ] the MSRV is not checked in CI against the new dependencies
- [ ] `/api/collections` and `/api/metrics` list open collections only, while `/api/readyz` also
      lists collections on disk; after a restart the list is empty until something opens them
- [ ] about sixteen error variants are never constructed
- [ ] `model::embeddings` and `core::observability` re-export other modules' items, against the
      one-canonical-path rule
- [ ] `RetrievalHook` can only change hidden states: `join` has no way to return passages for the
      driver to place in the KV cache, and `RetrievalRequest` carries token ids but no text to embed
- [ ] attention gathers a sequence's pages into a contiguous buffer on every layer of every step;
      there is no paged-attention kernel
- [ ] the device hidden state handed to a hook is a converted f32 copy when the weights are not f32,
      written back after the hook returns
- [ ] a CUDA pairwise score that fails returns NaN, because `DistanceKernels` pairwise methods are
      infallible
- [ ] candle's CUDA kernels do not build with a host gcc newer than 15; `NVCC_CCBIN` has to name an
      older compiler
- [ ] the benchmark dataset script carries a placeholder sha256 until a checked download pins it
- [ ] `/api/generate` retrieval formats passages one fixed way, in a system message or before a raw
      prompt; there is no template setting
- [ ] a collection written by Piramid 0.2 is refused at open and has to be re-ingested; its
      `.vecindex.db` file stays on disk until the collection is deleted

## Out of scope

- approximate nearest neighbour index families such as HNSW and IVF
- multi-node clustering and routing
- near-duplicate search and range search
- general vector database features that do not serve retrieval for generation
