# Roadmap

This is the working roadmap for contributors. If you want to help, start here and pick one scoped task. If your idea is not listed but adjacent, open an issue first and propose it before implementation.

### Documentation & Testing

**Blogs**

- [ ] add vector database components diagram on evolution page
- [ ] add more images for the database page methadologies,  links for bigger terms, add summary at the end  
- [ ] add more images for the embeddings page, link my workshop content there, links for bigger terms, add summary at the end
- [ ] add more images for storage page, links for bigger terms, add summary at the end
- [ ] add more images for indexing page and links for bigger terms, add summary at the end 
- [ ] add more images for query page and links for bigger terms, add summary at the end 

**Documentation**

- [ ] Separate API docs to `docs.piramiddb.com` (Mintlify)
- [ ] document rust sdk with examples, link blogs from /blogs

**Dashboard**

- [ ] Fix dashboard UI
- [ ] Add more metrics to dashboard (memory usage, disk usage, query latency, etc.)
- [ ] Fix Docker image

---

### Bug Fixes

**Storage**

- [ ] if mmap is disabled in the collection config, the storage layer panics on the first write that needs to grow the file. needs a proper guard instead of an unconditional unwrap.
- [ ] updating a vector or its metadata writes three separate WAL log entries per operation (update + delete + insert) instead of one. this inflates the WAL unnecessarily and makes recovery replay more expensive than it needs to be.
- [ ] index serialization uses unsafe pointer casts to convert the index trait object to a concrete type — if the wrong type is ever stored this is undefined behaviour. should use a safe serialization method on the index trait instead.
- [ ] the index pointer file (`.index.db`) is fully rewritten on every single insert, update, or delete, while the ANN graph file (`.vecindex.db`) is only written at checkpoints. this inconsistency means the two files can describe different states between checkpoints — if the WAL is disabled or corrupted during that window, the ANN graph silently misses entries with no error. both files should flush together at checkpoint time only, with the WAL as the only per-mutation write.
- [ ] finding the next write offset scans every existing entry on every insert to find the maximum. this should just be a counter that increments as vectors are added.

**IVF Index**

- [ ] during the IVF bootstrap phase, if not enough vectors have been inserted yet to form clusters, new vectors are silently dropped without any error or warning — they're just lost. vectors should be held in a buffer and replayed into the index once clustering is ready.
- [ ] IVF checks for duplicate vector IDs by scanning the cluster list on every insert, which gets slow as clusters grow. it can use the existing ID-to-cluster map instead for an instant lookup.

**Server & API**

- [ ] read endpoints (GET collection, GET vector, search) silently create a new empty collection on disk if the name doesn't exist, instead of returning a 404. only write endpoints should be allowed to create collections.
- [ ] the embedding cache uses a blocking mutex inside async request handlers, which can stall the async runtime under load. should use an async-aware lock or be restructured to avoid holding it across await points.

---

### Write Path & Durability

**Async I/O**

- [ ] non-blocking writes via `tokio-fs`
- [ ] async write pipeline: batching/coalescing, buffered writes, background flush worker
- [ ] prefetching for sequential reads
- [ ] background job queue for long-running storage operations

**Crash Safety & Recovery**

- [ ] WAL and all persistence file formats need version fields and checksums; recovery paths must handle partial writes and format mismatches safely
- [ ] dry-run config validation on startup; fail-fast on mismatched config between what's stored and what's loaded
- [ ] corrupted file detection on startup with safe rebuild prompt
- [ ] automatic index rebuild from WAL on detected corruption
- [ ] chaos tests: crash at WAL checkpoints, index rebuild idempotence, mmap-off fallback, partial-write recovery

**Transactions & Consistency**

- [ ] atomic batch operations (all-or-nothing insert/delete sets)
- [ ] rollback on failure
- [ ] idempotency keys + request deduplication
- [ ] snapshot API (copy-on-write) + point-in-time recovery
- [ ] incremental backups and database migrations

---

### Index & Search Quality

**Quantization Refactor**

- [ ] quantization currently happens at insert time in the storage layer, which permanently throws away the original vectors. this causes two problems: search gets no speed benefit because the index still fetches full float vectors during traversal anyway, and final scores are calculated from a lossy reconstruction instead of the originals. the fix is to store raw vectors in storage as the source of truth, move quantization inside the index so it accelerates graph traversal, and re-rank the final small candidate set using the original floats. as part of this: remove the upsert double-quantize path (storage no longer quantizes at all), remove the HNSW vector cache eviction bug (vector cache gets deleted entirely), and remove the metadata cache (re-ranking reads metadata from mmap for free alongside the vector).
- [ ] the quantization module (`src/quantization/`) already has PQ (Product Quantization) implemented — it splits vectors into sub-blocks and compresses each independently, giving much better compression than scalar quantization. but it's not wired into search yet. once connected, index traversal would use fast lookup-table distance math instead of full dot products, dropping search compute ~8× and memory per vector ~32×, while the re-ranking step on final candidates keeps recall high.

**Index Improvements**

- [ ] IVF uses random centroid initialisation (first K vectors) — k-means++ would sample proportionally to distance from the nearest existing centroid, producing better spread and fewer iterations to convergence
- [ ] adaptive index tuning: auto-adjust `ef`, `nprobe`, `filter_overfetch` based on per-collection latency/recall budgets and density
- [ ] background index maintenance: online HNSW compaction, tombstone cleanup, IVF cluster rebalancing without blocking reads
- [ ] circuit breaker for embedding API failures with fallback behaviour

**Filter & Cache Acceleration**

- [ ] the collection map in AppState keeps every opened collection in memory forever with no eviction — a server that opens many collections will grow unbounded. `cache_max_bytes` config exists but nothing enforces it. needs an LRU eviction policy so idle collections can be closed and their memory (vector cache, metadata cache, mmap) released.
- [ ] IVF prefiltering with metadata posting lists to avoid full-scan overfetch on filtered queries
- [ ] bitmap/roaring filters for post-filter paths; filter selectivity stats
- [ ] collection preloading on startup: optionally pre-open a configured list of collections rather than waiting for the first request

**Query Features**

- [ ] query result caching (LRU, TTL-based)
- [ ] query planning/optimization; query budget enforcement (timeouts, complexity limits)
- [ ] hybrid retrieval: dense ANN + sparse/BM25 scoring + rerank
- [ ] preset search modes: "fast / balanced / high-recall" mapped to tuned `ef`/`nprobe` params

---

### Schema, Filters & Clients

**Schema**

- [ ] enforce dimension constraints end-to-end: set expected dimensions at collection creation time, re-validate at open time (fail-fast on mismatch), and validate before any mmap write so a failed insert never leaves a ghost entry on disk
- [ ] metadata schema validation (typed fields, required/optional)

**Metadata Filters**

- [ ] complex boolean filters (AND/OR/NOT combinations)
- [ ] metadata indexing for fast pre-filtering
- [ ] range queries on numeric fields
- [ ] regex/pattern matching on string fields
- [ ] date range filters
- [ ] array membership checks
- [ ] metadata-only search (no vector similarity)
- [ ] vector count per metadata filter

**Search API Extensions**

- [ ] recommendation API (similar to these IDs, not those)
- [ ] grouped/diverse search (max results per category/namespace)
- [ ] scroll/cursor pagination for large result sets
- [ ] vector similarity between two stored vectors (no query vector)
- [ ] SQL integration

**Clients & SDK**

- [ ] Python client SDK + docs
- [ ] easy API reference for SDKs (Rust via MkDocs / Mintlify)
- [ ] collection aliases and rename
- [ ] move collection between directories

**MCP Integration**

- [ ] MCP server implementation
- [ ] tools: `search_similar`, `get_document`, `list_collections`, `add_document`
- [ ] agent-friendly structured responses (JSON-LD)

---

### Operations & Reliability

- [ ] enforce collection/request-level resource limits (max vectors, max bytes, QPS) with clear errors surfaced in metrics
- [ ] backpressure and rate limiting on write/search paths
- [ ] structured tracing with request IDs end-to-end; slow-query logging with configurable thresholds
- [ ] per-collection latency/recall histograms; cache and index freshness in `/api/metrics`
- [ ] set up benchmarks for latency, index strategies, memory usage across collection sizes
- [ ] refactor codebase for better modularity; expand unit and integration test coverage
- [ ] robust CI/CD pipeline covering all critical paths
- [ ] keep documentation and blogs in sync with code changes

---

### [Zipy](https://github.com/ashworks1706/zipy) GPU Acceleration

*depends on the quan- [ ] fix UI on overview page, make the block card clickable on index md page, make responsive on mobile 
- [ ] add section '#' icons, right sidebar embedded link formatting, leftsidebar page route highlighting 
tization refactor being complete — VRAM hydration and dual-write require raw f32 vectors in storage as the source of truth*

- [ ] add `zipy` crate to `Cargo.toml` as an optional feature
- [ ] wire the existing `ExecutionMode::Gpu` variant to dispatch through `ZipyEngine` — do not add a new variant, `Gpu` is already the intended hook and adding another leaves it as dead code
- [ ] attempt Zipy initialization on boot, fallback to CPU on failure
- [ ] hydrate existing on-disk vectors from mmap into GPU VRAM on startup (raw f32 — requires quantization refactor done first)
- [ ] dual-write architecture: inserts write to both mmap (persistence) and Zipy VRAM so VRAM stays warm without re-hydrating on every restart (requires quantization refactor done first)
- [ ] wire distance computation inside each index (HNSW hop distances, IVF centroid lookups, Flat candidate scores) to dispatch through Zipy when `ExecutionMode::Gpu` is active — the HTTP search handler does not change, only the internal compute path does
- [ ] flat search: replace the sequential per-vector distance loop with a single batched GPU matrix-multiply dispatch via Zipy (turns O(N) CPU calls into one GPU kernel)
- [ ] IVF: offload k-means cluster training and nearest-centroid lookups to Zipy GPU kernels (Zipy's TODO explicitly targets both)
- [ ] auto-switch to CPU search if Zipy returns OOM or timeout
- [ ] extend `/api/health` with GPU status (temperature, VRAM used/free)

---

### Distributed & Enterprise

**Scale**

- [ ] sharding strategies (range, hash)
- [ ] replication (primary-replica, multi-primary)
- [ ] consistency models (strong, eventual, bounded staleness)
- [ ] distributed transactions and cluster management (leader election, node discovery)

**Security**

- [ ] JWT token authentication
- [ ] multi-tenant isolation with collection-level permissions
- [ ] rate limiting and per-tenant quotas
- [ ] audit logging

**Platform**

- [ ] API versioning in URLs or headers; backward compatibility strategy; deprecation warnings
- [ ] email/webhook alerts for errors, disk pressure, memory, slow queries, index corruption
- [ ] import from JSON/CSV/Parquet with streaming and progress tracking
- [ ] export to JSON/CSV/Parquet; format validation on import
