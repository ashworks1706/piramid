# Roadmap

This is the working roadmap for contributors. If you want to help, start here and pick one scoped task. If your idea is not listed but adjacent, open an issue first and propose it before implementation.

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

---

### Index Quality patch

**Quantization Refactor (1.1.0)**
- [ ] quantization currently happens at insert time in the storage layer, which permanently throws away the original vectors. this causes two problems: search gets no speed benefit because the index still fetches full float vectors during traversal anyway, and final scores are calculated from a lossy reconstruction instead of the originals. the fix is to store raw vectors in storage as the source of truth, move quantization inside the index so it accelerates graph traversal, and re-rank the final small candidate set using the original floats. as part of this: remove the upsert double-quantize path (storage no longer quantizes at all), remove the HNSW vector cache eviction bug (vector cache gets deleted entirely), and remove the metadata cache (re-ranking reads metadata from mmap for free alongside the vector).
- [ ] the quantization module (`src/quantization/`) already has PQ (Product Quantization) implemented — it splits vectors into sub-blocks and compresses each independently, giving much better compression than scalar quantization. but it's not wired into search yet. once connected, index traversal would use fast lookup-table distance math instead of full dot products, dropping search compute ~8× and memory per vector ~32×, while the re-ranking step on final candidates keeps recall high.
- [ ] **FP16/BF16 vector precision:** promote `QuantizationLevel::Float16` from a stub to a real implementation — store and serve vectors in native half-precision without upcasting to FP32, eliminating a costly precision-conversion step on the hot search path.

**Index Improvements (1.1.1)**

- [ ] IVF uses random centroid initialisation (first K vectors) — k-means++ would sample proportionally to distance from the nearest existing centroid, producing better spread and fewer iterations to convergence
- [ ] adaptive index tuning: auto-adjust `ef`, `nprobe`, `filter_overfetch` based on per-collection latency/recall budgets and density
- [ ] background index maintenance: online HNSW compaction, tombstone cleanup, IVF cluster rebalancing without blocking reads
- [ ] circuit breaker for embedding API failures with fallback behaviour

### Searching patch

**Server & API (1.0.3)**

- [ ] read endpoints (GET collection, GET vector, search) silently create a new empty collection on disk if the name doesn't exist, instead of returning a 404. only write endpoints should be allowed to create collections.
- [ ] the embedding cache uses a blocking mutex inside async request handlers, which can stall the async runtime under load. should use an async-aware lock or be restructured to avoid holding it across await points.


**Filter & Cache Acceleration (1.1.2)**

- [ ] the collection map in AppState keeps every opened collection in memory forever with no eviction — a server that opens many collections will grow unbounded. `cache_max_bytes` config exists but nothing enforces it. needs an LRU eviction policy so idle collections can be closed and their memory (vector cache, metadata cache, mmap) released.
- [ ] IVF prefiltering with metadata posting lists to avoid full-scan overfetch on filtered queries
- [ ] bitmap/roaring filters for post-filter paths; filter selectivity stats
- [ ] collection preloading on startup: optionally pre-open a configured list of collections rather than waiting for the first request

**Query Features (1.1.3)**

- [ ] query result caching (LRU, TTL-based)
- [ ] query planning/optimization; query budget enforcement (timeouts, complexity limits)
- [ ] preset search modes: "fast / balanced / high-recall" mapped to tuned `ef`/`nprobe` params

**Metadata Filters**

- [ ] metadata indexing for fast pre-filtering
- [ ] range queries on numeric fields
- [ ] regex/pattern matching on string fields
- [ ] date range filters
- [ ] array membership checks
- [ ] vector count per metadata filter
- [ ] complex boolean filters (AND/OR/NOT combinations)

**Search API Extensions (1.1.7)**

- [ ] **Batch search endpoint:** add `POST /api/collections/:name/search/batch` accepting an array of query vectors and returning an array of result sets in a single round-trip — useful for high-throughput agentic pipelines where multiple queries are issued per request.
- [ ] **Streaming search interface:** add a WebSocket or SSE endpoint for continuous query submission so a client can push queries one at a time and receive results as they complete, enabling continuous batching without pre-grouping queries.
- [ ] hybrid retrieval: dense ANN + sparse/BM25 scoring + rerank
- [ ] metadata-only search (no vector similarity)
- [ ] recommendation API (similar to these IDs, not those)


### GPU Acceleration patch

**Introduce GPUBackend trait:**

- [ ] index traversal must dispatch distance computation through a pluggable backend abstraction, enabling future parallelism improvements.
- [ ] Add a query optimizer that switches to Flat Search + Bitmaps when metadata filters are highly selective (>90% reduction)
- [ ] Implement Logical Namespacing to allow multiple users to share one Collection/Index without cross-talk or performance degradation.
- [ ] Replace custom index serialization with rkyv for zero-copy, instant-load index access from mmap.
- [ ] Add LSH (Locality Sensitive Hashing) as a high-speed, low-RAM alternative to HNSW.
- [ ] add Annoy
- [ ] Add Binary Quantization (BQ): Turning vectors into 1s and 0s for 32x speedups
- [ ] Implement Cross-Encoders: A tiny built-in ML model to re-score the final top 10 results (provide options : Colbert, etc)

**GPU backend:**

- [ ] **WGPU Implementation:** Wire the `Cpu` and future parallel backends to dispatch distance-calc batches.
- [ ] attempt accelerated initialization on boot, fallback to baseline on failure (graceful degrade)

**Safetensors / precision compatibility:**

- [ ] **Safetensors-compatible vector export:** Add `GET /api/collections/:name/vectors/export?format=safetensors` that serializes the vector store in `.safetensors` format for interoperability with other tools.

**Blocked / Future (Systems Optimization):**

- [ ] **Warm Index Mirroring:** Automatically hydrate frequently accessed index clusters into memory on startup.
- [ ] **Batched Retrieval Dispatch:** Group multiple search requests into a single compute batch.

### Transformer Inference Patch 

**Introduce Transformer:**
- [ ] add support for running small transformer models 
- [ ] add kvcaching, batching and async support to the transformer inference module 
- [ ] add paged attention support for long contexts 
- [ ] add support for quantization 
- [ ] add streaming api 

### Transformer x Database Attention Fusion Patch

- [ ] modify transformer blocks :  configurable key/value projection heads for database vectors
- [ ] learnable gating mechanism to balance attention between internal context and external memory
- [ ] efficient retrieval of relevant database vectors per query (e.g. via ANN search) to keep attention tractable
- [ ] cross attention with database vectors as keyvalues with query from transformer 

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

### Operations & Reliability (1.1.9)

- [ ] set up benchmarks for latency, index strategies, memory usage across collection sizes
- [ ] refactor codebase for better modularity; expand unit and integration test coverage
- [ ] robust CI/CD pipeline covering all critical paths
- [ ] keep documentation and blogs in sync with code changes

---

### Later

### Schema, Filters & Clients

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

**Transactions & Consistency**

- [ ] atomic batch operations (all-or-nothing insert/delete sets)
- [ ] rollback on failure
- [ ] idempotency keys + request deduplication
- [ ] snapshot API (copy-on-write) + point-in-time recovery
- [ ] incremental backups and database migrations

**Platform**

- [ ] API versioning in URLs or headers; backward compatibility strategy; deprecation warnings
- [ ] email/webhook alerts for errors, disk pressure, memory, slow queries, index corruption
