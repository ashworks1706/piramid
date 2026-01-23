<img width="1114" height="191" alt="Piramid Logo" src="https://github.com/user-attachments/assets/efaa4c47-62d1-4397-9899-8bd58d400fc6" />

<p align="center">
    <b>Vector Database for Agentic Applications</b>
</p>

<p align="center">
    <a href="#quick-start">Quick Start</a> •
    <a href="#features">Features</a> •
    <a href="#usage">Usage</a> •
    <a href="#contributing">Contributing</a>
</p>

---

## What is Piramid?

Piramid is a vector database built in Rust, designed specifically for AI agent applications. Store embeddings, search by similarity, and let agents discover and walk your data.

### Why Piramid?

- **🦀 Rust Performance** - Memory-safe, zero-cost abstractions
- **🔌 Built-in Embeddings** - OpenAI, Ollama (local) support
- **🎯 Agent-First Design** - Purpose-built for LLM applications
- **🚀 Simple API** - REST API + Rust library
- **📊 Web Dashboard** - Visual interface for management

### Upcoming Features

- **GPU Acceleration** - 10-100x faster search (Phase 11)
- **Semantic Cache** - Save 70%+ on LLM API costs (Phase 13)
- **WASM Support** - Run in browser/edge/mobile (Phase 14)
- **Agent Memory** - Purpose-built memory system (Phase 15)

---

## Quick Start

### Docker (Recommended)

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

---

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

### Rust Library

```rust
use piramid::{VectorEntry, VectorStorage, SimilarityMetric};

fn main() -> piramid::Result<()> {
    let mut storage = VectorStorage::open("vectors.db")?;
    
    let entry = VectorEntry::new(
        vec![0.1, 0.2, 0.3, 0.4],
        "Hello world".to_string()
    );
    storage.store(entry)?;
    
    let results = storage.search(
        &[0.1, 0.2, 0.3, 0.4],
        5,
        SimilarityMetric::Cosine
    );
    
    for result in results {
        println!("{}: {}", result.text, result.score);
    }
    
    Ok(())
}
```

### With Embeddings

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

---

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

---

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Run example
cargo run --example basic

# Run server
cargo run --bin piramid-server

# Dashboard (development)
cd dashboard && npm install && npm run dev
```

---

## Contributing

We welcome contributions! See [docs/TODO.md](docs/TODO.md) for documentation.

---


## License

Soon

---

## Acknowledgments

Built with ❤️ in Rust for better AI agent infrastructure.

