# Roadmap

This is the working roadmap for contributors. If you want to help, start here and pick one scoped task. If your idea is not listed but adjacent, open an issue first and propose it before implementation.


---

### Bug Fixes patch


**Quantization (1.1.0)**
- [ ] remove the HNSW vector cache eviction bug by making delete/update graph semantics explicit: either tombstone deleted IDs until rebuild, or rebuild/repair HNSW whenever stale graph nodes can be returned.
- [ ] remove or redesign the metadata cache so filtered search and re-ranking have one explicit consistency model instead of silently reading stale metadata.
- [ ] the quantization module already has PQ (Product Quantization) implemented -- it splits vectors into sub-blocks and compresses each independently, but it's not wired into search yet
- [ ] **FP16/BF16 vector precision:** promote `QuantizationLevel::Float16` from a stub to a real implementation -- store and serve vectors in native half-precision without upcasting to FP32, eliminating a costly precision-conversion step on the hot search path.
- [ ] Add Binary Quantization (BQ): Turning vectors into 1s and 0s for 32x speedups
- [ ] Add `GET /api/collections/:name/vectors/export?format=safetensors` that serializes the vector store in `.safetensors` format for interoperability with other tools.

---

### GPU Acceleration patch

**GPU backend:**

- [ ] decide what library to use
- [ ] create definite GPU struct and traits
- [ ] modularized service

**Introduce Custom GPU Kernels trait:**

- [ ] use the custom GPU kernels on quantizations, indexings, searchings, wherever required.
- [ ] Add a query optimizer that switches to Flat Search + Bitmaps when metadata filters are highly selective (>90% reduction)

---

### Transformer Inference Patch

**Introduce Transformer:**

- [ ] add support for running small transformer models.
- [ ] add kvcaching, batching and async support to the transformer inference module.
- [ ] add paged attention support for long contexts.
- [ ] add support for quantization.
- [ ] add streaming api.
- [ ] add an OpenAI-compatible chat/completions surface.

---

### Piramid Indexing and Searching Mechanism

- [ ] decide how this Transformer x Database Attention Fusion will work and experiemnt with latent attention
- [ ] identify what can be improved and develop better indexing + searching algorithm
- [ ] add dataset generation/fine-tuning workflows for better piramid transformer 
- [ ] add a WebSocket or SSE endpoint for continuous query submission so a client can push queries one at a time and receive results as they complete, enabling continuous batching without pre-grouping queries.
- [ ] add context-packing policies: max tokens, diversity, source caps, recency weighting, metadata constraints, and citation-preserving chunk joins.
- [ ] add this as default indexing + searching method for piramid, always have an option to switch to piramid as a normal database

---

### RAG Support

- [ ] add popular reranking mechanisms support for the pipeline, users should be able to toggle how and where they want what, already prebuilt by piramid
- [ ] add sparse/BM25 indexes alongside dense vectors as option.
- [ ] evaludate piramid indexing on benchmarks
- [ ] add RAG evals: retrieval recall, answer faithfulness, citation correctness, latency, memory, and cost per query.
- [ ] experiment with hybrid retrieval techniques: dense ANN + sparse/BM25 scoring + rerank, GraphRAG, RAPTOR, Cross-Encoders for our platform, completely abstracted and better
 
---

### Speed Optimizations

**CLI and Logs**

- [ ] piramid show config, piramid show metrics
- [ ] piramid init should automatically detect system's computational resources etc and setup the config accordingly
- [ ] all the query planning budget, optimizations, gpu selections, etc and everything should be directly reflected from that generated config 
- [ ] adaptive index tuning: auto-adjust `ef`, `nprobe`, `filter_overfetch` based on per-collection latency/recall budgets and density
- [ ] add hardware profiles (`8gb`, `16gb`, `32gb`, `cpu-only`, `gpu`) that choose index type, quantization, cache size, and search depth automatically.
- [ ] add all logs properly such as inference, indexing, searching, etc


**Query Features (1.1.3)**

- [ ] metadata-only search (no vector similarity)
- [ ] add `/query` and `/chat`-oriented APIs
- [ ] query result caching (LRU, TTL-based)

**Blocked / Future (Systems Optimization):**

- [ ] Automatically hydrate frequently accessed index clusters into memory on startup.
- [ ] Group multiple search requests into a single compute batch.


**Server & API (1.0.3)**

- [ ] read endpoints (GET collection, GET vector, search) silently create a new empty collection on disk if the name doesn't exist, instead of returning a 404. only write endpoints should be allowed to create collections.
- [ ] the embedding cache uses a blocking mutex inside async request handlers, which can stall the async runtime under load. should use an async-aware lock or be restructured to avoid holding it across await points.

**Filter & Cache Acceleration (1.1.2)**

- [ ] the collection map in AppState keeps every opened collection in memory forever with no eviction -- a server that opens many collections will grow unbounded. `cache_max_bytes` config exists but nothing enforces it. needs an LRU eviction policy so idle collections can be closed and their memory (vector cache, metadata cache, mmap) released.
- [ ] IVF prefiltering with metadata posting lists to avoid full-scan overfetch on filtered queries
- [ ] bitmap/roaring filters for post-filter paths; filter selectivity stats
- [ ] collection preloading on startup: optionally pre-open a configured list of collections rather than waiting for the first request
- [ ] Add LSH (Locality Sensitive Hashing) as a high-speed, low-RAM alternative to HNSW.
- [ ] IVF uses random centroid initialisation (first K vectors) -- k-means++ would sample proportionally to distance from the nearest existing centroid, producing better spread and fewer iterations to convergence

**Metadata Filters**

- [ ] metadata indexing for fast pre-filtering
- [ ] range queries on numeric fields
- [ ] regex/pattern matching on string fields
- [ ] date range filters
- [ ] array membership checks
- [ ] vector count per metadata filter
- [ ] complex boolean filters (AND/OR/NOT combinations)

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

### Documentation & Testing

**Documentation**

- [ ] Separate API docs to `docs.piramiddb.com` (Mintlify)
- [ ] document rust sdk with examples, link blogs from /blogs
- [ ] add an architecture note explaining Piramid's distinction from model-as-database systems: Piramid is database-as-inference-memory, not model-weight decompilation.
- [ ] add a research log for failed fusion experiments and kill-test results so contributors know which paths are dead ends.

**Platform**

- [ ] add python pypi
