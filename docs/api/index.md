# API (outline)

Document the HTTP surface here. Keep examples minimal and link to full reference when ready.

## Collections
- `POST /api/collections` create
- `GET /api/collections/{collection}` describe
- `DELETE /api/collections/{collection}` drop

## Vectors
- `POST /api/collections/{collection}/vectors` insert/upsert (single or batch)
- `DELETE /api/collections/{collection}/vectors/{id}` delete

## Search
- `POST /api/collections/{collection}/search` vector search (k, metric, filters, overrides)
- Optional params: ef, nprobe, filter_overfetch, execution mode, presets.

## Embeddings
- `POST /api/embed` (or collection-specific) for single/batch text.
- Providers: OpenAI, local HTTP. Configure via `AppConfig.embedding`.

## Health and metrics
- `/healthz` liveness
- `/api/metrics` operational metrics (latency, locks, limits, cache usage)
- Add `/readyz` if you separate readiness checks.

## Admin / index
- Rebuild or warmup endpoints (if enabled).
- WAL/persistence controls (if exposed).

> Add request/response schemas, error shapes, and curl examples as you finalize the API surface.
