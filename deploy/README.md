# Deploy

| File | Purpose |
|---|---|
| `docker/piramid.Dockerfile` | CPU image, built with no features. cargo-chef caches dependency builds in their own layer. |
| `docker/piramid-cuda.Dockerfile` | CUDA image, built with `--features gpu-cuda`. Needs `--gpus all`. |
| `compose.yml` | Dev stack, builds from source. |
| `compose.prod.yml` | Overlay that swaps in GHCR images. |

Commands here are plain `docker compose`, since deploying does not assume a repo checkout or any
of the contributor tooling. From inside a checkout, `just up`, `just down`, `just logs`,
`just prod-up`, and `just prod-down` are shorthands for the same things.

Piramid is used through `piramid serve`, which the images run by default. The steps below start
with a published image, which stores documents and searches them, and then build an image that
also loads a model and answers questions with retrieval.

## Running a published image

Nothing to check out. This serves collections and search, with no model:

```bash
docker run -p 6333:6333 -v piramid-data:/data -e PIRAMID_API_KEY=<key> \
  ghcr.io/ashworks1706/piramid:main
```

The image binds `0.0.0.0:6333`, so it refuses to start without `PIRAMID_API_KEY`; the error in
`docker logs` says so. Generate a key with `openssl rand -hex 32` and send it as
`Authorization: Bearer <key>`:

```bash
curl -H "Authorization: Bearer $PIRAMID_API_KEY" http://localhost:6333/api/collections
```

Stop with `docker stop -t 45`, which leaves time for in-flight requests to drain (up to
`startup.http.drain_timeout_secs`, 30 by default) and for every open collection to checkpoint
before Docker kills the process.

Images are published by `.github/workflows/cd.yml` on every push to `main`, as
`ghcr.io/ashworks1706/piramid` and `ghcr.io/ashworks1706/piramid-cuda`, each tagged `main` and
`sha-<short commit>`. Pin to a `sha-` tag rather than `main` for anything you care about.

## Serving a model

The published images are built without the `inference-candle` feature. They serve collections and
search, and a configuration with `runtime.inference.enabled: true` is refused at startup. To serve
generation from a container, build the CUDA image with the feature. In
`docker/piramid-cuda.Dockerfile`, change the build line to
`cargo build --release --locked --bin piramid --features gpu-cuda,inference-candle`, then build from
the repository root:

```bash
docker build -f deploy/docker/piramid-cuda.Dockerfile -t piramid-cuda-inference .
```

Put the model checkpoint in a directory on the host, for example
`models/Qwen2.5-0.5B-Instruct` holding `config.json`, `tokenizer.json`, `tokenizer_config.json`
and the `.safetensors` weights. Then write `piramid.yaml` beside it. The CUDA image already sets
`startup.hardware.profile: gpu` and `runtime.execution: gpu`, so the file only needs the model and
an embedding provider:

```yaml
startup:
  embedding:
    provider: openai
    model: text-embedding-3-small

runtime:
  inference:
    enabled: true
    model_path: /models/Qwen2.5-0.5B-Instruct
```

Start the container with the model and the file mounted read-only, and `CONFIG_FILE` naming the
file:

```bash
docker run --gpus all -p 6333:6333 \
  -v piramid-data:/data \
  -v "$PWD/models:/models:ro" \
  -v "$PWD/piramid.yaml:/config/piramid.yaml:ro" \
  -e CONFIG_FILE=/config/piramid.yaml \
  -e PIRAMID_API_KEY -e OPENAI_API_KEY \
  piramid-cuda-inference
```

The server runs as uid 10001, so the mounted files must be readable by that user. An `-e` flag
with a name and no value passes that variable from your shell. Once `GET /api/model` answers,
store documents and ask a question that retrieves from them:

```bash
curl -X POST http://localhost:6333/api/collections/docs/embed \
  -H "Authorization: Bearer $PIRAMID_API_KEY" -H "Content-Type: application/json" \
  -d '{"texts": ["Compaction rewrites the record store without deleted documents."]}'

curl -X POST http://localhost:6333/api/generate \
  -H "Authorization: Bearer $PIRAMID_API_KEY" -H "Content-Type: application/json" \
  -d '{"messages": [{"role": "user", "content": "What does compaction do?"}],
       "retrieval": {"collection": "docs", "k": 1}}'
```

The response carries the answer in `text` and the passages it used in `retrieval`. The
OpenAI-compatible endpoints are served at `/v1/chat/completions` and `/v1/models` on the same port,
with the key sent as the client's API key. They do not retrieve; use `/api/generate` with
`retrieval` when the answer should draw on a collection. The
[README quickstart](../README.md#quickstart) walks through the same requests in more detail,
including metadata, streaming and an OpenAI client.

## Compose

From a checkout, building from source:

```bash
docker compose -f deploy/compose.yml up -d
docker compose -f deploy/compose.yml logs -f
docker compose -f deploy/compose.yml --profile ollama up -d    # add an Ollama embedding server
docker compose -f deploy/compose.yml down
```

With published images instead of a local build:

```bash
PIRAMID_IMAGE_TAG=main docker compose \
  -f deploy/compose.yml -f deploy/compose.prod.yml pull
PIRAMID_IMAGE_TAG=main docker compose \
  -f deploy/compose.yml -f deploy/compose.prod.yml up -d
```

## Configuration

Both compose files read `../.env` if it exists. See `.env.example` for every variable. Secrets like
`PIRAMID_API_KEY` and `OPENAI_API_KEY` belong in `.env`, never in a compose file or an image.
`PIRAMID_API_KEY` is required: without it the container exits at startup and restarts.

The console reads the same variable, so `PIRAMID_API_KEY=<key> piramid` watches a server that
requires it. Set `console.base_url` in its configuration file, or
`PIRAMID__CONSOLE__BASE_URL=http://host:6333`, when the server is not on the address `startup.bind`
names.

When something goes wrong, `piramid support-bundle` writes a diagnostic report from the binary,
the configuration and the data directory, with secrets redacted. It reads collection manifests
without opening a collection or writing to the data directory, so run it beside the server with
the same configuration and data volume the server uses. Read the file and attach it to the bug
report.

## Notes

The server runs as uid 10001 rather than root, and data lives on the `piramid-data` volume at
`/data`. Health is `GET /api/health` and readiness is `GET /api/readyz`, both served without a
key so container and load balancer probes work. `GET /metrics` is the Prometheus endpoint and
requires the key; give the scrape job `authorization: {credentials: <key>}`.

Rate limiting keys on the peer address. Behind a reverse proxy every client shares the proxy's
bucket, so raise `startup.http.rate_limit` or set it to null there.

The CUDA image sets `PIRAMID__RUNTIME__EXECUTION=gpu`. With no device present the server fails
to start, rather than quietly serving CPU results under a GPU configuration.
