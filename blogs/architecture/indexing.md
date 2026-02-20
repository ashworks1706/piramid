# Indexing

TODO cover:
- Index types (Flat, IVF, HNSW, Auto) and when each is chosen.
- Search parameters: ef, nprobe, filter_overfetch; filter-aware path vs. post-filter.
- Insert/update/remove paths and how vector index stays in sync with disk index.
- Rebuild flow (background job + status endpoint); compaction; duplicate detection.
- Tombstoning strategy (current or planned) and impact on graph connectivity.
- Product quantization or other compression (if/when added).
