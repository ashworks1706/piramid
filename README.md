<img width="1114" height="191" alt="Piramid Logo" src="https://github.com/user-attachments/assets/efaa4c47-62d1-4397-9899-8bd58d400fc6" />

<p align="center">
    <b>Vector Database for Agentic Applications</b>
</p>

<p align="center">
    <a href="https://crates.io/crates/piramid"><img src="https://img.shields.io/crates/v/piramid.svg" alt="crates.io"></a>
    &nbsp;·&nbsp;
    <a href="https://piramiddb.com">Website</a>
</p>

<p align="center">
    <a href="#quick-start">Quick Start</a> •
    <a href="#usage">Usage</a> •
    <a href="#configuration">Configuration</a> •
    <a href="#development">Development</a> •
    <a href="docs/contributing/index.md">Contributing</a>
</p>

Piramid is a Rust vector database tuned for low-latency agentic workloads. The long-term goal is to colocate vector search and the LLM on the same GPU (future Zipy kernel) to avoid CPU round-trips. Today it is a lean CPU server with fast search, WAL durability, embedding providers, and guardrails for production use.

- Single binary (`piramid`) with CLI + server
- Search engines: HNSW, IVF, flat; filters and metadata
- WAL + checkpoints; mmap-backed storage with caches
- Embeddings: OpenAI and local HTTP (Ollama/TEI-style), caching and retries
- Limits and disk/memory guards; tracing + metrics/health endpoints
- Roadmap: GPU kernel (Zipy) co-resident with the LLM for single-hop latency

## Quick Start

### Cargo (recommended)

```bash
cargo install piramid
piramid init                # writes piramid.yaml
piramid serve --data-dir ./data
```

Server defaults to `http://0.0.0.0:6333`.
Data is stored under `~/.piramid` by default (set `DATA_DIR` to override).

### From source

```bash
git clone https://github.com/ashworks1706/piramid
cd piramid
cargo run --release -- serve --data-dir ./data
```

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

## Configuration

Use a config file (`piramid.yaml`) and override with env vars.

```bash
piramid init --path piramid.yaml   # generate defaults
piramid serve --config piramid.yaml
```

Environment overrides (examples):

- Inline: `PORT=7000 DATA_DIR=~/piramid-data piramid serve`
- Point to a config file: `CONFIG_FILE=~/piramid/piramid.yaml piramid serve`
- Embeddings: `EMBEDDING_PROVIDER=openai OPENAI_API_KEY=sk-...`

Key env overrides:

```bash
PORT=6333                 # HTTP server port
DATA_DIR=/app/data        # Data storage directory
CONFIG_FILE=./piramid.yaml

# Embeddings
EMBEDDING_PROVIDER=openai|local
EMBEDDING_MODEL=text-embedding-3-small
OPENAI_API_KEY=sk-...
EMBEDDING_BASE_URL=http://localhost:11434   # for local/Ollama/TEI
EMBEDDING_TIMEOUT_SECS=15

# Limits/guards
DISK_MIN_FREE_BYTES=1073741824    # 1GB
DISK_READONLY_ON_LOW_SPACE=true
CACHE_MAX_BYTES=536870912         # 512MB
```

Minimal YAML sample:

```yaml
index:
  type: Auto
  metric: Cosine
  mode: Auto
search:
  ef: null
  nprobe: null
  filter_overfetch: 10
wal:
  enabled: true
  checkpoint_frequency: 1000
memory:
  use_mmap: true
limits:
  max_vectors: null
  max_bytes: null
```

## Development

```bash
cargo build
cargo test
cargo run -- serve --data-dir ./data
```

## License

[Apache 2.0 License](LICENSE)

## Acknowledgments

Built by @ashworks1706. Future work includes Zipy (GPU kernel) for co-located LLM + vector search. 
