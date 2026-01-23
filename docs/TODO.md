# Piramid TODO - Build Order & Documentation

**Use this as your single source of truth for what to build and when.**

**Follow this exact order. Each step depends on the previous.**

### Core Foundation
- [x] Vector storage (HashMap + bincode persistence)
- [x] UUID-based IDs, error handling
- [x] Store, retrieve, get all, delete, update

### Search & Similarity
- [x] Similarity metrics (cosine, euclidean, dot product)
- [x] Top-k search with scores
- [x] Metadata support with filtering

### HTTP Server & Embeddings
- [x] REST API (axum), Collections/Vectors CRUD
- [x] Dashboard (Next.js)
- [x] Embedding providers (OpenAI, Ollama)
- [x] Batch embedding endpoints

### Performance & Indexing 

**Why first:** Without HNSW, everything else is unusably slow at scale.

**Implementation:**
- [ ] **HNSW indexing**
  - [ ] Build HNSW graph on insert
  - [ ] Approximate nearest neighbor search (O(log n))
  - [ ] Configurable ef_construction and M parameters
  - [ ] Benchmark: 1M vectors in <10ms
- [ ] **SIMD acceleration**
  - [ ] SIMD distance calculations (AVX2/AVX-512)
  - [ ] Portable SIMD fallback
  - [ ] 3-5x speedup target
- [ ] **Memory optimization**
  - [ ] Memory-mapped files (mmap)
  - [ ] Scalar quantization (int8) - 4x memory reduction
  - [ ] Handle 10M vectors on 32GB RAM
- [ ] **Parallel processing**
  - [ ] Parallel search with rayon
  - [ ] Concurrent inserts
  - [ ] Linear scaling with CPU cores

**Goal:** Search 1M vectors in <10ms

---

### 2️⃣ Data Durability & Integrity 

**Implementation:**
- [ ] **Write-Ahead Log (WAL)**
  - [ ] Append-only log for all mutations
  - [ ] Recovery from WAL on crash/restart
  - [ ] Periodic checkpointing
  - [ ] Configurable fsync strategies
  - [ ] Test: Kill process mid-write, verify no data loss
- [ ] **ACID Transactions**
  - [ ] Atomic batch operations (all-or-nothing)
  - [ ] Rollback on failure
  - [ ] Isolation (at least serializable)
  - [ ] Transaction log
- [ ] **Graceful shutdown & recovery**
  - [ ] Flush on SIGTERM/SIGINT
  - [ ] Clean lock release
  - [ ] Corrupted file detection + auto-repair
  - [ ] Emergency read-only mode
- [ ] **Backup & Restore**
  - [ ] Snapshot API (copy-on-write)
  - [ ] Point-in-time recovery (PITR)
  - [ ] Export/import (portable format)
  - [ ] Incremental backups
- [ ] **Error handling hardening**
  - [ ] Replace ALL `.unwrap()` with proper errors
  - [ ] Graceful degradation
  - [ ] Poison-free lock handling
  - [ ] Retry logic with exponential backoff
- [ ] **Async storage I/O**
  - [ ] Non-blocking writes (tokio-fs)
  - [ ] Background flush worker
  - [ ] Write batching/coalescing

**Goal:** Zero data loss on crashes. Pass chaos engineering tests.

---

### 3️⃣ Production Features 

**Why third:** Can't run #1+#2 in production without knowing what's happening.

**Implementation:**
- [ ] **Observability**
  - [ ] Metrics (insert/search latency, index size, memory)
  - [ ] Structured logging (tracing crate)
  - [ ] Prometheus endpoint (`/metrics`)
  - [ ] Enhanced health checks
- [ ] **Batch operations**
  - [ ] Batch insert (10k inserts in <1s)
  - [ ] Batch search
  - [ ] Bulk delete
- [ ] **Validation**
  - [ ] Dimension consistency checks
  - [ ] Vector normalization option
  - [ ] Clear error messages
- [ ] **Schema support**
  - [ ] Define expected dimensions per collection
  - [ ] Metadata schema validation
  - [ ] Schema versioning
- [ ] **gRPC API** (optional but recommended)
  - [ ] Alternative to REST
  - [ ] Streaming inserts
  - [ ] Bi-directional streaming

---

### 4️⃣ Security & Authentication 

**Why fourth:** Now that it works and is observable, prevent abuse.

**Implementation:**
- [ ] **Authentication**
  - [ ] API key authentication
  - [ ] JWT token support
  - [ ] Multi-tenant isolation
  - [ ] Service-to-service auth (mTLS)
- [ ] **Authorization**
  - [ ] Role-based access control (RBAC)
  - [ ] Collection-level permissions
  - [ ] Read-only vs read-write users
  - [ ] Admin APIs
- [ ] **Rate limiting & quotas**
  - [ ] Per-client rate limits (requests/second)
  - [ ] Per-collection quotas
  - [ ] DDoS protection
  - [ ] Slow-query throttling
- [ ] **Security hardening**
  - [ ] Input validation & sanitization
  - [ ] Request size limits
  - [ ] TLS/SSL enforcement
  - [ ] Security headers (CORS, CSP, HSTS)
  - [ ] Audit logging

---

**→ Deploy to production, get real users, gather feedback**

**General v1.0 documentation:**
- [ ] `CONTRIBUTING.md` - Contribution guidelines
- [ ] `CHANGELOG.md` - Start version tracking
- [ ] `docs/API.md` - Complete REST API reference
- [ ] `docs/QUICKSTART.md` - 5-minute tutorial

---

### Document Ingestion 
*Upload docs instead of pre-chunking*

- [ ] Chunking strategies (fixed-size, semantic, recursive)
- [ ] Document upload endpoint (PDF, DOCX, Markdown, HTML)
- [ ] Chunk metadata (index, source doc, parent-child)

**Goal:** Upload PDF → auto-chunk → auto-embed → search

---

### MCP Integration 
*AI agents can use Piramid*

- [ ] MCP server implementation
- [ ] Tools: search_similar, get_document, list_collections, add_document
- [ ] Agent-friendly responses (structured JSON-LD)

**Goal:** Claude Desktop can use Piramid out of the box

---

### Hybrid Search 
*Vector + keyword combined*

- [ ] BM25 keyword search (inverted index, TF-IDF)
- [ ] Hybrid ranking (RRF, configurable weights)
- [ ] Full-text search endpoint with Boolean queries

**Goal:** Search "rust programming" → semantic + exact matches

---

### GPU Acceleration 
- [ ] wgpu backend (cross-platform GPU)
- [ ] Optional CUDA for NVIDIA
- [ ] Batch search on GPU (10-100x faster)
- [ ] Local embedding models on GPU

**Goal:** Search 10M vectors in <1ms

---

### Distributed System 
- [ ] Replication (master-slave, multi-master)
- [ ] Sharding (horizontal partitioning)
- [ ] Distributed queries (scatter-gather)
- [ ] Cluster management

**Goal:** Scale to billions of vectors

---

### Semantic Cache 
- [ ] Semantic matching for LLM responses
- [ ] TTL and LRU eviction
- [ ] OpenAI/Anthropic integration
- [ ] Cost savings dashboard

**Goal:** Save 70%+ on LLM costs

---

### WASM Support 
- [ ] Compile core to WASM
- [ ] Client-side vector search
- [ ] Edge deployment (Cloudflare, Vercel)
- [ ] Offline-first apps

**Goal:** Vector search in browser

---

### Agent Memory System 
- [ ] Memory types (working, episodic, semantic, procedural)
- [ ] Importance scoring & auto-consolidation
- [ ] LangChain/LlamaIndex integration

**Goal:** Agents that learn across sessions

---

### Other Advanced Features
- [ ] Temporal Vectors (time-travel queries)
- [ ] Privacy Mode (GDPR/HIPAA, encryption)
- [ ] Auto-Pilot (self-tuning, auto-optimization)

