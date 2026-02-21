# Roadmap

This is the working roadmap for contributors. If you want to help, start here and pick one scoped task. If your idea is not listed but adjacent, open an issue first and propose it before implementation.

### Documentation & Testing


**Blogs**

[ ] add vector database components diagram on evolution page
[ ] add more images for the database page methadologies,  links for bigger terms  
[ ] add more images for the embeddings page, link my workshop content there, links for bigger terms
[ ] add more images for storage page, links for bigger terms 
[ ] add more images for indexing page and links for bigger terms 
[ ] fix UI on overview page, make the block card clickable, make responsive on mobile 
[ ] add section '#' icons, right sidebar embedded link formatting, leftsidebar page route highlighting 
[ ] 

**Documentation**

- [ ] Separate API docs to `docs.piramiddb.com` (Mintlify)

**Dashboard**

- [ ] Fix dashboard UI
- [ ] Add more metrics to dashboard (memory usage, disk usage, query latency, etc.)
- [ ] Fix Docker image

---

### Post-Launch

**Benchmarks**

- [ ] Set up benchmarks for latency, index strategies, memory usage, etc.

**ACID Transactions**

- [ ] Atomic batch operations (all-or-nothing)
- [ ] Rollback on failure
- [ ] Isolation (at least serializable)
- [ ] Idempotency keys
- [ ] Request deduplication

**Async Storage I/O**

- [ ] Non-blocking writes (`tokio-fs`)
- [ ] Async write pipeline (batching/coalescing, buffering, background flush worker)
- [ ] Prefetching for sequential reads
- [ ] Background job queue for long operations

** Regular codebase refreshment**
- [ ] refactor codebase for better modularity and maintainability
- [ ] add more unit tests and integration tests
- [ ] make sure ci cd pipeline is robust and covers all critical paths
- [ ] update documentation to reflect any code changes and new features 
- [ ] update blogs to reflect any code changes and new features 

**Query Optimization**

- [ ] Query result caching
- [ ] Query planning/optimization
- [ ] Query budget enforcement (timeouts, complexity limits)

**Backup & Restore**

- [ ] Snapshot API (copy-on-write)
- [ ] Point-in-time recovery (PITR)
- [ ] Incremental backups
- [ ] Database migrations

** Regular codebase refreshment**
- [ ] refactor codebase for better modularity and maintainability
- [ ] add more unit tests and integration tests
- [ ] make sure ci cd pipeline is robust and covers all critical paths
- [ ] update documentation to reflect any code changes and new features 
- [ ] update blogs to reflect any code changes and new features 

**Metadata Improvements**

- [ ] Complex filters (AND/OR/NOT combinations)
- [ ] Metadata indexing for fast filtering
- [ ] Range queries on numeric metadata
- [ ] Regex/pattern matching on string metadata
- [ ] Date range filters
- [ ] Array membership checks

** Regular codebase refreshment**
- [ ] refactor codebase for better modularity and maintainability
- [ ] add more unit tests and integration tests
- [ ] make sure ci cd pipeline is robust and covers all critical paths
- [ ] update documentation to reflect any code changes and new features 
- [ ] update blogs to reflect any code changes and new features 

**Schema Support**

- [ ] Define expected dimensions per collection
- [ ] Metadata schema validation

**Python Support**

- [ ] Python client SDK
- [ ] Add docs
- [ ] Easy API docs for SDKs (Rust via MkDocs)

---

### Future Considerations

**Distributed Systems**

- [ ] Sharding strategies (range, hash, etc.)
- [ ] Replication strategies (master-slave, multi-master, etc.)
- [ ] Consistency models (strong, eventual, etc.)
- [ ] Distributed transactions
- [ ] Cluster management (node discovery, leader election, etc.)

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
- [ ] Client-side distributed systems support

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
