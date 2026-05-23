# Roadmap

This is the working roadmap for contributors. If you want to help, start here and pick one scoped task. If your idea is not listed but adjacent, open an issue first and propose it before implementation.

Piramid's north star is a consumer-hardware inference database: start as a reliable single-binary RAG database, it is making mutable database memory usable by transformer inference without copying every retrieved chunk into the prompt.

---

### Target Architecture patch

**Module Boundaries**

- [ ] keep `storage/` focused on record store, WAL, mmap, persistence formats, checksums, recovery, and file growth.
- [ ] keep `index/` focused on vector index implementations, index configs, index persistence, and index-specific query knobs.
- [ ] add `compute/` or `kernels/` for CPU/GPU distance kernels, backend dispatch, batching, SIMD/JIT/GPU implementations, and benchmark gates.
- [ ] add `cache/` for shared cache policy, memory budgeting, eviction strategy, query/result caches, embedding cache, and future KV-cache accounting.
- [ ] add `inference/` for model placement, local inference adapters, batching, streaming, KV-cache ownership, and OpenAI-compatible inference APIs.
- [ ] add `cluster/` for distributed membership, node capability discovery, shard ownership, replication policy, fan-out routing, and partial-result handling.

---

### Architecture Refactor patch

**Vector Reader**

- [ ] introduce a `VectorReader` trait for vector access by ID and iteration over live vectors, with mmap-backed and cache-backed implementations.
- [ ] change `VectorIndex::insert` and `VectorIndex::search` to depend on `VectorReader` instead of `HashMap<Uuid, Vec<f32>>`.
- [ ] keep `HashMap` adapter only for tests and simple in-memory benchmarks.
- [ ] use the `VectorReader` boundary as the future integration point for quantized traversal, GPU distance kernels, and distributed/sharded vector access.


---

### Bug Fixes patch

**Storage (1.0.1)**

- [ ] finding the next write offset scans every existing entry on every insert to find the maximum. this should just be a counter that increments as vectors are added.

**IVF Index (1.0.2)**

- [ ] during the IVF bootstrap phase, if not enough vectors have been inserted yet to form clusters, new vectors are silently dropped without any error or warning -- they're just lost. vectors should be held in a buffer and replayed into the index once clustering is ready.
- [ ] IVF checks for duplicate vector IDs by scanning the cluster list on every insert, which gets slow as clusters grow. it can use the existing ID-to-cluster map instead for an instant lookup.

---

### GPU Acceleration patch

**GPU backend:**

- [ ] **WGPU Implementation:** Wire the `Cpu` and future parallel backends to dispatch distance-calc batches.
- [ ] attempt accelerated initialization on boot, fallback to baseline on failure (graceful degrade)
- [ ] keep CPU as the correctness baseline for every GPU path; GPU acceleration should never change result ordering outside documented approximation tolerances.

**Introduce Custom GPU Kernels trait:**

- [ ] index traversal must dispatch distance computation through a pluggable backend abstraction, enabling future parallelism improvements.
- [ ] Add a query optimizer that switches to Flat Search + Bitmaps when metadata filters are highly selective (>90% reduction)
- [ ] Replace custom index serialization with rkyv for zero-copy, instant-load index access from mmap.
- [ ] Add LSH (Locality Sensitive Hashing) as a high-speed, low-RAM alternative to HNSW.
- [ ] Add Binary Quantization (BQ): Turning vectors into 1s and 0s for 32x speedups
- [ ] add a CPU/GPU backend benchmark harness that reports p50/p95/p99 latency, recall@k, memory usage, and index build time on consumer hardware presets.

**Safetensors / precision compatibility:**

- [ ] **Safetensors-compatible vector export:** Add `GET /api/collections/:name/vectors/export?format=safetensors` that serializes the vector store in `.safetensors` format for interoperability with other tools.

**Blocked / Future (Systems Optimization):**

- [ ] **Warm Index Mirroring:** Automatically hydrate frequently accessed index clusters into memory on startup.
- [ ] **Batched Retrieval Dispatch:** Group multiple search requests into a single compute batch.

---

### Index Quality patch

**Quantization Refactor (1.1.0)**
- [ ] quantization currently happens at insert time in the storage layer, which permanently throws away the original vectors. this causes two problems: search gets no speed benefit because the index still fetches full float vectors during traversal anyway, and final scores are calculated from a lossy reconstruction instead of the originals. the fix is to store raw vectors in storage as the source of truth, move quantization inside the index so it accelerates graph traversal, and re-rank the final small candidate set using the original floats. as part of this: remove the upsert double-quantize path (storage no longer quantizes at all), remove the HNSW vector cache eviction bug (vector cache gets deleted entirely), and remove the metadata cache (re-ranking reads metadata from mmap for free alongside the vector).
- [ ] the quantization module (`src/quantization/`) already has PQ (Product Quantization) implemented -- it splits vectors into sub-blocks and compresses each independently, giving much better compression than scalar quantization. but it's not wired into search yet. once connected, index traversal would use fast lookup-table distance math instead of full dot products, dropping search compute ~8× and memory per vector ~32×, while the re-ranking step on final candidates keeps recall high.
- [ ] **FP16/BF16 vector precision:** promote `QuantizationLevel::Float16` from a stub to a real implementation -- store and serve vectors in native half-precision without upcasting to FP32, eliminating a costly precision-conversion step on the hot search path.
- [ ] add a quantization acceptance suite: each quantized index must report recall loss, latency gain, memory reduction, and rerank recovery against the raw-float baseline.

**Index Improvements (1.1.1)**

- [ ] IVF uses random centroid initialisation (first K vectors) -- k-means++ would sample proportionally to distance from the nearest existing centroid, producing better spread and fewer iterations to convergence
- [ ] adaptive index tuning: auto-adjust `ef`, `nprobe`, `filter_overfetch` based on per-collection latency/recall budgets and density
- [ ] add hardware profiles (`8gb`, `16gb`, `32gb`, `cpu-only`, `gpu`) that choose index type, quantization, cache size, and search depth automatically.

---

### Searching patch

**Server & API (1.0.3)**

- [ ] read endpoints (GET collection, GET vector, search) silently create a new empty collection on disk if the name doesn't exist, instead of returning a 404. only write endpoints should be allowed to create collections.
- [ ] the embedding cache uses a blocking mutex inside async request handlers, which can stall the async runtime under load. should use an async-aware lock or be restructured to avoid holding it across await points.

**Filter & Cache Acceleration (1.1.2)**

- [ ] the collection map in AppState keeps every opened collection in memory forever with no eviction -- a server that opens many collections will grow unbounded. `cache_max_bytes` config exists but nothing enforces it. needs an LRU eviction policy so idle collections can be closed and their memory (vector cache, metadata cache, mmap) released.
- [ ] IVF prefiltering with metadata posting lists to avoid full-scan overfetch on filtered queries
- [ ] bitmap/roaring filters for post-filter paths; filter selectivity stats
- [ ] collection preloading on startup: optionally pre-open a configured list of collections rather than waiting for the first request
- [ ] add sparse/BM25 indexes alongside dense vectors; vector-only retrieval is not strong enough for production RAG.

**Query Features (1.1.3)**

- [ ] query result caching (LRU, TTL-based)
- [ ] query planning/optimization; query budget enforcement (timeouts, complexity limits)
- [ ] add a query planner that can choose dense ANN, sparse/BM25, metadata prefilter, hybrid fusion, rerank, or flat scan based on selectivity and latency budget.
- [ ] add result attribution metadata so downstream inference can trace each answer token/span back to retrieved records.

**Metadata Filters**

- [ ] metadata indexing for fast pre-filtering
- [ ] range queries on numeric fields
- [ ] regex/pattern matching on string fields
- [ ] date range filters
- [ ] array membership checks
- [ ] vector count per metadata filter
- [ ] complex boolean filters (AND/OR/NOT combinations)

**Search API Extensions (1.1.7)**

- [ ] **Streaming search interface:** add a WebSocket or SSE endpoint for continuous query submission so a client can push queries one at a time and receive results as they complete, enabling continuous batching without pre-grouping queries.
- [ ] hybrid retrieval: dense ANN + sparse/BM25 scoring + rerank
- [ ] metadata-only search (no vector similarity)
- [ ] recommendation API (similar to these IDs, not those)
- [ ] add `/query` and `/chat`-oriented APIs that combine retrieval, reranking, context packing, citations, and optional local inference in one request.
- [ ] add context-packing policies: max tokens, diversity, source caps, recency weighting, metadata constraints, and citation-preserving chunk joins.

---

### Write Path & Durability

**Async I/O (1.1.4)**

- [ ] non-blocking writes via `tokio-fs`
- [ ] async write pipeline: batching/coalescing, buffered writes, background flush worker
- [ ] prefetching for sequential reads
- [ ] background job queue for long-running storage operations

**Crash Safety & Recovery (1.1.5)**

- [ ] WAL and all persistence file formats need version fields and checksums; recovery paths must handle partial writes and format mismatches safely
- [ ] dry-run config validation on startup; fail-fast on mismatched config between what's stored and what's loaded
- [ ] automatic index rebuild from WAL on detected corruption

---

### Operations & Reliability (1.1.9)

- [ ] set up benchmarks for latency, index strategies, memory usage across collection sizes
- [ ] publish benchmark profiles for consumer machines: laptop CPU-only, integrated GPU, 16GB RAM, 32GB RAM, and small home server.
- [ ] add regression gates for p95/p99 search latency, recall@k, WAL recovery time, compaction time, and memory ceiling.
- [ ] add end-to-end RAG benchmark reports comparing Piramid against a strong baseline stack (vector DB + BM25 + reranker + local LLM).

---

### Transformer Inference Patch

**Introduce Transformer:**
- [ ] add support for running small transformer models, but scope the first version to correctness and integration rather than competing with llama.cpp/vLLM kernels.
- [ ] add kvcaching, batching and async support to the transformer inference module.
- [ ] add paged attention support for long contexts.
- [ ] add support for quantization.
- [ ] add streaming api.
- [ ] add an OpenAI-compatible chat/completions surface so Piramid can be used as a drop-in local RAG inference server.
- [ ] add a baseline mode that uses normal prompt-RAG: retrieve, rerank, pack context, stream answer. this is the baseline every fusion experiment must beat.
- [ ] add inference benchmarks against external local runtimes (Ollama/llama.cpp-style OpenAI-compatible servers) so Piramid does not accidentally spend months rebuilding a worse inference engine.

---

### Distributed Systems & Inference Patch

**Distributed Runtime:**

- [ ] add a node runtime abstraction with stable node IDs, advertised capabilities, heartbeat state, and graceful shutdown semantics.
- [ ] add cluster membership for small trusted deployments first: static config, explicit join/leave, health checks, and no automatic rebalancing until failure semantics are tested.
- [ ] define placement policies for consumer hardware: CPU-only node, integrated GPU node, discrete GPU node, storage-heavy node, and mixed laptop/home-server profiles.
- [ ] add request routing that chooses local execution by default and only crosses the network when the latency budget justifies it.

**Distributed Search & Storage:**

- [ ] shard collections by vector ID or partition key, with deterministic routing and a clear single-node fallback path.
- [ ] add replicated read-only shards for hot collections before adding distributed writes; correctness should not depend on consensus in the first version.
- [ ] add fan-out search across shards with top-k merge, timeout budgets, partial-result reporting, and per-shard latency attribution.
- [ ] add snapshot shipping and mmap-friendly shard loading so a second node can serve a collection without rebuilding the index from scratch.

**Distributed Inference:**

- [ ] add model placement metadata: model name, quantization, context length, KV-cache capacity, backend, GPU memory, and supported batch sizes.
- [ ] add distributed inference routing for prompt-RAG first: retrieve locally or remotely, pack context, send to the best available inference node, and stream tokens back.
- [ ] add continuous batching across clients on each inference node, but keep admission control explicit so one long generation cannot starve short RAG queries.
- [ ] add KV-cache locality policies so follow-up chat turns route to the node that already owns the session cache when possible.
- [ ] prototype tensor/pipeline parallel inference only after single-node inference is benchmarked; reject it unless it beats simpler model replication on consumer networks.

**Reliability & Observability:**

- [ ] add distributed tracing across retrieve, rerank, context-pack, inference-prefill, inference-decode, and response-stream phases.
- [ ] expose cluster metrics: node health, shard ownership, queue depth, GPU memory, KV-cache usage, network fan-out time, and partial-result rates.
- [ ] add failure-mode tests for node loss, slow shard, stale replica, interrupted stream, duplicated request, and model-node overload.
- [ ] document the distributed-system boundary clearly: Piramid should scale from single binary to small trusted clusters before attempting internet-scale database semantics.

---

### Transformer x Database Attention Fusion Patch

- [ ] write the fusion spec first: define which space database memory lives in (embedding space, residual stream space, per-layer K/V space, or learned adapter space). do not assume ordinary embedding vectors are valid transformer keys/values.
- [ ] add a kill-test benchmark before transformer surgery: hybrid retrieval + rerank + prompt injection must be the baseline, and fusion must beat it on answer quality, latency, and memory on the same hardware.
- [ ] implement model-aware memory records: raw text, metadata, dense vector, sparse/BM25 terms, source spans, and optional per-model memory tensors.
- [ ] prototype learned adapters that map retrieved chunks/vectors into model-usable memory tokens or K/V-like states without changing the base model.
- [ ] only after adapters beat prompt-RAG, modify transformer blocks with configurable key/value projection heads for database vectors.
- [ ] learnable gating mechanism to balance attention between internal context and external memory.
- [ ] efficient retrieval of relevant database vectors per query (e.g. via ANN search) to keep attention tractable.
- [ ] cross attention with database vectors as keyvalues with query from transformer.
- [ ] add ablations for retrieval frequency: per-request, per-chunk, per-layer, and per-token. per-token retrieval should be rejected unless it proves latency-safe on consumer hardware.
- [ ] document dead ends from RETRO/REALM/RAG/kNN-LM-style systems so Piramid does not repeat expensive research paths without evidence.

---

### RAG Features

**Variations**

- [ ] implement GraphRAG as native option, but keep it as a memory-building/retrieval strategy rather than the core identity of Piramid.
- [ ] add RAPTOR as an optional hierarchical summarization/indexing strategy.
- [ ] add latent rag experiments behind a feature flag until they beat normal hybrid retrieval in evals.
- [ ] add RAFT-style dataset generation/fine-tuning workflows as an optional training pipeline, not as a requirement for basic Piramid usage.
- [ ] Implement Cross-Encoders: a tiny built-in ML model to re-score the final top 10 results (provide options: ColBERT-style late interaction, small cross-encoder, or external reranker).
- [ ] add citations/source-span tracking as a first-class response primitive; every answer path should be able to explain which records influenced it.
- [ ] add RAG evals: retrieval recall, answer faithfulness, citation correctness, latency, memory, and cost per query.

---

### Documentation & Testing

**Documentation**

- [ ] Separate API docs to `docs.piramiddb.com` (Mintlify)
- [ ] document rust sdk with examples, link blogs from /blogs
- [ ] add an architecture note explaining Piramid's distinction from model-as-database systems: Piramid is database-as-inference-memory, not model-weight decompilation.
- [ ] add a research log for failed fusion experiments and kill-test results so contributors know which paths are dead ends.

**Platform**

- [ ] add python pypi
