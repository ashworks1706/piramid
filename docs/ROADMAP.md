# Roadmap

## v0.3.0 — a retrieval path worth measuring

- [ ] route the IVF posting-list scan through the batch kernels, as the flat scan and the
      rerank already are
- [ ] settle the execution modes against the bench: keep what wins, drop what doesn't
- [ ] a real CUDA device — allocate, upload, run a batch kernel, take the top-k on the device
- [ ] keep the candidate set device-resident across queries, and measure it against per-call upload
- [ ] quantize on the device, with recall reported alongside the speedup
- [ ] choose the index family per device: IVF where a device runs it, HNSW on the host
- [ ] the server reports its device in `/api/metrics` and `/metrics`: GPU memory, utilisation and
      temperature, host CPU and memory. What the build can't measure is absent, never zero
- [ ] a device view in the console graphing those over time, against a local or remote server,
      with a key that hands the terminal to nvtop or htop for the local machine
- [x] make `serve` safe to expose — graceful shutdown, authentication, rate limiting, and a test
      that actually starts the server
- [ ] clear the dependency debt: bincode 2.x with a read path for existing data, off the archived
      YAML parser, and either implement the unimplemented quantization levels or drop them

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
- [ ] make `runtime:` reload reach a running collection, or document that it doesn't
- [ ] test config reload against a running server, not just the loader

