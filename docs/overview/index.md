# Piramid documentation (skeleton)

Piramid is a latency-first vector database written in Rust. The goal is to keep the database and your LLM on the same device, minimize round-trips, and expose a simple HTTP API plus a single binary CLI.

## What you get
- Axum server with health and metrics endpoints.
- Storage backed by mmap + WAL/checkpoints, with HNSW/IVF/Flat indexes.
- Unified embed/search/insert routes (single or batch) with filters and metadata cache.
- Guardrails: limits, disk low-space read-only mode, cache caps, tracing.

## Quick start
```bash
cargo install piramid
piramid init --path piramid.yaml
piramid serve --data-dir ./data
```

## Next steps
- Architecture: see how storage, indexes, and caches fit together.
- API: HTTP surface area for vectors, embeds, search, and metrics.
- Configuration: tweak index/search/memory/WAL limits.
- Operations: running, metrics, warmup, backups, and troubleshooting.
