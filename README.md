<img width="1114" height="191" alt="Screenshot 2025-11-23 at 12 47 47 AM" src="https://github.com/user-attachments/assets/efaa4c47-62d1-4397-9899-8bd58d400fc6" />

<p align="center">
    <b>Vector database for Agentic Applications</b>
</p>

---

## Roadmap

### Phase 1: Core Foundation 
- [x] Basic vector storage (HashMap + file persistence)
- [x] Binary serialization with bincode
- [x] UUID-based document IDs
- [x] Error handling with thiserror
- [x] Store and retrieve vectors by ID
- [x] Get all vectors
- [x] Persistence to disk

### Phase 2: Search & Similarity 
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

### Phase 3: Data Operations 
- [x] **Delete operations**
  - [x] Delete by ID
- [x] **Update operations**
  - [x] Update vector by ID
  - [x] Update metadata by ID

### Phase 4: HTTP Server 
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

### Phase 5: Built-in Embeddings (Next Priority)
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
  - [ ] Rate limiting / retry logic

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

### Phase 9: Performance & Indexing 
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

## Current Architecture

```
src/
├── lib.rs           # Public API exports
├── storage.rs       # VectorStorage - HashMap + bincode persistence
├── search.rs        # SearchResult type
├── metadata.rs      # MetadataValue enum + Metadata type alias
├── error.rs         # PiramidError + Result type
├── config.rs        # Config struct
├── metrics/         # Similarity metrics
│   ├── mod.rs       # SimilarityMetric enum
│   ├── cosine.rs    # Cosine similarity
│   ├── euclidean.rs # Euclidean distance
│   └── dot.rs       # Dot product
├── query/           # Filtering
│   ├── mod.rs
│   └── filter.rs    # Filter builder + FilterCondition
├── embeddings/      # Embedding providers
│   ├── mod.rs       # Embedder trait + types
│   ├── openai.rs    # OpenAI provider
│   ├── ollama.rs    # Ollama provider
│   └── providers.rs # Provider factory
├── server/          # HTTP API (axum)
│   ├── mod.rs
│   ├── routes.rs    # Route definitions
│   ├── handlers.rs  # Request handlers
│   ├── state.rs     # AppState + SharedState
│   └── types.rs     # Request/Response structs
└── bin/
    └── server.rs    # Main entry point

dashboard/           # Next.js admin UI
├── app/
│   ├── page.tsx     # Main dashboard
│   ├── components/  # React components
│   └── lib/api.ts   # API client
```

---

## Quick Start

### As a Library

```rust
use piramid::{VectorEntry, VectorStorage, SimilarityMetric, Filter, metadata};

// Open or create storage
let mut storage = VectorStorage::open("vectors.db").unwrap();

// Store a vector with metadata
let entry = VectorEntry::with_metadata(
    vec![0.1, 0.2, 0.3, 0.4],  // embedding
    "Hello world".to_string(), // text
    metadata([
        ("category", "greeting".into()),
        ("importance", 5i64.into()),
    ]),
);
let id = storage.store(entry).unwrap();

// Search for similar vectors
let query = vec![0.1, 0.2, 0.3, 0.4];
let results = storage.search(&query, 5, SimilarityMetric::Cosine);

for result in results {
    println!("{}: {} (score: {})", result.id, result.text, result.score);
}

// Filtered search
let filter = Filter::new()
    .eq("category", "greeting")
    .gt("importance", 3i64);
let filtered = storage.search_with_filter(&query, 5, SimilarityMetric::Cosine, Some(&filter));
```

### Via HTTP API

```bash
# Store a vector
curl -X POST http://localhost:6333/api/collections/docs/vectors \
  -H "Content-Type: application/json" \
  -d '{"vector": [0.1, 0.2, 0.3], "text": "hello", "metadata": {"tag": "test"}}'

# Search
curl -X POST http://localhost:6333/api/collections/docs/search \
  -H "Content-Type: application/json" \
  -d '{"vector": [0.1, 0.2, 0.3], "k": 5}'

# Embed text and store (requires EMBEDDING_PROVIDER env var)
curl -X POST http://localhost:6333/api/collections/docs/embed \
  -H "Content-Type: application/json" \
  -d '{"text": "The quick brown fox", "metadata": {"category": "example"}}'

# Search by text (auto-embeds the query)
curl -X POST http://localhost:6333/api/collections/docs/search/text \
  -H "Content-Type: application/json" \
  -d '{"query": "fast animals", "k": 5}'

# Batch embed multiple texts
curl -X POST http://localhost:6333/api/collections/docs/embed/batch \
  -H "Content-Type: application/json" \
  -d '{
    "texts": ["First document", "Second document", "Third document"],
    "metadata": [{"source": "doc1"}, {"source": "doc2"}, {"source": "doc3"}]
  }'
```

### Embedding Configuration

Enable embedding support by setting environment variables:

```bash
# Using OpenAI
export EMBEDDING_PROVIDER=openai
export EMBEDDING_MODEL=text-embedding-3-small
export OPENAI_API_KEY=sk-...

# Using Ollama (local)
export EMBEDDING_PROVIDER=ollama
export EMBEDDING_MODEL=nomic-embed-text
export EMBEDDING_BASE_URL=http://localhost:11434

# Then start the server
cargo run --bin piramid-server
```

## Run the Example

```bash
cargo run --example basic
```

---

## Running the Server

### Development

```bash
cargo run --bin piramid-server
```

Server runs at `http://localhost:6333`

### Dashboard (Next.js)

```bash
cd dashboard
npm install
npm run dev
```

Dashboard runs at `http://localhost:3000`

### Production (Docker)

```bash
docker-compose up
```

Both server and dashboard at `http://localhost:6333`

