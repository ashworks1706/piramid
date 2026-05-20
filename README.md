### imp updates
- heavily looking for collaborators 
---
<img width="1114" height="191" alt="Piramid Logo" src="https://github.com/user-attachments/assets/efaa4c47-62d1-4397-9899-8bd58d400fc6" />

<p align="center">
    <b>Inference Engine for Retrieval Augmented Systems</b>
</p>

<p align="center">
    <a href="https://crates.io/crates/piramid"><img src="https://img.shields.io/crates/v/piramid.svg" alt="crates.io"></a>
</p>

<p align="center">
  <a href="#overview">Overview</a> •
  <a href="#usage">Usage</a> •
  <a href="docs/setup.md">Setup</a> •
  <a href="https://piramiddb.com/blogs/contributions">Contributing</a>
</p>

## Overview

Piramid is a combination of vector database and transformer inference tuned for low-latency agentic workloads written in Rust. Inspired from google deepmind's RETRO project, Piramid is meant to convert traditional RAG applications involving separate LLM and Database connections into one single hosted binary to serve and fuse transformer's attention with database queries.

- Single binary (`piramid`) with CLI + server
- Search engines: HNSW, IVF, flat; filters and metadata
- WAL + checkpoints; mmap-backed storage with caches
- Embeddings: OpenAI and local HTTP (Ollama/TEI-style), caching and retries
- Limits and disk/memory guards; tracing + metrics/health endpoints

https://github.com/user-attachments/assets/487cbc0f-c279-4a15-a160-9acd4666fbe6


## Get Started

For full setup on Linux, macOS, WSL2, Docker, the website, and the SDKs, see [docs/setup.md](docs/setup.md).

If you already have the binary installed, start the server with:

```bash
piramid serve --data-dir ./data
```

Server defaults to `http://0.0.0.0:6333`.
Data is stored under `~/.piramid` by default; set `DATA_DIR` to override it.

## Usage

### REST API (v1)

```bash
# Create collection
curl -X POST http://localhost:6333/api/collections \
  -H "Content-Type: application/json" \
  -d '{"name": "docs"}'

# Store vector
curl -X POST http://localhost:6333/api/collections/docs/vectors \
  -H "Content-Type: application/json" \
  -d '{
    "vector": [0.1, 0.2, 0.3, 0.4],
    "text": "Hello world",
    "metadata": {"category": "greeting"}
  }'

# Embed text (single or batch) and store
curl -X POST http://localhost:6333/api/collections/docs/embed \
  -H "Content-Type: application/json" \
  -d '{"text": ["hello", "bonjour"], "metadata": [{"lang": "en"}, {"lang": "fr"}]}'

# Search
curl -X POST http://localhost:6333/api/collections/docs/search \
  -H "Content-Type: application/json" \
  -d '{"vector": [0.1, 0.2, 0.3, 0.4], "k": 5}'
```

Health and metrics: `/healthz`, `/readyz`, `/api/metrics`.

## License

[Apache 2.0 License](LICENSE)

## Acknowledgments

Built by @ashworks1706.
