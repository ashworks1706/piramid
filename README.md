<img width="1114" height="191" alt="Piramid Logo" src="https://github.com/user-attachments/assets/efaa4c47-62d1-4397-9899-8bd58d400fc6" />

<p align="center">
    <b>Vector Database for Agentic Applications</b>
</p>

<p align="center">
    <a href="https://crates.io/crates/piramid"><img src="https://img.shields.io/crates/v/piramid.svg" alt="crates.io"></a>
    <a href="https://www.npmjs.com/package/piramid"><img src="https://img.shields.io/npm/v/piramid.svg" alt="npm"></a>
    <a href="https://pypi.org/project/piramid/"><img src="https://img.shields.io/pypi/v/piramid.svg" alt="PyPI"></a>
</p>

<p align="center">
    <a href="#quick-start">Quick Start</a> •
    <a href="#usage">Usage</a> •
    <a href="#contributing">Contributing</a>
</p>

 


Piramid is a vector database built in Rust, designed specifically for AI agent applications. Store embeddings, search by similarity, and let agents discover and walk your data.

- REST API + Rust library
- Memory-safe, zero-cost abstractions
- OpenAI, Ollama (local) support
- Purpose-built with MCPs for LLM applications
- Visual interface for management
- GPU Acceleration for faster search
- Semantic Caching on LLM API costs
- WASM support for running in browser/edge/mobile

 

## Quick Start

### Docker

```bash
git clone https://github.com/ashworks1706/piramid
cd piramid
docker compose up -d
```

Access dashboard at `http://localhost:6333`

### From Source

```bash
# Prerequisites: Rust 1.70+
cargo build --release
./target/release/piramid-server
```

 

## Usage

### REST API

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

# Search
curl -X POST http://localhost:6333/api/collections/docs/search \
  -H "Content-Type: application/json" \
  -d '{"vector": [0.1, 0.2, 0.3, 0.4], "k": 5}'
```
 

## Configuration

Configure via environment variables:

```bash
PORT=6333              # HTTP server port
DATA_DIR=/app/data     # Data storage directory

# Optional: Embedding provider
EMBEDDING_PROVIDER=openai|ollama
EMBEDDING_MODEL=text-embedding-3-small
OPENAI_API_KEY=sk-...
EMBEDDING_BASE_URL=http://localhost:11434
```

```bash
# Configure embedding provider
export EMBEDDING_PROVIDER=openai
export EMBEDDING_MODEL=text-embedding-3-small
export OPENAI_API_KEY=sk-your-key-here

# Or use local Ollama
export EMBEDDING_PROVIDER=ollama
export EMBEDDING_MODEL=nomic-embed-text
export EMBEDDING_BASE_URL=http://localhost:11434

# Embed text and store
curl -X POST http://localhost:6333/api/collections/docs/embed \
  -H "Content-Type: application/json" \
  -d '{"text": "The quick brown fox", "metadata": {"source": "example"}}'

# Search by text
curl -X POST http://localhost:6333/api/collections/docs/search/text \
  -H "Content-Type: application/json" \
  -d '{"query": "fast animals", "k": 5}'
```


 

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Run server
cargo run --bin piramid-server

# Dashboard (development)
cd dashboard && npm install && npm run dev
```

## Contributing

See [docs/TODO.md](docs/TODO.md) for documentation.


## License

[Apache 2.0 License](LICENSE)

## Acknowledgments

built by @ashworks1706 for educational purposes

