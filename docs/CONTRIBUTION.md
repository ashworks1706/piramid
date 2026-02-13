• Beginner Priorities

  - src/bin/server.rs – entrypoint: builds config via `config::loader::load_app_config`, wires optional embedder, spins up Axum server with graceful shutdown and checkpoints.
  - src/server/state.rs – shared AppState: collection map, embedder, shutdown flag, latency trackers. Learn how Arc/RwLock/DashMap are used.
  - src/server/routes.rs + src/server/handlers/ – HTTP surface. Handlers show validation, locking (via metrics helper), and how storage/search are called. Great place
    to see async/axum patterns.
  - src/storage/collection/ – core database: mmap-backed file + in-memory index map + pluggable vector index + WAL. Files to skim: builder.rs (open/replay WAL),
    operations.rs (insert/delete/upsert), cache.rs (vector cache), persistence/ (WAL + checkpoints).
  - src/index/ – vector index strategies. traits.rs defines the interface; selector.rs picks Flat/HNSW/IVF; implementations live in subfolders.
  - src/metrics – cosine/euclidean/dot and LatencyTracker; shows SIMD mode handling.
  - src/embeddings/ – embedding abstraction + providers (OpenAI/Ollama), retry/cache wrappers.
  - src/validation.rs – input checks (dims, batch sizes, names); reused everywhere.
  - src/quantization/mod.rs – int8 quantization; understand how vectors are stored/loaded.

  Architecture Flow

  1. Startup (src/bin/server.rs): read config → create AppState (maybe with embedder) → build router (create_router).
  2. Request hits Axum router (src/server/routes.rs) → handler in src/server/handlers/*.
  3. Handler: validate input (src/validation.rs) → AppState::get_or_create_collection (loads/creates Collection with mmap, index, WAL) → perform op:
      - Inserts/upserts: log to WAL, serialize doc to mmap, update in-memory offset index, update vector index, update metadata, track latency.
      - Search: gather vectors, ask vector index for neighbors, score via metric, return DTOs.
      - Embedding endpoints: call embedder (with retry/cache), then reuse insert/search paths.
  4. Periodic checkpoints/flush on shutdown or WAL thresholds persist state.

  Folder Cheat Sheet

  - src/server/ – HTTP API wiring, state, DTOs.
  - src/storage/ – on-disk layout, WAL, mmap, collection CRUD/search.
  - src/index/ – vector index interface + Flat/HNSW/IVF implementations.
  - src/search/ – search helpers and filters atop collection/index.
  - src/embeddings/ – provider-agnostic embedding layer.
  - src/metrics/ – distance metrics + latency tracking.
  - src/config/ – knobs for execution mode, mmap, WAL, index selection, etc.
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
  - Embed + Store: POST /embed → call embedder (OpenAI/Ollama via RetryEmbedder/cache) to get vector → then go through insert path → return id+embedding+tokens.
  - Text Search: Embed query text via embedder → run normal vector search → return hits.
  - Collections CRUD: Create just ensures collection exists (touches disk/index metadata); list enumerates loaded collections with counts; delete removes files/entries.
  - Metrics/Health: Read-only endpoints summarize counts, index type, memory usage, latency stats, and embedder status.
  - Shutdown: Ctrl+C → set shutting_down flag (new requests rejected) → checkpoint all collections (flush mmap/index/vector-index/metadata) → graceful drain.

