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
- [x] **Distance metrics module**
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
- [ ] **Embedding providers module**
  - [ ] OpenAI (text-embedding-3-small, text-embedding-3-large)
  - [ ] Azure OpenAI
  - [ ] Cohere (embed-english-v3.0)
  - [ ] Ollama (local models - nomic-embed-text, mxbai-embed-large)
  - [ ] Voyage AI
  - [ ] HuggingFace Inference API
- [ ] **Text-to-vector API endpoints**
  - [ ] `POST /api/collections/{name}/embed` - embed text and store
  - [ ] `POST /api/collections/{name}/search/text` - search by text query
- [ ] **Configuration**
  - [ ] Provider selection via env vars / config
  - [ ] API key management
  - [ ] Model selection per collection
- [ ] **Batch embedding**
  - [ ] Batch embed multiple texts in one request
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
├── distance/        # Distance metrics
│   ├── mod.rs       # DistanceMetric enum
│   ├── cosine.rs    # Cosine similarity
│   ├── euclidean.rs # Euclidean distance
│   └── dot.rs       # Dot product
├── query/           # Filtering
│   ├── mod.rs
│   └── filter.rs    # Filter builder + FilterCondition
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
use piramid::{VectorEntry, VectorStorage, DistanceMetric, Filter, metadata};

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
let results = storage.search(&query, 5, DistanceMetric::Cosine);

for result in results {
    println!("{}: {} (score: {})", result.id, result.text, result.score);
}

// Filtered search
let filter = Filter::new()
    .eq("category", "greeting")
    .gt("importance", 3i64);
let filtered = storage.search_with_filter(&query, 5, DistanceMetric::Cosine, Some(&filter));
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

