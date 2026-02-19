# Health & Metrics

Piramid exposes four endpoints for monitoring server health, readiness, and operational metrics.

---

## `GET /api/health`

Liveness probe. Returns immediately with a static `200 OK` — no I/O or lock acquisition.

**Response**
```json
{
  "status": "ok",
  "version": "0.1.1"
}
```

**Use:** Load balancer health checks, uptime monitors.

---

## `GET /api/health/embeddings`

Checks whether an embedding provider is configured and available.

**Response**
- `200 OK` — embedder is configured
- `503 Service Unavailable` — no embedder configured (server started without `EMBEDDING_PROVIDER`)

**Use:** Gate application flows that depend on text embedding (e.g. `/embed`, `/search/text`).

---

## `GET /api/readyz`

Full readiness and integrity snapshot. Inspects every collection on disk, reports disk usage, and confirms the server is not shutting down.

Returns `503` if the server is in the process of shutting down.

**Response**
```json
{
  "ok": true,
  "version": "0.1.1",
  "data_dir": "/app/data",
  "total_collections": 2,
  "loaded_collections": 2,
  "total_vectors": 4200,
  "disk_total_bytes": 107374182400,
  "disk_available_bytes": 53687091200,
  "collections": [
    {
      "name": "docs",
      "loaded": true,
      "count": 4200,
      "index_type": "HNSW",
      "last_checkpoint": 1718000000,
      "checkpoint_age_secs": 42,
      "wal_size_bytes": 16384,
      "schema_version": 1,
      "integrity_ok": true,
      "error": null
    }
  ]
}
```

`ok` is `true` only when every collection is loaded and `integrity_ok`. Collections discovered on disk but not yet loaded appear with `"loaded": false` and `"error": "not loaded"`.

**Use:** Kubernetes readiness probe, pre-deploy smoke tests, human inspection.

---

## `GET /api/metrics`

Operational metrics for all collections, WAL state, and the embedding provider.

**Response**
```json
{
  "total_collections": 2,
  "total_vectors": 4200,
  "collections": [
    {
      "name": "docs",
      "vector_count": 4200,
      "index_type": "HNSW",
      "memory_usage_bytes": 33554432,
      "insert_latency_ms": 1.2,
      "search_latency_ms": 0.8,
      "lock_read_ms": 0.05,
      "lock_write_ms": 0.1,
      "search_overfetch": 10,
      "hnsw_ef_search": 100,
      "ivf_nprobe": null
    }
  ],
  "wal_stats": [
    {
      "collection": "/app/data/docs",
      "last_checkpoint": 1718000000,
      "checkpoint_age_secs": 42,
      "wal_size_bytes": 16384
    }
  ],
  "embedding": {
    "requests": 150,
    "texts": 620,
    "total_tokens": 84000,
    "avg_latency_ms": 95.4
  },
  "app_config": { }
}
```

**Field reference**

| Field | Description |
|---|---|
| `insert_latency_ms` | Rolling average insert time per vector (ms) |
| `search_latency_ms` | Rolling average search time (ms) |
| `lock_read_ms` | Rolling average read-lock acquisition time (ms) |
| `lock_write_ms` | Rolling average write-lock acquisition time (ms) |
| `search_overfetch` | Candidate multiplier used during filtered search |
| `hnsw_ef_search` | HNSW beam width at query time (`null` for non-HNSW indexes) |
| `ivf_nprobe` | IVF probe count (`null` for non-IVF indexes) |
| `wal_size_bytes` | Current size of the WAL file on disk |
| `checkpoint_age_secs` | Seconds since the last WAL checkpoint |
| `embedding.avg_latency_ms` | Average round-trip to the embedding provider (ms) |

**Use:** Prometheus scrape (curl + push gateway), Grafana dashboards, alerting on slow queries or lock contention.

```bash
# Example: scrape metrics with curl
curl http://localhost:6333/api/metrics | jq '.collections[].search_latency_ms'
```
