# piramid-serving

HTTP transport, use-case services, and shared process state: everything `piramid serve` runs.

## Endpoints

Collection and search routes live under `/api/collections/{collection}`: `vectors` to insert,
list and delete documents, `vectors/{id}` to get or delete one, `upsert`, `embed` to embed texts
with the configured provider and store them, `search` for vector queries, `search/text` for a text
query, `count`, and `compact`.

Generation is served when a model is loaded:

| Route | What it does |
|---|---|
| `POST /api/generate` | Generate from a `prompt` or `messages`. With `retrieval`, first embeds the query, searches the named collection, and places the passages in the system message, or before a raw prompt. Returns one JSON body, or server-sent events with `"stream": true`. |
| `GET /api/model` | The loaded model: name, architecture, device, maximum sequence length. |
| `POST /v1/chat/completions` | OpenAI-compatible chat completion, streamed or not. It does not retrieve. |
| `GET /v1/models` | The loaded model in OpenAI form. |

Operational routes are `/api/health`, `/api/readyz`, `/api/version`, `/api/metrics`,
`/api/health/embeddings`, `/api/config`, `POST /api/config/reload`, and `/metrics` for
Prometheus.

## Three layers

The handlers, the services and the database crate below can look like three copies of the same
thing. They do different jobs:

| Layer | Owns | Example, for one insert |
|---|---|---|
| `http/handlers` | axum extraction only | pull `State`, `Path` and `Json` out of the request |
| `services` | everything true because a server exists | shutdown check, read-only check, name validation, request shape to `Document`, take the write lock, record lock wait and latency, build the response |
| `piramid-database` | durability and search | WAL, record store, offsets, resident vectors and metadata, exact search |

So `services` is the request-scoped layer: locks, metrics, admission, and the API shapes. Without
it, those would move either into the handlers, which would then need `AppState`, locks and metrics,
or into the database crate, which would then know about HTTP and shutdown.

`state.rs` holds `AppState`, the composition root: the collection, embedding and inference
managers, the GPU device, the config, and the flags every request reads. `disk.rs` watches free
space and switches to read-only mode. `machine.rs` samples the host and GPU readings on a background thread.

`http::ApiError` is where a transport-agnostic `ErrorKind` becomes an HTTP status. That is also what
keeps `piramid-core` free of axum.

Part of [Piramid](https://github.com/ashworks1706/piramid). See
[`docs/ARCHITECTURE.md`](../../../docs/ARCHITECTURE.md) for how the crates fit together.
