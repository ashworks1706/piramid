# Roadmap

This is the working roadmap for contributors. If you want to help, start here and pick one scoped task. If your idea is not listed but adjacent, open an issue first and propose it before implementation. GPU acceleration is a Phase-2 optional capability — Piramid is fully functional without it. GPU work begins after base piramid tasks are complete.

---

### Bug Fixes patch

**Storage (1.0.1)**

- [ ] if mmap is disabled in the collection config, the storage layer panics on the first write that needs to grow the file. needs a proper guard instead of an unconditional unwrap.
- [ ] updating a vector or its metadata writes three separate WAL log entries per operation (update + delete + insert) instead of one. this inflates the WAL unnecessarily and makes recovery replay more expensive than it needs to be.
- [ ] index serialization uses unsafe pointer casts to convert the index trait object to a concrete type — if the wrong type is ever stored this is undefined behaviour. should use a safe serialization method on the index trait instead.
- [ ] the index pointer file (`.index.db`) is fully rewritten on every single insert, update, or delete, while the ANN graph file (`.vecindex.db`) is only written at checkpoints. this inconsistency means the two files can describe different states between checkpoints — if the WAL is disabled or corrupted during that window, the ANN graph silently misses entries with no error. both files should flush together at checkpoint time only, with the WAL as the only per-mutation write.
- [ ] finding the next write offset scans every existing entry on every insert to find the maximum. this should just be a counter that increments as vectors are added.

**IVF Index (1.0.2)**

- [ ] during the IVF bootstrap phase, if not enough vectors have been inserted yet to form clusters, new vectors are silently dropped without any error or warning — they're just lost. vectors should be held in a buffer and replayed into the index once clustering is ready.
- [ ] IVF checks for duplicate vector IDs by scanning the cluster list on every insert, which gets slow as clusters grow. it can use the existing ID-to-cluster map instead for an instant lookup.

**Server & API (1.0.3)**

- [ ] read endpoints (GET collection, GET vector, search) silently create a new empty collection on disk if the name doesn't exist, instead of returning a 404. only write endpoints should be allowed to create collections.
- [ ] the embedding cache uses a blocking mutex inside async request handlers, which can stall the async runtime under load. should use an async-aware lock or be restructured to avoid holding it across await points.

---

### Index Quality patch

**Quantization Refactor (1.1.0)**
- [ ] quantization currently happens at insert time in the storage layer, which permanently throws away the original vectors. this causes two problems: search gets no speed benefit because the index still fetches full float vectors during traversal anyway, and final scores are calculated from a lossy reconstruction instead of the originals. the fix is to store raw vectors in storage as the source of truth, move quantization inside the index so it accelerates graph traversal, and re-rank the final small candidate set using the original floats. as part of this: remove the upsert double-quantize path (storage no longer quantizes at all), remove the HNSW vector cache eviction bug (vector cache gets deleted entirely), and remove the metadata cache (re-ranking reads metadata from mmap for free alongside the vector).
- [ ] the quantization module (`src/quantization/`) already has PQ (Product Quantization) implemented — it splits vectors into sub-blocks and compresses each independently, giving much better compression than scalar quantization. but it's not wired into search yet. once connected, index traversal would use fast lookup-table distance math instead of full dot products, dropping search compute ~8× and memory per vector ~32×, while the re-ranking step on final candidates keeps recall high.
- [ ] **FP16/BF16 vector precision:** promote `QuantizationLevel::Float16` from a stub to a real implementation — store and serve vectors in native half-precision without upcasting to FP32. Required for zero-copy handoff to GPU kernels (wgpu WGSL shaders operate in FP16/BF16), eliminating a costly precision-conversion step on the hot search path.

**Index Improvements (1.1.1)**

- [ ] IVF uses random centroid initialisation (first K vectors) — k-means++ would sample proportionally to distance from the nearest existing centroid, producing better spread and fewer iterations to convergence
- [ ] adaptive index tuning: auto-adjust `ef`, `nprobe`, `filter_overfetch` based on per-collection latency/recall budgets and density
- [ ] background index maintenance: online HNSW compaction, tombstone cleanup, IVF cluster rebalancing without blocking reads
- [ ] circuit breaker for embedding API failures with fallback behaviour

**Introduce ComputeBackend trait (Cpu | Gpu):**

- [ ] index traversal must dispatch distance computation through a backend abstraction. Design the handshake for the GPU backend to take ownership of distance-calc batches.
- [ ] Add a query optimizer that switches to Flat Search + Bitmaps when metadata filters are highly selective (>90% reduction)
- [ ] Implement Logical Namespacing to allow multiple users to share one Collection/Index without cross-talk or performance degradation.
- [ ] Replace custom index serialization with rkyv for zero-copy, instant-load index access from mmap.
- [ ] Add LSH (Locality Sensitive Hashing) as a high-speed, low-RAM alternative to HNSW.
- [ ] add Annoy
- [ ] Add Binary Quantization (BQ): Turning vectors into 1s and 0s for 32x speedups
- [ ] Implement Cross-Encoders: A tiny built-in ML model to re-score the final top 10 results (provide options : Colbert, etc)

### Searching patch

**Filter & Cache Acceleration (1.1.2)**

- [ ] the collection map in AppState keeps every opened collection in memory forever with no eviction — a server that opens many collections will grow unbounded. `cache_max_bytes` config exists but nothing enforces it. needs an LRU eviction policy so idle collections can be closed and their memory (vector cache, metadata cache, mmap) released.
- [ ] IVF prefiltering with metadata posting lists to avoid full-scan overfetch on filtered queries
- [ ] bitmap/roaring filters for post-filter paths; filter selectivity stats
- [ ] collection preloading on startup: optionally pre-open a configured list of collections rather than waiting for the first request

**Query Features (1.1.3)**

- [ ] query result caching (LRU, TTL-based)
- [ ] query planning/optimization; query budget enforcement (timeouts, complexity limits)
- [ ] preset search modes: "fast / balanced / high-recall" mapped to tuned `ef`/`nprobe` params

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
- [ ] corrupted file detection on startup with safe rebuild prompt
- [ ] automatic index rebuild from WAL on detected corruption
- [ ] chaos tests: crash at WAL checkpoints, index rebuild idempotence, mmap-off fallback, partial-write recovery

---

### Schema, Filters & Clients

**Schema (1.1.6)**

- [ ] enforce dimension constraints end-to-end: set expected dimensions at collection creation time, re-validate at open time (fail-fast on mismatch), and validate before any mmap write so a failed insert never leaves a ghost entry on disk
- [ ] metadata schema validation (typed fields, required/optional)

**Search API Extensions (1.1.7)**

- [ ] grouped/diverse search (max results per category/namespace)
- [ ] scroll/cursor pagination for large result sets
- [ ] **Batch search endpoint:** add `POST /api/collections/:name/search/batch` accepting an array of query vectors and returning an array of result sets in a single round-trip — required for the batched RAG scheduler pipeline where multiple queries are issued per inference request.
- [ ] **Streaming search interface:** add a WebSocket or SSE endpoint for continuous query submission so an inference scheduler can push queries one at a time and receive results as they complete, enabling continuous batching iteration-level scheduling without pre-grouping queries (see [Zipy](https://github.com/ashworks1706/zipy) for the inference-side implementation).

**Clients & SDK (1.1.8)**

- [ ] Python client SDK + docs
- [ ] easy API reference for SDKs (Rust via MkDocs / Mintlify)
- [ ] collection aliases and rename
- [ ] allow moving collection between directories and setting DIR paths customization

---

### Documentation & Testing

**Documentation**

- [ ] Separate API docs to `docs.piramiddb.com` (Mintlify)
- [ ] document rust sdk with examples, link blogs from /blogs

**Dashboard**

- [ ] Fix dashboard UI
- [ ] Add more metrics to dashboard (memory usage, disk usage, query latency, etc.)
- [ ] Fix Docker image

---

### Operations & Reliability (1.1.9)

- [ ] enforce collection/request-level resource limits (max vectors, max bytes, QPS) with clear errors surfaced in metrics
- [ ] backpressure and rate limiting on write/search paths
- [ ] structured tracing with request IDs end-to-end; slow-query logging with configurable thresholds
- [ ] per-collection latency/recall histograms; cache and index freshness in `/api/metrics`
- [ ] set up benchmarks for latency, index strategies, memory usage across collection sizes
- [ ] refactor codebase for better modularity; expand unit and integration test coverage
- [ ] robust CI/CD pipeline covering all critical paths
- [ ] keep documentation and blogs in sync with code changes

---

### GPU Acceleration & Inference Fusion

*(Phase 2 - Systems-level integration between Storage and Inference. Blocked by quantization refactor & storage durability work. The inference engine counterpart to this work is [Zipy](https://github.com/ashworks1706/zipy). The items below are Piramid's side of the shared memory and Zero-Prefill contract between the two projects.)*

**Compute backend:**

- [ ] **Unified ComputeBackend Implementation:** Wire the `Gpu` backend to dispatch distance-calc batches to wgpu kernels.
- [ ] attempt GPU initialization on boot, fallback to CPU on failure (graceful degrade)
- [ ] add `/api/health` GPU status fields (temperature, VRAM used/free)

**Shared memory protocol (Piramid ↔ Zipy Phase 5):**

- [ ] **IPC Wire Format:** Define a `SharedBufferHandle` type — wraps a VRAM raw pointer, byte length, and a sync token — transmitted over a Unix domain socket or memfd. This is Piramid's side of the Shared Memory Protocol that lets the inference engine pull index data directly into GPU buffers without a CPU copy.
- [ ] **Shared VRAM Memory Pool:** Implement a memory-mapping protocol to allow zero-copy access to physical GPU buffers.
- [ ] **Pinned Memory Staging:** Implement wgpu staging belts to stream mmap data directly to VRAM without intermediate CPU copies.
- [ ] **Direct-to-Attention Handshake:** Implement the logic for Piramid to pass VRAM pointers of retrieved document caches directly to a PagedAttention block table.

**Zero-Prefill RAG (Piramid ↔ Zipy Phase 4 & 5):**

- [ ] **KV-Cache Storage Type:** Add a `KvCacheBlock` storage entry type alongside `Document` — stores pre-computed transformer KV tensors (FP16, safetensors layout) keyed by document ID. When a document's KV block is resident, Zipy can skip prefill entirely for that document.
- [ ] **Zero-Prefill Search Response Extension:** Extend the `Hit` type to optionally carry a KV-cache block pointer alongside the document ID and text — when the block is available and VRAM is resident, Zipy can inject it directly into the attention mechanism without re-reading the document.
- [ ] **KV-Cache Block Invalidation:** When a document is updated or deleted, invalidate and evict its associated KV-cache block from both the Piramid storage layer and any VRAM-resident copy, so stale KV data is never passed to the inference engine.
- [ ] **NVMe KV-Cache Offload Target:** Implement Piramid as the persistent NVMe store for Zipy's spilled KV blocks — when Zipy evicts blocks from VRAM to reclaim capacity, Piramid receives and durably stores them keyed by sequence ID for rapid warm reload.

**Safetensors / precision compatibility (Piramid ↔ Zipy Phase 1):**

- [ ] **Safetensors-compatible vector export:** Add `GET /api/collections/:name/vectors/export?format=safetensors` that serializes the vector store in `.safetensors` format so Zipy's staging belt can load it directly into GPU buffers without a format-conversion step.

**Blocked / Future (Systems Optimization):**

- [ ] **Warm Index Mirroring:** Automatically hydrate frequently accessed index clusters into GPU-managed VRAM on startup.
- [ ] **Batched Retrieval Dispatch:** Group multiple RAG search requests into a single GPU command buffer.
- [ ] **Unified Circuit Breaker:** Implement cross-process resource monitoring to balance VRAM allocation between Piramid indexes and LLM KV-caches.
- [ ] auto-switch to CPU if GPU reports OOM/timeout
- [ ] **Speculative Decode Context:** Annotate search results with token-level context windows in a format compatible with Zipy's speculative decoding pipeline, so retrieved documents can prime draft-model generation without a separate prefill pass.

---

### Later

**MCP Integration**

- [ ] MCP server implementation
- [ ] tools: `search_similar`, `get_document`, `list_collections`, `add_document`
- [ ] agent-friendly structured responses (JSON-LD)

**Security**

- [ ] JWT token authentication
- [ ] rate limiting and per-tenant quotas
- [ ] audit logging

**Metadata Filters**

- [ ] metadata indexing for fast pre-filtering
- [ ] range queries on numeric fields
- [ ] regex/pattern matching on string fields
- [ ] date range filters
- [ ] array membership checks
- [ ] vector count per metadata filter
- [ ] complex boolean filters (AND/OR/NOT combinations)

**Transactions & Consistency**

- [ ] atomic batch operations (all-or-nothing insert/delete sets)
- [ ] rollback on failure
- [ ] idempotency keys + request deduplication
- [ ] snapshot API (copy-on-write) + point-in-time recovery
- [ ] incremental backups and database migrations

**Platform**

- [ ] hybrid retrieval: dense ANN + sparse/BM25 scoring + rerank
- [ ] metadata-only search (no vector similarity)
- [ ] vector similarity between two stored vectors (no query vector)
- [ ] recommendation API (similar to these IDs, not those)
- [ ] API versioning in URLs or headers; backward compatibility strategy; deprecation warnings
- [ ] email/webhook alerts for errors, disk pressure, memory, slow queries, index corruption
