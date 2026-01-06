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

### Phase 3: Data Operations (In Progress)
- [x] **Delete operations**
  - [x] Delete by ID
  - [ ] Bulk delete
- [x] **Update operations**
  - [x] Update vector by ID
  - [x] Update metadata by ID
- [ ] **Batch operations**
  - [ ] Batch insert (insert many vectors at once)
  - [ ] Batch search (multiple queries)
- [ ] **Validation**
  - [ ] Dimension consistency checks
  - [ ] Vector normalization option

### Phase 4: Indexing (Performance)
- [ ] **HNSW (Hierarchical Navigable Small World)**
  - [ ] Build HNSW graph on insert
  - [ ] Approximate nearest neighbor search
  - [ ] Configurable ef_construction and M parameters
- [ ] **IVF (Inverted File Index)** - optional
  - [ ] K-means clustering
  - [ ] Cluster-based search
- [ ] **Index persistence**
  - [ ] Save/load index to disk
  - [ ] Incremental index updates

### Phase 5: Performance Optimization
- [ ] **SIMD acceleration**
  - [ ] SIMD distance calculations (AVX2/AVX-512)
  - [ ] Portable SIMD fallback
- [ ] **Memory optimization**
  - [ ] Memory-mapped files (mmap)
  - [ ] Quantization (scalar/product quantization)
- [ ] **Parallel processing**
  - [ ] Parallel search with rayon
  - [ ] Concurrent inserts

### Phase 6: Collections & Organization
- [ ] **Collections/Namespaces**
  - [ ] Create/delete collections
  - [ ] Separate storage per collection
  - [ ] Collection-level configuration
- [ ] **Schema support**
  - [ ] Define expected dimensions per collection
  - [ ] Metadata schema validation

### Phase 7: Production Features
- [ ] **Async API**
  - [ ] Async storage operations
  - [ ] Tokio integration
- [ ] **Server mode** (optional)
  - [ ] REST API
  - [ ] gRPC API
- [ ] **Observability**
  - [ ] Metrics (insert latency, search latency, index size)
  - [ ] Logging

### Phase 8: Advanced Features (Future Innovation)
- [ ] Hybrid search (vector + keyword)
- [ ] Multi-vector documents
- [ ] Clustering & auto-organization
- [ ] Streaming inserts
- [ ] Replication
- [ ] Custom distance functions

---

## Quick Start

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

## Run the Example

```bash
cargo run --example basic
```

