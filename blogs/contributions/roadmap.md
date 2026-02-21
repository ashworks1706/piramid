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

### Post-Launch

**Storage Correctness**

- [ ] in `insert_internal` in `storage/collection/operations.rs`, bytes are written to the mmap file and the index entry is inserted before dimension validation runs. if validation fails the mmap has a ghost entry with no index pointer, which silently accumulates over time and wastes space. dimension validation should happen before any writes.
- [ ] HNSW deletes use tombstones for graph connectivity but never remove the deleted vector from `vector_cache`. the cache entry stays in memory until manual compaction is triggered, meaning collections with frequent deletes will grow unbounded in memory. other index types (IVF, Flat) correctly remove from cache on delete. fix: evict from `vector_cache` after tombstone is set, or schedule a background eviction pass.
- [ ] `storage/persistence/vector_index.rs` downcasts `Box<dyn VectorIndex>` to concrete types using raw `*const dyn VectorIndex` pointer casts with `unsafe`. there is no type-level guarantee the cast is valid and a wrong index type at runtime would be undefined behaviour. should be replaced with a `to_serializable()` method on the `VectorIndex` trait that each implementation returns safely.
- [ ] `grow_mmap_if_needed` in `storage/persistence/mmap.rs` calls `.unwrap()` on `mmap.as_ref()` unconditionally at the start. if mmap is disabled in collection config, this panics on the first insert that requires a grow. needs a guard for the mmap-disabled path.
- [ ] `update_metadata` and `update_vector` in `storage/collection/operations.rs` each log 3 WAL entries per operation: an Update entry they log directly, then a Delete entry from calling `delete()`, then an Insert entry from calling `insert()`. a single logical update should produce one WAL entry. fix: use `delete_internal` and `insert_internal` directly and log only the single Update entry already at the top of each function.
- [ ] upsert on the update path double-quantizes vectors: it serializes the already-quantized entry to bytes, then deserializes it back into a Document (which still holds a `QuantizedVector`), then `insert_internal` calls `get_vector()` to dequantize and immediately re-quantizes again. the vector goes through quantize → serialize → deserialize → dequantize → re-quantize. fix: pass the original raw f32 through the upsert path directly.
- [ ] IVF `insert` silently drops vectors during the bootstrap phase: if `centroids.is_empty()` and `vectors.len() < num_clusters`, it returns early without inserting the new vector into any structure. vectors inserted before the threshold are permanently lost from the IVF index with no error or warning. fix: buffer pre-cluster vectors and replay them into the index once clusters are built.
- [ ] IVF `insert` uses `Vec::contains` to check for duplicate IDs in the inverted list, which is O(cluster_size) per insert. should use the existing `vector_to_cluster` HashMap for the duplicate check instead.

**Server & API Correctness**

- [ ] every read endpoint (`GET /collections/:name`, `GET /collections/:name/vectors/:id`, `POST /collections/:name/search`, etc.) calls `get_or_create_collection` which creates a new empty collection on disk if the name does not exist. a GET or search on a non-existent collection should return 404, not silently create it. read-only handlers should use a `get_collection` helper that returns NotFound instead.
- [ ] `CachedEmbedder` in `embeddings/cache.rs` uses `std::sync::Mutex` around the LRU cache inside an async context. holding a sync mutex across `.await` points blocks the Tokio thread pool under cache contention. should use `tokio::sync::Mutex` or restructure to release the lock before any await.

**Storage Write Performance**

- [ ] `save_index` is called on every single insert, upsert, delete, and update in `storage/collection/operations.rs`. this serializes the entire `HashMap<UUID → EntryPointer>` to disk on every mutation which is O(N) disk I/O per write. it should only trigger at checkpoint time, not per operation. will noticeably degrade write throughput as collections grow.
- [ ] offset calculation on every insert scans all entries in the index to find the write position via `.max()` over all `EntryPointer` values. this is O(N) per insert and should be replaced with a single `next_offset: u64` field tracked on the collection struct and incremented after each write.

**Schema Support**

- [ ] Define expected dimensions per collection
- [ ] Metadata schema validation

**Metadata Improvements**

- [ ] Complex filters (AND/OR/NOT combinations)
- [ ] Metadata indexing for fast filtering
- [ ] Range queries on numeric metadata
- [ ] Regex/pattern matching on string metadata
- [ ] Date range filters
- [ ] Array membership checks

**Query Optimization**

- [ ] Query result caching
- [ ] Query planning/optimization
- [ ] Query budget enforcement (timeouts, complexity limits)

**Async Storage I/O**

- [ ] Non-blocking writes (`tokio-fs`)
- [ ] Async write pipeline (batching/coalescing, buffering, background flush worker)
- [ ] Prefetching for sequential reads
- [ ] Background job queue for long operations

**ACID Transactions**

- [ ] Atomic batch operations (all-or-nothing)
- [ ] Rollback on failure
- [ ] Isolation (at least serializable)
- [ ] Idempotency keys
- [ ] Request deduplication

**Backup & Restore**

- [ ] Snapshot API (copy-on-write)
- [ ] Point-in-time recovery (PITR)
- [ ] Incremental backups
- [ ] Database migrations

**Benchmarks**

- [ ] Set up benchmarks for latency, index strategies, memory usage, etc.

**Python Support**

- [ ] Python client SDK
- [ ] Add docs
- [ ] Easy API docs for SDKs (Rust via MkDocs)

**Regular codebase refreshment**
- [ ] refactor codebase for better modularity and maintainability
- [ ] add more unit tests and integration tests
- [ ] make sure ci cd pipeline is robust and covers all critical paths
- [ ] update documentation to reflect any code changes and new features
- [ ] update blogs to reflect any code changes and new features

---

### Future Considerations

**Distributed Systems**

- [ ] Sharding strategies (range, hash, etc.)
- [ ] Replication strategies (master-slave, multi-master, etc.)
- [ ] Consistency models (strong, eventual, etc.)
- [ ] Distributed transactions
- [ ] Cluster management (node discovery, leader election, etc.)

**Index/Search Improvements**

- [ ] Adaptive index tuning: auto-adjust ef/nprobe/filter_overfetch based on latency/error budgets and collection density
- [ ] Filter-aware search across indexes: add IVF prefiltering with metadata posting lists to avoid full-scan overfetch
- [ ] Hybrid retrieval: combine dense ANN with sparse/BM25 scoring and rerank
- [ ] Background index maintenance: online HNSW/IVF compaction and tombstone cleanup without blocking reads
- [ ] Range and preset search modes: expose range queries and "fast/balanced/high" presets mapped to tuned params
- [ ] Search observability: per-collection recall/latency histograms and sampled miss diagnostics in `/api/metrics`
- [ ] move quantization from Storage to Index layer: currently `storage/collection/operations.rs` applies `QuantizedVector::from_f32_with_config()` at insert time, permanently discarding the original f32. This causes two concrete problems: (1) no speed benefit — the index only stores UUIDs so HNSW/IVF graph traversal still fetches full f32s from `vector_cache` for every distance comparison, meaning the quantization compression never accelerates search at all; (2) accuracy loss with no upside — `search/engine.rs` does dequantize at score time, but it reconstructs from a lossy approximation, not the original, so final scores are degraded. The fix has three parts: store original f32 in mmap as the source of truth, move quantization inside the index layer so graph traversal uses compressed vectors for fast candidate selection, then re-rank the final `ef` candidates using the original f32s from storage for accurate scoring. As a result of this fix, the dequantization step currently in `search/engine.rs` should be removed entirely — once storage returns raw f32 there is nothing to decode, and the search engine scores directly against the original vector. This is the standard approach used by FAISS, Qdrant, and Weaviate.
- [ ] src/quantization/ module exists in scaffolded form. Once integrated, the PQ codes would be stored alongside the HNSW graph in the .vidx.db file, and search_layer's distance function would use lookup-table ADC instead of full dot products. The reranking pass over the final ef candidates would still use mmap'd float32 vectors, keeping recall high while the search-phase compute drops by 8× and memory drops by 32×.
- [ ] Piramid uses random initialisation (just takes the first KK vectors). k-means++ initialises centroids by sampling proportionally to ∥x−nearest existing centroid∥2∥x−nearest existing centroid∥2, which produces better initial spread and usually converges in fewer iterations. It's a potential improvement to the build phase for distributions where random initialisation produces early clustering near dense regions

**Regular codebase refreshment**
- [ ] refactor codebase for better modularity and maintainability
- [ ] add more unit tests and integration tests
- [ ] make sure ci cd pipeline is robust and covers all critical paths
- [ ] update documentation to reflect any code changes and new features 
- [ ] update blogs to reflect any code changes and new features 


**Reliability & Safety**

- [ ] WAL/schema versioning across all files with checksums; recovery paths that handle partial corruption safely
- [ ] Dimension/metric validation per collection; fail-fast on mismatched inserts/searches; add dry-run config validation
- [ ] Snapshot + PITR plan; import/export for portability; corruption detection and safe rebuild prompts
- [ ] Chaos tests for WAL replay (crash at checkpoints), index rebuild idempotence, mmap-off fallback

**Caching & Filters**

- [ ] Metadata cache alongside vector cache with rebuild/invalidation metrics; configurable cache caps/eviction
- [ ] Filter acceleration: IVF prefiltering (posting lists), bitmap/roaring filters for post-filter paths, filter selectivity stats
- [ ] Background cache warmup tasks and refresh scheduling

**Limits & Guardrails**

- [ ] Enforce collection/request-level resource limits (vectors, bytes, QPS) with clear errors and metrics
- [ ] Backpressure and rate limits surfaced in health/metrics endpoints

**Regular codebase refreshment**
- [ ] refactor codebase for better modularity and maintainability
- [ ] add more unit tests and integration tests
- [ ] make sure ci cd pipeline is robust and covers all critical paths
- [ ] update documentation to reflect any code changes and new features 
- [ ] update blogs to reflect any code changes and new features 

**Observability**

- [ ] Structured tracing with request IDs end-to-end; slow-query logging with thresholds per collection
- [ ] Per-collection latency/recall histograms; cache/index freshness surfaced in `/api/metrics`

**Advanced Search**

- [ ] Recommendation API (similar to these IDs, not those)
- [ ] Grouped/diverse search (max results per category)
- [ ] Scroll/pagination for large result sets
- [ ] Metadata-only search (no vector similarity)
- [ ] Vector similarity between two stored vectors
- [ ] Vector count per metadata filter
- [ ] SQL integration

**Additional Features**

- [ ] Corrupted file detection + auto-repair
- [ ] Automatic index rebuild on corruption
- [ ] Circuit breaker for embedding API failures
- [ ] Collection aliases
- [ ] Move collection between directories

**MCP Integration**

- [ ] MCP server implementation
- [ ] Tools: `search_similar`, `get_document`, `list_collections`, `add_document`
- [ ] Agent-friendly responses (structured JSON-LD)

### [Zipy](https://github.com/ashworks1706/zipy) Development Begins

**Zipy Integration (GPU Acceleration)**

- [ ] Dependency integration: add `zipy` crate to `Cargo.toml` as an optional feature
- [ ] Compute backend enum: refactor `ExecutionMode` to support `Zipy(Arc<ZipyEngine>)`
- [ ] Startup handshake: attempt Zipy initialization on boot, fallback to CPU on failure
- [ ] VRAM hydration: load existing on-disk vectors into GPU VRAM on startup
- [ ] Dual-write architecture: ensure inserts write to both disk (persistence) and Zipy (VRAM)
- [ ] Search router: route `POST /search` requests to Zipy when active
- [ ] Fallback circuit breaker: auto-switch to CPU search if Zipy returns OOM/timeout
- [ ] Health check extension: add GPU status (temperature, memory usage) to `/api/health`

**Advanced Security**

- [ ] JWT token support
- [ ] Multi-tenant isolation
- [ ] Collection-level permissions
- [ ] Rate limiting & quotas
- [ ] Audit logging

**API Versioning**

- [ ] API version in URLs or headers
- [ ] Backward compatibility strategy
- [ ] Deprecation warnings for old endpoints
- [ ] API changelog tracking

**Monitoring & Alerting**

- [ ] Email alerts for errors
- [ ] Disk space alerts
- [ ] Memory usage alerts
- [ ] Index corruption alerts
- [ ] Slow query alerts

**Data Import/Export**

- [ ] Import from JSON/CSV/Parquet
- [ ] Export to JSON/CSV/Parquet
- [ ] Streaming import for large datasets
- [ ] Import progress tracking
- [ ] Format validation on import
