# Documentation TODO

These files need to be created/updated as development progresses:

### Phase 1: Core Foundation ✅ **COMPLETED** 
- [x] Basic vector storage (HashMap + file persistence)
- [x] Binary serialization with bincode
- [x] UUID-based document IDs
- [x] Error handling with thiserror
- [x] Store and retrieve vectors by ID
- [x] Get all vectors
- [x] Persistence to disk

### Phase 2: Search & Similarity ✅ **COMPLETED**
- [x] **Similarity metrics module**
  - [x] Cosine similarity
  - [x] Euclidean distance
  - [x] Dot product
- [x] **Similarity search API**
  - [x] `search(query_vector, top_k)` → returns nearest neighbors
  - [x] Return results with scores
- [x] **Metadata support**
  - [x] Add `metadata: HashMap<String, Value>` to VectorEntry
  - [x] JSON-like metadata storage
- [x] **Filtered search**
  - [x] Filter by metadata during search
  - [x] Support basic operators (eq, ne, gt, gte, lt, lte, in)

### Phase 3: Data Operations ✅ **COMPLETED** 
- [x] **Delete operations**
  - [x] Delete by ID
- [x] **Update operations**
  - [x] Update vector by ID
  - [x] Update metadata by ID

### Phase 4: HTTP Server ✅ **COMPLETED** 
- [x] **REST API (axum)**
  - [x] Health endpoint
  - [x] Collections CRUD
  - [x] Vectors CRUD
  - [x] Search endpoint
  - [x] CORS support
- [x] **Dashboard (Next.js)**
  - [x] Static export embedded in Rust server
  - [x] Collection management UI
  - [x] Vector browsing
  - [x] Search interface

### Phase 5: Built-in Embeddings ✅ **COMPLETED**
*no need to embed before storing*
- [x] **Embedding providers module**
  - [x] OpenAI (text-embedding-3-small, text-embedding-3-large)
  - [x] Ollama (local models - nomic-embed-text, mxbai-embed-large)
  - [ ] HuggingFace Inference API
- [x] **Text-to-vector API endpoints**
  - [x] `POST /api/collections/{name}/embed` - embed text and store
  - [x] `POST /api/collections/{name}/search/text` - search by text query
- [x] **Configuration**
  - [x] Provider selection via env vars / config
  - [x] API key management
  - [x] Model selection per collection
- [x] **Batch embedding**
  - [x] Batch embed multiple texts in one request
  - [ ] Rate limiting / retry logic (→ moved to Phase 10.5)

### Phase 6: Document Ingestion 
*Upload docs, auto-chunk, auto-embed*
- [ ] **Chunking strategies**
  - [ ] Fixed-size chunking (by tokens/characters)
  - [ ] Semantic chunking (sentence/paragraph boundaries)
  - [ ] Recursive character splitter
  - [ ] Overlap configuration
- [ ] **Document upload endpoint**
  - [ ] `POST /api/collections/{name}/ingest` - upload raw text/file
  - [ ] PDF support (via pdf-extract or similar)
  - [ ] Markdown/HTML support
- [ ] **Chunk metadata**
  - [ ] Auto-add chunk index, source document ID
  - [ ] Parent-child relationships

### Phase 7: MCP (Model Context Protocol) Integration 
*Let AI agents discover and walk your data*
- [ ] **MCP server implementation**
  - [ ] Built-in MCP tool definitions
  - [ ] `search_similar` tool
  - [ ] `get_document` tool
  - [ ] `list_collections` tool
- [ ] **Agent-friendly responses**
  - [ ] Structured output formats
  - [ ] Context window aware truncation

### Phase 8: Hybrid Search 
*Vector + keyword search combined*
- [ ] **BM25 keyword search**
  - [ ] Inverted index for text fields
  - [ ] TF-IDF scoring
- [ ] **Hybrid ranking**
  - [ ] Reciprocal Rank Fusion (RRF)
  - [ ] Configurable vector/keyword weights
- [ ] **Full-text search endpoint**
  - [ ] `POST /api/collections/{name}/search/hybrid`

### Phase 9: Performance & Indexing ⚡ **HIGH PRIORITY**
- [ ] **HNSW (Hierarchical Navigable Small World)**
  - [ ] Build HNSW graph on insert
  - [ ] Approximate nearest neighbor search
  - [ ] Configurable ef_construction and M parameters
- [ ] **SIMD acceleration**
  - [ ] SIMD distance calculations (AVX2/AVX-512)
  - [ ] Portable SIMD fallback
- [ ] **Memory optimization**
  - [ ] Memory-mapped files (mmap)
  - [ ] Scalar quantization (int8)
- [ ] **Parallel processing**
  - [ ] Parallel search with rayon
  - [ ] Concurrent inserts

### Phase 9.5: Data Durability & Integrity 🔴 **CRITICAL - MUST DO BEFORE PRODUCTION**
*Production databases don't lose your data*
- [ ] **Write-Ahead Log (WAL)**
  - [ ] Append-only log for all mutations
  - [ ] Recovery from WAL on crash/restart
  - [ ] Periodic checkpointing to main storage
  - [ ] Configurable fsync strategies (performance vs durability)
- [ ] **ACID Transactions**
  - [ ] Atomic batch operations (all-or-nothing)
  - [ ] Rollback on failure
  - [ ] Isolation levels (at least serializable)
  - [ ] Transaction log for debugging
- [ ] **Graceful shutdown & recovery**
  - [ ] Flush pending writes on SIGTERM/SIGINT
  - [ ] Clean lock release on shutdown
  - [ ] Corrupted file detection on startup
  - [ ] Auto-repair minor corruption
  - [ ] Emergency read-only mode
- [ ] **Backup & Restore**
  - [ ] Snapshot API (copy-on-write)
  - [ ] Point-in-time recovery (PITR)
  - [ ] Export/import collections (portable format)
  - [ ] Incremental backups
  - [ ] Verify backup integrity
- [ ] **Error handling hardening**
  - [ ] Replace all .unwrap() with proper error types
  - [ ] Graceful degradation on failures
  - [ ] Poison-free lock handling (no panics while holding locks)
  - [ ] Retry logic with exponential backoff
- [ ] **Async storage I/O**
  - [ ] Non-blocking disk writes
  - [ ] Async file handles (tokio-fs)
  - [ ] Background flush worker
  - [ ] Write batching/coalescing

### Phase 10: Production Features 
- [ ] **Batch operations**
  - [ ] Batch insert (insert many vectors at once)
  - [ ] Batch search (multiple queries)
  - [ ] Bulk delete
- [ ] **Validation**
  - [ ] Dimension consistency checks per collection
  - [ ] Vector normalization option
- [ ] **Observability**
  - [ ] Metrics (insert latency, search latency, index size)
  - [ ] Structured logging (tracing)
  - [ ] Prometheus endpoint
- [ ] **Schema support**
  - [ ] Define expected dimensions per collection
  - [ ] Metadata schema validation
- [ ] **gRPC API**
  - [ ] Alternative to REST for performance

### Phase 10.5: Security & Authentication 🔒 **HIGH PRIORITY**
*Don't let anyone delete your production data*
- [ ] **Authentication**
  - [ ] API key authentication
  - [ ] JWT token support
  - [ ] Multi-tenant isolation
  - [ ] Service-to-service auth (mTLS)
- [ ] **Authorization**
  - [ ] Role-based access control (RBAC)
  - [ ] Collection-level permissions (read/write/admin)
  - [ ] Read-only vs read-write users
  - [ ] Fine-grained operation permissions
- [ ] **Rate limiting & quotas**
  - [ ] Per-client rate limits (requests/second)
  - [ ] Per-collection quotas (vector count, storage size)
  - [ ] Quota enforcement
  - [ ] DDoS protection (connection limits)
  - [ ] Slow-query detection & throttling
- [ ] **Security hardening**
  - [ ] Input validation & sanitization
  - [ ] SQL injection prevention (if adding SQL features)
  - [ ] Request size limits
  - [ ] TLS/SSL enforcement
  - [ ] Security headers (CORS, CSP)

### Phase 11: GPU Acceleration 
*most vector DBs are CPU-only*
- [ ] **GPU-accelerated distance calculations**
  - [ ] wgpu backend (cross-platform: Vulkan/Metal/DX12/WebGPU)
  - [ ] Optional CUDA backend for NVIDIA GPUs (cudarc)
  - [ ] Automatic fallback to CPU SIMD
- [ ] **Batch operations on GPU**
  - [ ] Batch search (100+ queries) - 10-100x faster
  - [ ] Brute-force search on large collections
  - [ ] Matrix multiplication for distance calculations
- [ ] **GPU memory management**
  - [ ] Keep hot vectors in VRAM
  - [ ] Async transfer between CPU/GPU
  - [ ] LRU eviction for large collections
- [ ] **Local embedding models on GPU**
  - [ ] Candle integration for Rust-native inference
  - [ ] GGUF model support (nomic-embed, bge, etc.)
  - [ ] Same GPU for embedding + search (zero round-trip)
- [ ] **Quantized GPU operations**
  - [ ] INT8/FP16 tensor core acceleration
  - [ ] Reduced VRAM usage

### Phase 12: Advanced Features 
- [ ] Multi-vector documents
- [ ] Clustering & auto-organization
- [ ] Streaming inserts
- [ ] Replication
- [ ] Sharding
- [ ] Custom distance functions
- [ ] Graph relationships between vectors (like HelixDB)

### Phase 13: Semantic Cache for LLMs 
*Cache LLM responses by meaning, not exact match - save 70%+ on API costs*
- [ ] **Semantic matching**
  - [ ] Hash query embeddings for fast lookup
  - [ ] Configurable similarity threshold
  - [ ] "What's the capital of France?" ≈ "Tell me France's capital"
- [ ] **Cache management**
  - [ ] TTL (time-to-live) per entry
  - [ ] LRU eviction
  - [ ] Manual invalidation API
- [ ] **LLM integration helpers**
  - [ ] OpenAI/Anthropic response caching
  - [ ] Token usage tracking
  - [ ] Cost savings dashboard

### Phase 14: WebAssembly (WASM) - Run Anywhere 
*Rust's superpower - Piramid in the browser, edge, mobile*
- [ ] **Browser runtime**
  - [ ] Compile core to WASM
  - [ ] Client-side vector search (no server needed)
  - [ ] IndexedDB persistence
- [ ] **Edge deployment**
  - [ ] Cloudflare Workers compatible
  - [ ] Vercel Edge Functions
  - [ ] Deno Deploy
- [ ] **Embedded use cases**
  - [ ] React Native / Flutter integration
  - [ ] Desktop apps (Tauri)
  - [ ] Offline-first applications

### Phase 15: Agent Memory System 
*Purpose-built for AI agents, not just RAG*
- [ ] **Memory types**
  - [ ] Working Memory - current conversation context
  - [ ] Episodic Memory - past interactions, time-decayed
  - [ ] Semantic Memory - long-term knowledge
  - [ ] Procedural Memory - learned tool usage patterns
- [ ] **Memory management**
  - [ ] Importance scoring (what to remember)
  - [ ] Auto-consolidation (compress old memories)
  - [ ] Cross-session persistence
  - [ ] Memory retrieval by recency + relevance
- [ ] **Agent integrations**
  - [ ] LangChain/LlamaIndex memory backend
  - [ ] AutoGPT/CrewAI compatible

### Phase 16: Temporal Vectors (Time-Travel) 
*Version control for embeddings*
- [ ] **Vector versioning**
  - [ ] Query: "What was similar to X as of 3 months ago?"
  - [ ] Track embedding drift over time
  - [ ] Rollback bad embedding updates
- [ ] **A/B testing embeddings**
  - [ ] Compare embedding models without migration
  - [ ] Shadow indexing with new models
- [ ] **Audit trail**
  - [ ] Who changed what, when
  - [ ] Compliance-friendly logging

### Phase 17: Privacy-First / Local-Only Mode 
*GDPR, HIPAA, enterprise-ready*
- [ ] **Zero network mode**
  - [ ] All embeddings via local models (Ollama/candle)
  - [ ] No telemetry, no external calls
  - [ ] Air-gapped deployment support
- [ ] **Encryption**
  - [ ] Encrypted at rest (AES-256)
  - [ ] Encrypted in transit (TLS)
  - [ ] Key management integration (Vault, KMS)
- [ ] **Compliance features**
  - [ ] Audit logs
  - [ ] Data residency controls
  - [ ] Right to deletion (GDPR Article 17)

### Phase 18: Auto-Pilot Mode 
*Zero-config optimization - it just works*
- [ ] **Auto-indexing**
  - [ ] Auto-select HNSW vs brute-force based on collection size
  - [ ] Auto-tune M and ef_construction parameters
  - [ ] Rebuild index in background when beneficial
- [ ] **Auto-optimization**
  - [ ] Auto-quantize when memory is tight
  - [ ] Auto-batch small inserts
  - [ ] Query pattern analysis → index hints
- [ ] **Smart defaults**
  - [ ] Suggest embedding model based on your data
  - [ ] Warn about dimension mismatches
  - [ ] Performance recommendations in dashboard

---

## 🏆 Production Readiness Tracker

### Phases Required for Production (v1.0)
| Phase | Status | Priority | Blocks Production? |
|-------|--------|----------|-------------------|
| Phase 1-5 | ✅ Complete | N/A | Already done |
| **Phase 9** | ⏳ Pending | 🔴 Critical | **YES** - Need HNSW indexing |
| **Phase 9.5** | ⏳ Pending | 🔴 Critical | **YES** - Need WAL/ACID |
| **Phase 10** | ⏳ Pending | 🔴 Critical | **YES** - Need observability |
| **Phase 10.5** | ⏳ Pending | 🔴 Critical | **YES** - Need auth/security |

### Feature Expansion (v1.x)
| Phase | Priority | Can Deploy Without? |
|-------|----------|---------------------|
| Phase 6 | 🟡 Medium | Yes - users can chunk manually |
| Phase 7 | 🟡 Medium | Yes - MCP is nice-to-have |
| Phase 8 | 🟡 Medium | Yes - vector-only is viable |

### Advanced Features (v2.0+)
| Phase | Status | Competitive Advantage |
|-------|--------|----------------------|
| Phase 11 | 🟢 Future | **HIGH** - GPU acceleration (unique) |
| Phase 12 | 🟢 Future | Medium - Replication/sharding (table stakes) |
| Phase 13 | 🟢 Future | **HIGH** - Semantic cache (unique) |
| Phase 14 | 🟢 Future | **HIGH** - WASM (unique) |
| Phase 15 | 🟢 Future | **HIGH** - Agent memory (unique) |
| Phase 16-18 | 🟢 Future | Medium - Nice differentiators |

### Comparison with Competitors (After Phases 9-10.5)
| Feature | Piramid v1.0 | Qdrant | HelixDB | Milvus |
|---------|--------------|--------|---------|--------|
| HNSW Indexing | ✅ (Phase 9) | ✅ | ✅ | ✅ |
| SIMD Acceleration | ✅ (Phase 9) | ✅ | ✅ | ✅ |
| WAL/ACID | ✅ (Phase 9.5) | ✅ | ✅ | ✅ |
| Auth/RBAC | ✅ (Phase 10.5) | ✅ | ✅ | ✅ |
| Observability | ✅ (Phase 10) | ✅ | ✅ | ✅ |
| Replication | ❌ (Phase 12) | ✅ | ✅ | ✅ |
| **GPU Acceleration** | 🎯 (Phase 11) | ❌ | ❌ | Limited |
| **Semantic Cache** | 🎯 (Phase 13) | ❌ | ❌ | ❌ |
| **WASM Support** | 🎯 (Phase 14) | ❌ | ❌ | ❌ |
| **Agent Memory** | 🎯 (Phase 15) | ❌ | ❌ | ❌ |

🎯 = Unique competitive advantage after implementation

---

