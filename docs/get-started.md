Step 1: Core (15 min) - Must read these

  - src/storage.rs - THE MOST IMPORTANT FILE
    - VectorEntry - single vector (UUID + vector array + text + metadata)
    - VectorStorage - the database (HashMap in RAM, bincode on disk)
    - Methods: open(), store(), get(), search(), delete()
    - Key insight: Everything's in a HashMap. On every mutation, it rewrites the ENTIRE file (naive but simple - will be fixed later)
  - src/metrics/mod.rs then cosine.rs 
    - How to compare vectors: cosine similarity (most common), euclidean distance, dot product
    - Higher score = more similar
  - src/search.rs - What you get back from searches (ID + score + text + vector + metadata)
  - src/metadata.rs + src/query/filter.rs
    - Metadata = JSON-like {key: value} attached to vectors
    - Filter = query builder (.eq(), .gt(), .in_() etc)

Step 2: HTTP API (10 min)

  - src/server/state.rs - Shared app state (collections HashMap, optional embedder)
  - src/server/handlers.rs - All the endpoint logic (create collection, store vector, search, etc)
  - src/server/routes.rs - URL routing (POST /api/collections/:name/search)
  - src/bin/server.rs - Main entry point, reads env vars, starts server on port 6333

Step 3: Optional - Embeddings (5 min)

  - src/embeddings/ - OpenAI/Ollama clients that convert text → vectors

-----------------------------------------------------------------------------------------------------------------------------------------

How Things Flow:

  User Request → routes.rs → handlers.rs → storage.rs → metrics/ → SearchResult
                                  ↓
                            state.rs (shared collections)

Example search flow:

  - POST /api/collections/docs/search with query vector
  - handlers.rs extracts collection from state
  - Calls storage.search() 
  - Iterates ALL vectors, calculates similarity (cosine/euclidean)
  - Sorts by score, returns top k
  - Applies filters if provided

-----------------------------------------------------------------------------------------------------------------------------------------

Testing & Debugging:

  # Run all tests (27 tests, all should pass)
  cargo test
  
  # See output
  cargo test -- --nocapture
  
  # Run example (best way to understand!)
  cargo run --example basic
  
  # Start server
  cargo run --bin piramid-server
  
  # Test API
  curl -X POST http://localhost:6333/api/collections \
    -H "Content-Type: application/json" \
    -d '{"name": "test"}'
  
  curl -X POST http://localhost:6333/api/collections/test/vectors \
    -d '{"vector": [0.1, 0.2, 0.3], "text": "hello"}'

Debug tips:

  - Add println!("DEBUG: {:?}", variable); anywhere
  - Check test modules at bottom of files (#[cfg(test)])
  - Read examples/basic.rs - it's a complete walkthrough

-----------------------------------------------------------------------------------------------------------------------------------------

The problem: Right now it does brute-force search (checks EVERY vector). Slow af at scale. The fix: Implement HNSW (approximate nearest
neighbor) - Phase 1 in TODO.md

-----------------------------------------------------------------------------------------------------------------------------------------

Architecture at a Glance:

  Storage Layer:     HashMap<Uuid, VectorEntry> + disk file
  Metrics Layer:     Similarity calculations (cosine/euclidean/dot)
  Query Layer:       Metadata filtering
  Server Layer:      HTTP API (axum)
  Embeddings Layer:  Optional text→vector conversion

Data flow:

  - Writes: Add to HashMap → serialize entire HashMap to disk (slow, will be fixed with WAL)
  - Reads: Already in RAM (fast)
  - Search: Iterate ALL vectors, compute similarity, sort (O(n) - will be fixed with HNSW)

-----------------------------------------------------------------------------------------------------------------------------------------

Common Gotchas:

  - No persistent server state - If you restart, data persists but in-memory state rebuilds
  - Full file rewrites - Every mutation writes entire DB (Phase 9.5 will add WAL)
  - No indexes yet - Search is O(n) brute force (Phase 1 adds HNSW)
  - Thread-safe via Arc<Mutex> - Multiple requests can access collections safely
