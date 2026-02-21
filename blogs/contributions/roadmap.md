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
- [ ] fix UI on overview page, make the block card clickable, make responsive on mobile 
- [ ] add section '#' icons, right sidebar embedded link formatting, leftsidebar page route highlighting 
- [ ] add limitations and future work section to the end of the blogs, add more images for the limitations and future work section, links for bigger terms, add summary at the end of the limitations and future work section

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

- [ ] in `insert_internal` (`storage/collection/operations.rs`), dimension validation runs after the mmap write — if validation fails, the mmap has a ghost entry with no index pointer that silently wastes space. move dimension validation before any writes.
- [ ] HNSW tombstone deletes never evict from `vector_cache` — deleted entries stay in memory until manual compaction. other index types (IVF, Flat) evict correctly on delete. fix: evict from `vector_cache` immediately after tombstone is set.
- [ ] `grow_mmap_if_needed` (`storage/persistence/mmap.rs`) calls `.unwrap()` on `mmap.as_ref()` unconditionally — panics on the first grow when mmap is disabled in config. add a guard for the mmap-disabled path.
- [ ] `update_metadata` and `update_vector` each write 3 WAL entries per call (Update + Delete + Insert) because they delegate to `delete()` and `insert()` instead of the internal variants. use `delete_internal` + `insert_internal` and emit only the single Update entry already logged at the top of each function.
- [ ] upsert double-quantizes on the update path: quantize → serialize → deserialize → dequantize → re-quantize. pass the original f32 through the upsert path directly.
- [ ] `storage/persistence/vector_index.rs` downcasts `Box<dyn VectorIndex>` via raw `*const dyn VectorIndex` pointer casts with `unsafe` — undefined behaviour on a wrong index type. replace with a `to_serializable()` method on the `VectorIndex` trait.
- [ ] `save_index` is called on every single insert, upsert, delete, and update — serializing the full `HashMap<UUID → EntryPointer>` to disk on every mutation is O(N) disk I/O per write. checkpoint-only saves, not per operation.
- [ ] offset calculation on every insert scans all entries via `.max()` over all `EntryPointer` values — O(N) per insert. replace with a single `next_offset: u64` field on the collection struct.

**IVF Index**

- [ ] IVF `insert` silently drops vectors during bootstrap: if `centroids.is_empty()` and `vectors.len() < num_clusters`, it returns early without inserting anywhere — vectors are permanently lost from the index with no error. buffer pre-cluster vectors and replay them into the index once clusters are built.
- [ ] IVF `insert` uses `Vec::contains` for duplicate ID checking (O(cluster_size) per insert) — use the existing `vector_to_cluster` HashMap instead.

**Server & API**

- [ ] every read endpoint calls `get_or_create_collection`, auto-creating a new empty collection on disk if the name does not exist. read-only handlers should use a `get_collection` that returns 404 on NotFound.
- [ ] `CachedEmbedder` (`embeddings/cache.rs`) holds `std::sync::Mutex` across `.await` points — blocks Tokio threads under cache contention. switch to `tokio::sync::Mutex` or drop the lock before any await.

---

### Write Path & Durability

**Async I/O**

- [ ] non-blocking writes via `tokio-fs`
- [ ] async write pipeline: batching/coalescing, buffered writes, background flush worker
- [ ] prefetching for sequential reads
- [ ] background job queue for long-running storage operations

**Crash Safety & Recovery**

- [ ] WAL and all persistence file formats need version fields and checksums; recovery paths must handle partial writes and format mismatches safely
- [ ] dimension/metric validation per collection at open time; fail-fast on mismatched inserts/searches; dry-run config validation
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

- [ ] move quantization from Storage to Index layer: `storage/collection/operations.rs` currently applies `QuantizedVector::from_f32_with_config()` at insert time, permanently discarding the original f32. Two problems: (1) no speed benefit — the index only stores UUIDs so graph traversal still fetches full f32s from `vector_cache` for every distance comparison; (2) accuracy loss — `search/engine.rs` dequantizes at score time but reconstructs from a lossy approximation, not the original. Fix: store original f32 in mmap, move quantization inside the index layer for graph traversal, re-rank final `ef` candidates with original f32 from storage, and remove the dequantization step in `search/engine.rs` entirely. This is the standard approach used by FAISS, Qdrant, and Weaviate.
- [ ] `src/quantization/` exists in scaffolded form — once integrated, PQ codes would live alongside the HNSW graph in `.vidx.db` and `search_layer` would use lookup-table ADC instead of full dot products. Reranking the final `ef` candidates with mmap'd f32 keeps recall high while search-phase compute drops ~8× and memory ~32×.

**Index Improvements**

- [ ] IVF uses random centroid initialisation (first K vectors) — k-means++ would sample proportionally to distance from the nearest existing centroid, producing better spread and fewer iterations to convergence
- [ ] adaptive index tuning: auto-adjust `ef`, `nprobe`, `filter_overfetch` based on per-collection latency/recall budgets and density
- [ ] background index maintenance: online HNSW compaction, tombstone cleanup, IVF cluster rebalancing without blocking reads
- [ ] circuit breaker for embedding API failures with fallback behaviour

**Filter & Cache Acceleration**

- [ ] metadata cache alongside vector cache with configurable eviction and rebuild/invalidation metrics
- [ ] IVF prefiltering with metadata posting lists to avoid full-scan overfetch on filtered queries
- [ ] bitmap/roaring filters for post-filter paths; filter selectivity stats
- [ ] background cache warmup and refresh scheduling

**Query Features**

- [ ] query result caching (LRU, TTL-based)
- [ ] query planning/optimization; query budget enforcement (timeouts, complexity limits)
- [ ] hybrid retrieval: dense ANN + sparse/BM25 scoring + rerank
- [ ] filter-aware search: expose range queries, AND/OR/NOT filter combinations
- [ ] preset search modes: "fast / balanced / high-recall" mapped to tuned `ef`/`nprobe` params

---

### Schema, Filters & Clients

**Schema**

- [ ] expected dimensions defined per collection at creation time
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

- [ ] add `zipy` crate to `Cargo.toml` as an optional feature
- [ ] refactor `ExecutionMode` to support `Zipy(Arc<ZipyEngine>)` as a compute backend
- [ ] attempt Zipy initialization on boot, fallback to CPU on failure
- [ ] hydrate existing on-disk vectors into GPU VRAM on startup
- [ ] dual-write architecture: inserts write to both disk (persistence) and Zipy (VRAM)
- [ ] route `POST /search` requests to Zipy when active
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
