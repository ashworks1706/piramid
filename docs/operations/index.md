# Operations (outline)

Guidance for running Piramid in development and production.

## Running
- `piramid serve --data-dir ./data` (or systemd/docker).
- Health checks: `/healthz`; readiness if added.
- Metrics: `/api/metrics` (locks, latency, cache sizes, limits).

## Warmup
- Background warmup of mmap/index files on collection load (if enabled).
- Describe how to trigger manual warmup or rebuild when available.

## Backups & recovery
- Checkpoint cadence; WAL replay on startup.
- How to take a consistent snapshot (pause writes vs. rely on checkpoints).

## Maintenance
- Rebuild/compact indexes, remove tombstoned vectors, clean sidecar files.
- Disk space safeguards and read-only mode thresholds.
- Cache eviction/limits and how to tune them.

## Troubleshooting
- Enable verbose tracing logs.
- Common errors (limits exceeded, disk full, mmap disabled).
- Where logs/metrics surface in your stack.
