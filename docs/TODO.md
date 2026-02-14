### Foundation

[x] **Schema Versioning:** Add version headers to storage files to allow future data migration.

### Search & Similarity

[ ] **Filtered HNSW Traversal:** Integrate bitmap checks directly into the graph traversal loop.

### Performance & Indexing

[ ] **Index Warmup:** Utility to fault-in mmap pages on startup to prevent initial latency spikes.

### Data Durability & Reliability

**Write-Ahead Log (WAL)**

[ ] **WAL Truncation:** Logic to safely delete old WAL segments after a successful snapshot.

### Production 

**Vector Operations**

[ ] implement actual memory detection for collection 

**Configuration**

[x] Config file support (YAML/JSON)

[ ] make sure config is universal and accessed from one point 

[ ] Config hot reload (limited subset)

[ ] Environment variable documentation

**Validation & Safety**

[ ] Runtime config validation

**Embeddings Optimization**

[ ] remove prebuilt embedding functionality for now

[ ] CPU Local Embeddings support (e.g. sentence-transformers)

[ ] Request metrics (count, latency, tokens, cost)

[ ] Provider timeout configuration

**Observability**

[x] Structured logging with tracing crate

[x] Request ID for tracing

[ ] Enhanced health checks (storage status, index health, disk space)

[ ] Server version endpoint

[x] Slow query logging

**Index Management**


[ ] Rebuild index function

[ ] Index compaction (remove deleted vectors)

[ ] Startup validation (check integrity on boot)

[ ] Startup health check (validate all collections load)

[ ] Duplicate detection (find similar vectors in collection)

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

**Security Basics**


[ ] API key authentication

[ ] Security headers (CORS, CSP, HSTS)

[ ] TLS/SSL support

### Documentation & Testing

**Documentation**

[ ] `docs/API.md` - Interactive API docs (Swagger/OpenAPI)

[ ] `pip/README.md` - Python client usage

[ ] `docs/QUICKSTART.md` - 5-minute tutorial

[ ] `CHANGELOG.md` - Version tracking

[ ] License headers in source files

[ ] Third-party license audit

**Launch Prep**

[ ] dashboard full update and revamp

**CI/CD**

[ ] fix broken ci pipleine gh workflow for cargo 

[ ] fix broken ci pipleine  gh workflow for pip  

[ ] fix broken ci pipleine  gh workflow for npm 

[ ] fix broken ci pipleine gh workflow for docker image  

[ ] cargo fuzz to test parser robustness.

[ ] Add`proptest for state consistency verification.

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

[x] Range search (distance threshold instead of top-k)

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
