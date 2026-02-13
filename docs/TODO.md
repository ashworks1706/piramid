### Foundation


[x] Vector storage (HashMap + bincode persistence)

[x] UUID-based IDs, error handling

[x] Store, retrieve, get all, delete, update

[ ] **Schema Versioning:** Add version headers to storage files to allow future data migration.

### Search & Similarity


[x] Similarity metrics (cosine, euclidean, dot product)

[x] Top-k search with scores

[x] Metadata support with filtering

[ ] **Filtered HNSW Traversal:** Integrate bitmap checks directly into the graph traversal loop.

### HTTP Server & Embeddings


[x] REST API (axum), Collections/Vectors CRUD

[x] Dashboard (Next.js - placeholder)

[x] Embedding providers (OpenAI, Ollama)

[x] Batch embedding endpoints

### Performance & Indexing


[x] Execution mode configuration (Auto, SIMD, Scalar)

[x] HNSW indexing (production-grade approximate k-NN)

[x] HNSW index persistence to disk

[x] HNSW ef_search parameter for search quality control

[x] IVF index persistence to disk

[x] IVF nprobe runtime override for search quality

[x] Flat index persistence to disk

[x] Benchmark suite

[x] SIMD acceleration (dot product, cosine, euclidean)

[x] Scalar fallback implementations for all metrics

[x] Memory-mapped files (mmap)

[x] Configurable mmap initial size

[x] Mmap enable/disable option

[x] Scalar quantization (int8) - 4x memory reduction

[x] Quantization configuration (None, Int8, Int4, Float16)

[x] Parallel search with rayon

[x] Parallelism configuration (SingleThreaded, Auto, Fixed threads)

[x] Parallel search toggle

[x] LRU cache for embeddings (50-90% cost savings)

[x] Cache configuration (size, TTL, enable/disable)

[ ] **Index Warmup:** Utility to fault-in mmap pages on startup to prevent initial latency spikes.

### Data Durability & Reliability

**Error Handling**


[x] Replace ALL `.unwrap()` with proper error handling

[x] Graceful degradation for failures

[x] Poison-free lock handling

[x] Clear, actionable error messages

[x] Domain-specific error types (Storage, Index, Server, Embedding)

[x] Error context helpers (.context(), .with_context())

[x] HTTP error response mapping

**Write-Ahead Log (WAL)**


[x] Append-only log for all mutations (insert/update/delete)

[x] Recovery from WAL on crash/restart

[x] Periodic checkpointing to reduce replay time

[x] WAL configuration (enable/disable, checkpoint frequency)

[x] WAL high durability and fast modes

[x] WAL disabled mode support

[ ] **WAL Truncation:** Logic to safely delete old WAL segments after a successful snapshot.

**Graceful Shutdown**


[x] Handle SIGTERM/SIGINT signals

[x] Flush all pending writes to disk

[x] Clean lock release

[x] Save HNSW index state

[x] Drain connections before shutdown

[x] Pre-shutdown warning to active clients

**Concurrent Safety**


[x] Lock-free or fine-grained locking for writes

[x] Deadlock detection/prevention

[x] Write conflict resolution strategy

### Production 

**Batch Operations**


[x] Batch insert API (10k inserts in <1s)

[x] Batch search (multiple queries in one request)

[x] Batch get vectors by IDs (via list_vectors with pagination)

[x] Bulk delete

**Collection Management**


[x] Delete collection (cascade remove all data)

[x] Collection metadata (created_at, updated_at, dimensions)

[x] List collections with stats

[x] Per-collection config override

[x] Unified CollectionConfig with all settings

[x] Storage usage per collection

**Vector Operations**


[x] Upsert (insert or update)

[x] Update vector only (keep metadata)

[x] Update metadata only (keep vector)

[x] Atomic update (vector + metadata together)

[x] Check vector existence by ID (via get)

[x] List vector IDs only (without full data) (via list_vectors)

[x] Implement SIMD/Parallel/Jit/Binary execution actual implementation with cpu detection 

[x] fix query in search folder modularity import

[ ] implement actual memory detection for collection 

**Codebase Organization**


[x] Modularize code into clear layers if not already (API, Service, Storage, Indexing)

[x] No redundant code

[x] no dead code 

[x] maximize for longetivity naming conventions NOT short-term convenience like search_with_smd or search_width_qualtiy -> optimize for UX experience and clarity instead of short-term dev speed 

[x] make sure folders are organized by domain (e.g. all search-related code in search/ folder) and not by technical layer (e.g. not api/search.rs, service/search.rs, etc.)

[x] make sure utility files are seperate and categorized 

**Configuration**


[ ] Config file support (YAML)

[ ] make sure config is universal and accessed from one point 

[ ] Config hot reload (limited subset)

[ ] Environment variable documentation

**Validation & Safety**


[x] Dimension consistency checks per collection

[x] Vector normalization option

[x] Vector format validation (NaN, infinity checks)

[x] Request size limits

[x] Input validation & sanitization

[x] Request timeout configuration (5s lock timeout)

[ ] Runtime config validation

**Embeddings Optimization**


[x] Native batch API support (OpenAI/Ollama - 2x-10x speedup)

[ ] remove prebuilt embedding functionality for now

[ ] CPU Local Embeddings support (e.g. sentence-transformers)

[ ] native batch api support for hugginface (make sure )

[ ] Request metrics (count, latency, tokens, cost)

[x] Type-safe config (enum-based instead of strings)

[x] Retry with exponential backoff

[ ] Provider timeout configuration

[x] Benchmark to verify 3-5x SIMD speedup target

**Observability**


[x] Basic `/metrics` endpoint

[x] Metrics: insert/search latency, index size, memory usage

[ ] Structured logging with tracing crate

[ ] Request ID for tracing

[ ] Enhanced health checks (storage status, index health, disk space)

[x] Ready endpoint (vs alive endpoint) - /api/health

[ ] Server version endpoint

[ ] Slow query logging

**Index Management**


[ ] Rebuild index function

[ ] Index compaction (remove deleted vectors)

[x] Index statistics endpoint

[x] HNSW memory usage calculation

[ ] Startup validation (check integrity on boot)

[ ] Startup health check (validate all collections load)

[ ] Duplicate detection (find similar vectors in collection)

[x] HNSW (current default)

[x] Flat/Brute Force (for small collections <10k vectors)

[x] IVF (Inverted File Index)

[ ] Product Quantization (PQ) (CPU Compression)

[ ] HNSW Tombstoning: Soft-delete nodes without breaking graph connectivity.

[ ] Implement HNSW Pre-filtering (Bitmap visitor) 


**Resource Management**

[ ] Max vectors per collection

[ ] Storage size limits per collection

[ ] Disk space monitoring

[ ] Memory pressure handling

[ ] Read-only mode when disk full

[ ] Automatic cleanup of orphaned files

[ ] Data compaction (reclaim space from deletes)

**HTTP & Networking**


[ ] HTTP/2 support

[ ] Compression (gzip/brotli) for responses

[ ] Keep-alive connection management

[x] Configurable max request body size

**Security Basics**


[ ] API key authentication

[ ] Security headers (CORS, CSP, HSTS)

[ ] TLS/SSL support

### Documentation & Testing

**Documentation**


[ ] `docs/API.md` - Interactive API docs (Swagger/OpenAPI)

[ ] `pip/README.md` - Python client usage

[ ] `npm/README.md` - JavaScript client usage

[ ] `docs/QUICKSTART.md` - 5-minute tutorial

[ ] `CHANGELOG.md` - Version tracking

[ ] License headers in source files

[ ] Third-party license audit

**Launch Prep**


[ ] dashboard full update and revamp

**CI/CD**


[x] GitHub Actions CI pipeline

[x] Docker image publishing (Dockerfile exists)

[ ] fix broken ci pipleine gh workflow for cargo 

[ ] fix broken ci pipleine  gh workflow for pip  

[ ] fix broken ci pipleine  gh workflow for npm 

[ ] fix broken ci pipleine gh workflow for docker image  

[ ] **Fuzz Testing:** Run `cargo fuzz` to test parser robustness.

[ ] **Property Testing:** Add `proptest` for state consistency verification.

### Post-Launch 

**ACID Transactions**

[ ] Atomic batch operations (all-or-nothing)

[ ] Rollback on failure

[ ] Isolation (at least serializable)

[ ] Idempotency keys

[ ] Request deduplication

**Async Storage I/O**

[ ] Non-blocking writes (tokio-fs)

[ ] Background flush worker

[ ] Write batching/coalescing

[ ] Prefetching for sequential reads

[ ] Write buffering optimization

[ ] Background job queue for long operations

**Query Optimization**

[ ] Query result caching

[ ] Query planning/optimization

[ ] Query timeout enforcement

[ ] Query complexity limits

**Backup & Restore**

[ ] Snapshot API (copy-on-write)

[ ] Point-in-time recovery (PITR)

[ ] Incremental backups

**Metadata Improvements**

[ ] Complex filters (AND/OR/NOT combinations)

[ ] Metadata indexing for fast filtering

[ ] Range queries on numeric metadata

[ ] Regex/pattern matching on string metadata

[ ] Date range filters

[ ] Array membership checks

**Schema Support**

[ ] Define expected dimensions per collection

[ ] Metadata schema validation

[ ] Schema versioning

**Advanced Search**

[ ] Range search (distance threshold instead of top-k)

[ ] Recommendation API (similar to these IDs, not those)

[ ] Grouped/diverse search (max results per category)

[ ] Scroll/pagination for large result sets

[ ] Metadata-only search (no vector similarity)

[ ] Vector similarity between two stored vectors

[ ] Vector count per metadata filter

**Data Import/Export**

[ ] Import from JSON/CSV/Parquet

[ ] Export to JSON/CSV/Parquet

[ ] Streaming import for large datasets

[ ] Import progress tracking

[ ] Format validation on import

**Advanced Security**

[ ] JWT token support

[ ] Multi-tenant isolation

[ ] Collection-level permissions

[ ] Rate limiting & quotas

[ ] Audit logging

**API Versioning**

[ ] API version in URLs or headers

[ ] Backward compatibility strategy

[ ] Deprecation warnings for old endpoints

[ ] API changelog tracking

**Monitoring & Alerting**


[ ] Email alerts for errors

[ ] Disk space alerts

[ ] Memory usage alerts

[ ] Index corruption alerts

[ ] Slow query alerts

### Future Considerations

**Additional Features**

[ ] Corrupted file detection + auto-repair

[ ] Automatic index rebuild on corruption

[ ] Fallback to brute-force search if HNSW fails

[ ] Circuit breaker for embedding API failures

[ ] Soft delete with cleanup

[ ] Collection aliases

[ ] Hot reload configuration

[ ] Move collection between directories

[ ] Development mode with auto-reload

[ ] Verbose debug logging mode


**MCP Integration**

[ ] MCP server implementation

[ ] Tools: search_similar, get_document, list_collections, add_document

[ ] Agent-friendly responses (structured JSON-LD)

### [Zipy](https://github.com/ashworks1706/zipy) development begins

**Zipy Integration (GPU Acceleration)**

[ ] Dependency Integration: Add zipy crate to Cargo.toml as an optional feature.

[ ] Compute Backend Enum: Refactor ExecutionMode to support Zipy(Arc<ZipyEngine>) variant.

[ ] Startup Handshake: Implement logic to attempt Zipy initialization on boot and fallback to CPU if failed.

[ ] VRAM Hydration: Utility to load existing on-disk vectors into GPU VRAM on startup.

[ ] Dual-Write Architecture: Ensure `insert_vector` writes to both Disk (Persistence) and Zipy (VRAM).

[ ] Search Router: Implement logic to route `POST /search` requests to Zipy when active.

[ ] Fallback Circuit Breaker: Auto-switch to CPU search if Zipy returns OOM or timeout errors.

[ ] Health Check Extension: Add GPU status (temperature, memory usage) to `/api/health`.



