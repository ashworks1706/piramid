
### Production 

**Index Management**

[x] Product Quantization (PQ) (CPU compression)

[x] HNSW tombstoning: soft-delete nodes without breaking graph connectivity

### Documentation & Testing

**Launch Prep**

[ ] Remove redundant functions
[ ] Maintain proper logging
[ ] Codebase final refactor
[ ] seperate tests file, get rid of tests on the same file

**Dashboard**rog

[ ] Dashboard full update with functionality
[ ] Final docker image push

**CI/CD**

[ ] Fix broken CI pipeline GitHub workflow for cargo 

[ ] Remove npm sdk CI pipeline github workflow for now, we will add it later 

[ ] Fix broken CI pipeline GitHub workflow for docker image  

[ ] Cargo fuzz to test parser robustness.


**Documentation**

[ ] Easy low effort API docs for SDKs (Rust via MkDocs)

[ ] Entire Technical architecture breakdown (MkDocs)

[ ] docs/CONTRIBUTION.md - 5-minute tutorial and updates

[ ] CHANGELOG.md - Version tracking

[ ] README.md - update readme

[ ] License headers in source files

[ ] Third-party license audit

---

### Post-Launch 

**ACID Transactions**

[ ] Atomic batch operations (all-or-nothing)

[ ] Rollback on failure

[ ] Isolation (at least serializable)

[ ] Idempotency keys

[ ] Request deduplication

**Async Storage I/O**

[ ] Non-blocking writes (tokio-fs)

[ ] Async write pipeline (batching/coalescing, buffering, background flush worker)

[ ] Prefetching for sequential reads

[ ] Background job queue for long operations

**Query Optimization**

[ ] Query result caching

[ ] Query planning/optimization

[ ] Query budget enforcement (timeouts, complexity limits)

**Backup & Restore**

[ ] Snapshot API (copy-on-write)

[ ] Point-in-time recovery (PITR)

[ ] Incremental backups

[ ] Database Migrations 

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


**Python Support**

[ ] Python client SDK

[ ] Add docs 

[ ] Easy low effort API docs for SDKs (Rust via MkDocs)

**CLI**

[ ] add cli for piramid setup

[ ] use piramid cli animations

---

### Future Considerations

**Advanced Search**

[ ] Recommendation API (similar to these IDs, not those)

[ ] Grouped/diverse search (max results per category)

[ ] Scroll/pagination for large result sets

[ ] Metadata-only search (no vector similarity)

[ ] Vector similarity beti thinkween two stored vectors

[ ] Vector count per metadata filter

[ ] SQL integration

**Additional Features**

[ ] Corrupted file detection + auto-repair

[ ] Automatic index rebuild on corruption

[ ] Circuit breaker for embedding API failures

[ ] Collection aliases

[ ] Move collection between directories

[ ] Client side distributed Systems

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

[ ] Dual-Write Architecture: Ensure insert_vector writes to both Disk (Persistence) and Zipy (VRAM).

[ ] Search Router: Implement logic to route POST /search requests to Zipy when active.

[ ] Fallback Circuit Breaker: Auto-switch to CPU search if Zipy returns OOM or timeout errors.

[ ] Health Check Extension: Add GPU status (temperature, memory usage) to /api/health.


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


**Data Import/Export**

[ ] Import from JSON/CSV/Parquet

[ ] Export to JSON/CSV/Parquet

[ ] Streaming import for large datasets

[ ] Import progress tracking

[ ] Format validation on import
