• Beginner Priorities (read in this order)

  1) `src/bin/server.rs` – entrypoint. Loads config via `config::loader::load_runtime_config` (app + env-driven runtime settings like PORT/DATA_DIR/SLOW_QUERY_MS/embedding), optionally wires an embedder, builds the Axum router, and starts the server with graceful shutdown.
  2) `src/server/state.rs` – shared `AppState`: holds collections (DashMap of `Arc<RwLock<Collection>>`), embedder, shutdown flag, latency trackers, and config.
  3) `src/server/routes.rs` + `src/server/handlers/` – HTTP surface. Handlers show validation, lock timing (via metrics helper), and how storage/search are called. Unified endpoints: vectors/search/embed each accept single or batch payloads (see DTOs in `src/server/types.rs`).
  4) `src/storage/collection/` – core data path. Start with:
      • `builder.rs` (open/replay WAL, build indexes),
      • `operations.rs` (insert/delete/upsert),
      • `cache.rs` (vector cache),
      • `persistence/` (WAL + checkpoints).
  5) `src/index/` – vector indexes. `traits.rs` defines the interface; `selector.rs` picks Flat/HNSW/IVF; implementations in subfolders.
  6) `src/search/` – unified search engine + filters.
  7) `src/metrics/` – distance metrics and `LatencyTracker` (SIMD vs scalar).
  8) `src/embeddings/` – provider abstraction (OpenAI/Ollama), retry/cache wrappers.
  9) `src/validation.rs` – input checks (dims, batch sizes, names); reused everywhere.
 10) `src/quantization/mod.rs` – int8 quantization (how vectors are stored/loaded).

  How a request flows

  1. Startup: load config → create `AppState` (maybe with embedder) → build router.
  2. Request: Axum router → handler.
  3. Handler steps: validate input → `AppState::get_or_create_collection` (load/create collection with mmap, index, WAL) → perform op:
      - Insert/upsert: WAL log → serialize to mmap → update offset index → update vector index → update metadata → record latency.
      - Search: use cached vectors → index.search → score with metric → return DTOs → record latency.
      - Embed: call embedder (with retry/cache), then insert/search.
  4. Shutdown/checkpoint: op-count or time-based checkpoints; flush on shutdown.

  Folder Cheat Sheet

  - src/server/ – HTTP API wiring, state, DTOs, metrics helpers.
  - src/storage/ – on-disk layout, WAL, mmap, collection CRUD/search, cache, persistence.
  - src/index/ – vector index interface + Flat/HNSW/IVF implementations.
  - src/search/ – unified search engine and filters.
  - src/embeddings/ – provider-agnostic embedding layer.
  - src/metrics/ – distance metrics + latency tracking.
  - src/config/ – config types and loader.
  - dashboard/, website/, sdk/ – UI, marketing, and client SDKs (not core server).


• Request Flow (text-based sequence)

  - Startup: piramid-server reads env → builds AppState (collections map + optional embedder + latency trackers) → create_router wires routes → Axum serve with graceful
    shutdown.
  - Insert Vector: Client POST /api/collections/{c}/vectors → handler validates (vector/text/collection) → AppState::get_or_create_collection loads/creates Collection
    (mmap, offset index, vector index, WAL) → lock collection (write) → WAL log Insert → serialize Document (quantized vector) into mmap, update offset index, update
    vector index, update metadata dims/count → record latency → return id.
  - Batch Insert: Same as insert but bulk WAL entries, bulk serialize into mmap, then batch insert into vector index.
  - Upsert: Parse/generate id → if exists, WAL Update, remove old index entries, reinsert; else behaves like insert; latency recorded as insert/update.
  - Delete / Batch Delete: Validate → WAL Delete entries → remove from in-memory offset index + vector index → save index/vector-index sidecars → return deleted count.
  - Get/List: Read lock collection → fetch by UUID (deserialize from mmap) or iterate get_all with offset/limit → return DTOs.
  - Search (vector): Validate → read lock collection → build HashMap<Uuid, Vec<f32>> view → VectorIndex::search (Flat/HNSW/IVF) returns neighbor ids → for each id,
    deserialize doc, compute score via metric (cosine/euclidean/dot) → return hits (id/score/text/metadata) → record latency.
  - Batch Search: Same but over multiple queries (rayon if enabled), returns list-of-lists of hits.
  - Filtered Search (used when filter provided): Run wider search (k*10) → filter hits by metadata predicate → sort+truncate to k.
  - Embed + Store: POST /embed → call embedder (OpenAI/Ollama via RetryEmbedder/cache) to get vector(s) → then go through insert path → return id(s)+embedding(s)+tokens.
  - Text Search: Embed query text via embedder → run normal vector search → return hits.
  - Collections CRUD: Create just ensures collection exists (touches disk/index metadata); list enumerates loaded collections with counts; delete removes files/entries.
  - Metrics/Health: Read-only endpoints summarize counts, index type, memory usage, latency stats, and embedder status.
  - Shutdown: Ctrl+C → set shutting_down flag (new requests rejected) → checkpoint all collections (flush mmap/index/vector-index/metadata) → graceful drain.
