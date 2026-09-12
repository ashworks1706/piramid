# Roadmap

## v0.3.0 — a retrieval path worth measuring

- [ ] route the IVF posting-list scan through the batch kernels without a gather copy: either a
      batch over the store slab by row index, or posting lists that own their rows
- [x] settle the execution modes against the bench: keep what wins, drop what doesn't
- [ ] a real CUDA device — allocate, upload, run a batch kernel, take the top-k on the device
- [ ] keep the candidate set device-resident across queries, and measure it against per-call upload
- [ ] quantize on the device, with recall reported alongside the speedup
- [ ] choose the index family per device: IVF where a device runs it, HNSW on the host
- [x] the server reports host CPU and memory in `/api/metrics` and `/metrics`, absent when the
      build can't measure them
- [ ] GPU memory, utilisation and temperature beside the host readings, absent when unmeasured
- [x] a device view in the console graphing host readings over time, against a local or remote
      server, with keys that hand the terminal to htop or nvtop for the local machine
- [ ] GPU readings graphed in the device view
- [x] make `serve` safe to expose — graceful shutdown, authentication, rate limiting, and a test
      that actually starts the server
- [x] clear the dependency debt: bincode 2.x with a read path for existing data, off the archived
      YAML parser
- [ ] implement the quantization levels or drop them; until then every quantization key is refused
      at its default

## v0.4.0 — the integrated baseline

One model, one GPU, batch size one, no HTTP.

- [ ] prove the model runtime and our device runtime can share one device with no host round trip
- [ ] run a model on the same device retrieval uses
- [ ] a forward-pass driver with the retrieval seam wired in from the first commit
- [ ] the first real `RetrievalHook` implementation, in its own crate
- [ ] an end-to-end benchmark: embed, search, fetch, prefill, decode — TTFT, tokens/sec, recall
- [ ] report TTFT, tokens/sec and retrieval-hook latency as metrics, graphed in the device view
- [ ] measure the configurations that matter against it, with retrieval-before-prefill as control
- [ ] publish the result

## v0.5.0 — `piramid serve`

Co-located RAG with unmodified models. Also the baseline v0.6 is measured against.

- [ ] serve a model and a collection from one command
- [ ] an inference endpoint and an OpenAI-compatible one, both streaming
- [ ] paged KV cache and continuous batching
- [ ] KV cache blocks used and free, evictions, hit rate, queue depth and batch size as metrics,
      with a KV panel in the device view
- [ ] embed in-process, reusing the device already held, beside the existing providers
- [ ] cut the website copy down to what the runtime does by then

## v0.6.0 — retrieval during generation

- [ ] hook retrieval into the decoder layers of a forked model
- [ ] run retrieval on its own stream, overlapped with model compute
- [ ] retrieve at block boundaries, measured against the v0.5 baseline at equal token budget
- [ ] take tokenization off the hot path, and reuse document KV state where that is sound
- [ ] a scheduler dividing the device between index, weights and KV under load, with that split
      shown in the device view

## v0.7.0+ — beyond one model

- [ ] fusion on models Piramid has not forked
- [ ] fuse retrieval and attention into fewer launches, once profiling says it is worth it
- [ ] an index co-designed for the attention access pattern
- [ ] half precision end to end, no upcasting on the hot path
- [ ] retrieval over the model's own KV history rather than external documents
- [ ] block-diffusion decoding, where a block is the retrieval unit

## Housekeeping

- [ ] backfill doc comments so `missing_docs` can move from `allow` to `warn`
- [x] make `runtime:` reload reach a running collection, or document that it doesn't
- [x] test config reload against a running server, not just the loader

## Unscheduled

- [ ] GPU settings are accepted and read by nothing: `gpu_memory_budget_bytes`, `gpu.*`, `vram.*`
- [ ] inference sub-settings (batching, kv_cache, sampling, fusion, document_kv, dtype, model_path)
      are accepted while inference is off and read by nothing
- [ ] the `auto` hardware profile means the same as `cpu-only` until something detects a GPU
- [ ] an `auto` index picks its family when the collection opens and keeps it as the collection
      grows past the thresholds
- [ ] behind a reverse proxy every client shares one rate-limit bucket; forwarded headers are not
      read
- [ ] `/api/readyz` is unauthenticated and names the data directory and every collection
- [ ] requests still running when the drain timeout expires continue after shutdown begins
- [ ] bincode is unmaintained upstream at every version (RUSTSEC-2025-0141); moving off it is a
      format change
- [ ] the embedding token total counts a provider that reports no usage as zero tokens
- [ ] the MSRV is not checked in CI against the new dependencies

