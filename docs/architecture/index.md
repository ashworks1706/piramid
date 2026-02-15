# Architecture (outline)

Use this as a living doc to explain how Piramid is put together. Fill in details as you iterate.

## Server
- Axum HTTP server, single binary `piramid`.
- Shared state holds `AppConfig`, collection registry, caches, metrics.
- Health: `/healthz`, metrics: `/api/metrics`.

## Storage
- Data files stored per collection: vectors, metadata, indexes, WAL, checkpoints.
- Memory-mapped vectors (when enabled) to keep hot data in RAM with low copy overhead.
- WAL + checkpoints for durability and crash recovery.

## Indexes
- Flat, IVF, HNSW. Per-request overrides for ef/nprobe/filter_overfetch.
- Filter-aware search path when metadata predicates are present.
- Warmup: optional background touch to fault pages into memory.

## Caching
- Cached vector map and metadata map to avoid rebuild per query.
- Invalidation on writes/checkpoints.

## Embeddings
- Providers: OpenAI and local HTTP (Ollama/TEI style).
- Unified embed endpoint for single/batch payloads.
- Retries and simple caching.

## Guardrails
- Limits per collection (vectors, bytes, vector size) and disk low-space read-only mode.
- Cache caps to prevent runaway memory.
- Tracing + structured logs for lock/search timings.
