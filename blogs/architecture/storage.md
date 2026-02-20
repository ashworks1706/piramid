# Storage

TODO cover:
- On-disk layout per collection: data file, index, vector index, metadata, WAL, checkpoints, sidecars.
- Mmap vs. file IO fallback; initial sizing and growth strategy.
- WAL: what is logged, sequence handling, replay order, checkpoint semantics.
- Checkpoint and compaction flows; when caches rebuild.
- Caches: vector cache, metadata cache; invalidation rules.
- Disk/memory guards and read-only mode behavior.
