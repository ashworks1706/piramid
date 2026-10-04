# Usage

Running `piramid serve`, configuring it, and the HTTP API. [`config.example.yaml`](../config.example.yaml)
lists every setting at its default.

## Builds

Model execution is behind two Cargo features, both off by default. `inference-candle` runs the
model; `gpu-cuda` runs it, and search, on a CUDA device. A build without them serves collections and
search but refuses `runtime.inference.enabled: true`.

```bash
cargo install --path apps/cli --locked --features inference-candle,gpu-cuda   # GPU, needs a CUDA toolkit
cargo install --path apps/cli --locked --features inference-candle            # CPU
```

Piramid runs Qwen2 and Qwen3 checkpoints (Qwen2.5 included). The model directory holds
`config.json`, `tokenizer.json`, `tokenizer_config.json` and the `.safetensors` weights.

## Configuration

Settings resolve in this order, later winning:

1. defaults in `apps/engine/core/src/config`
2. a YAML or JSON file, from `piramid serve --config` or `CONFIG_FILE`
3. `PIRAMID__` environment variables, spelled from the path: `runtime.wal.max_log_size` is
   `PIRAMID__RUNTIME__WAL__MAX_LOG_SIZE`
4. `--port` and `--data-dir`

`PIRAMID_API_KEY` and `OPENAI_API_KEY` are environment-only. An unknown, misspelled or
unimplemented key fails startup with a message naming it.

The file has three blocks. `startup:` applies once at boot. `runtime:` is re-read by
`POST /api/config/reload`, except that `inference` is fixed at startup, and `quantization`,
`memory`, `wal.enabled` and `wal.sync_on_write` cannot change while a collection is open.
`search.metric` is copied into a collection when it is created. `console:` configures the console.

On a GPU, set `startup.hardware.profile: gpu` and `runtime.execution: gpu`; the model loads onto
`cuda:N` at `startup.hardware.gpu.device_ordinal`. On the CPU, leave both out and set a KV cache
budget:

```yaml
runtime:
  inference:
    enabled: true
    model_path: ./models/Qwen2.5-0.5B-Instruct
    kv_cache:
      max_bytes: 2147483648
```

## Embedding providers

`startup.embedding` turns text into vectors for `/embed`, `/search/text` and retrieval in
`/api/generate`. A collection's dimension is fixed by the model that first fills it.

| `provider` | What it talks to |
|---|---|
| `openai` | OpenAI with `OPENAI_API_KEY`, or any server speaking the format (TEI, vLLM, llama.cpp, llama-swap) when `base_url` is the full embeddings endpoint |
| `ollama` | an Ollama server; `just up ollama` starts one beside the server |
| `piramid` | a Qwen3 embedding checkpoint run in-process; `model` is the directory, `options` takes `device` (`cpu` or `cuda:N`), `dtype` and `max_tokens` |

```yaml
startup:
  embedding:
    provider: openai
    model: nomic-embed-text
    base_url: http://127.0.0.1:8100/v1/embeddings
```

## HTTP API

The server listens on `127.0.0.1:6333` and stores collections under `./data`.

### Collections

```bash
# Embed texts with the configured provider and store them; creates the collection
curl -X POST localhost:6333/api/collections/docs/embed -H 'Content-Type: application/json' \
  -d '{"texts": ["first", "second"], "metadata": [{"topic": "a"}, {"topic": "b"}]}'

# Store vectors you computed yourself
curl -X POST localhost:6333/api/collections/notes/vectors -H 'Content-Type: application/json' \
  -d '{"vectors": [[0.1, 0.2, 0.3, 0.4]], "texts": ["hello"], "metadata": [{"kind": "greeting"}]}'

# Search by text, or by vectors (one result list per query vector)
curl -X POST localhost:6333/api/collections/docs/search/text -H 'Content-Type: application/json' \
  -d '{"query": "crash safety", "k": 5, "filter": {"topic": {"eq": "a"}}}'
curl -X POST localhost:6333/api/collections/notes/search -H 'Content-Type: application/json' \
  -d '{"vectors": [[0.1, 0.2, 0.3, 0.4]], "k": 5}'
```

Lists are positional. Search is an exact scan; a filter maps a metadata field to `eq`, `ne`, `gt`,
`gte`, `lt`, `lte` or `in`.

### Generation

```bash
curl -X POST localhost:6333/api/generate -H 'Content-Type: application/json' \
  -d '{"messages": [{"role": "user", "content": "What happens to deleted documents?"}],
       "retrieval": {"collection": "docs", "k": 2}, "max_new_tokens": 128}'
```

With `retrieval`, the server embeds the last user message, adds the `k` closest documents to the
system message and generates. The response carries `text`, `finish_reason`, `usage` (token counts,
time to first token, total time) and `retrieval` (the passages and embed and search times).
`prompt` instead of `messages` skips the chat template. `"stream": true` returns server-sent events:
`retrieval`, one `token` per token, then `done`, or `error`.

`/v1/chat/completions` and `/v1/models` follow the OpenAI format and do not retrieve. The model id
is the model directory name unless `runtime.inference.model_name` sets it. Unsupported fields are
refused, not ignored.

### Operations

`GET /api/model`, `/api/health`, `/api/readyz`, `/api/version`, `/api/metrics` (JSON) and
`/metrics` (Prometheus).

## Serving on a network

The default bind serves without a key. Any other bind needs `PIRAMID_API_KEY`, after which every
route except `/api/health` and `/api/readyz` requires `Authorization: Bearer <key>`;
`startup.http.auth.allow_unauthenticated: true` opens a port on purpose. Each client IP gets a token
bucket (`startup.http.rate_limit`, 100 per second, burst 200). On SIGINT or SIGTERM the server
drains for up to `startup.http.drain_timeout_secs`, checkpoints every open collection and exits.
[SECURITY.md](../SECURITY.md) has the threat model; [deploy/README.md](../deploy/README.md) covers
containers.

## Console

`piramid` with no subcommand opens a terminal UI over a running server. `1 collections` shows each
collection's dimension, memory, latencies, last checkpoint and WAL size; `c` compacts the selected
one. `2 config` shows the resolved configuration, `3 device` graphs the host, each GPU, the device
memory budget and generation. `?` lists every key. Settings live under `console:`, and
`console.base_url` follows `startup.bind` by default.

`piramid support-bundle` writes diagnostics for a bug report, with credentials redacted.
