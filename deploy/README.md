# Deploy

| File | Purpose |
|---|---|
| `docker/piramid.Dockerfile` | CPU image. cargo-chef caches dependency builds in their own layer. |
| `docker/piramid-cuda.Dockerfile` | CUDA image, built with `--features gpu-cuda`. Needs `--gpus all`. |
| `compose.yml` | Dev stack, builds from source. |
| `compose.prod.yml` | Overlay that swaps in GHCR images. |

Commands here are plain `docker compose`, since deploying does not assume a repo checkout or any
of the contributor tooling. From inside a checkout, `just up`, `just down`, `just logs`,
`just prod-up`, and `just prod-down` are shorthands for the same things.

## Running a published image

Nothing to check out:

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

Images are published by `.github/workflows/cd.yml` on every push to `main`, tagged with both the
commit SHA and `main`. Pin to a SHA rather than `main` for anything you care about.

## Compose

From a checkout, building from source:

```bash
docker compose -f deploy/compose.yml up -d
docker compose -f deploy/compose.yml logs -f
docker compose -f deploy/compose.yml --profile ollama up -d    # add local embeddings
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
requires it.

## Notes

The server runs as uid 10001 rather than root, and data lives on the `piramid-data` volume at
`/data`. Health is `GET /api/health` and readiness is `GET /api/readyz`, both served without a
key so container and load balancer probes work. `GET /metrics` is the Prometheus endpoint and
requires the key; give the scrape job `authorization: {credentials: <key>}`.

Rate limiting keys on the peer address. Behind a reverse proxy every client shares the proxy's
bucket, so raise `startup.http.rate_limit` or set it to null there.

The CUDA image sets `PIRAMID__RUNTIME__EXECUTION=gpu`. With no device present the server fails
to start, rather than quietly serving CPU results under a GPU configuration.
