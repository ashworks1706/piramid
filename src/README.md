# Piramid Source Code

Vector database for agentic applications.

## How to Read This Codebase

### 1. Start here → `lib.rs`
The entry point. Shows all the public types and how they're organized. Read this first to see the big picture.

### 2. Core data structure → `storage.rs`
The heart of the database. Understand:
- `VectorEntry` - what you store (vector + text + metadata)
- `VectorStorage` - how storage works (HashMap in RAM, file on disk)
- How `search()` works (brute-force: compare against all vectors)

### 3. How similarity works → `distance/`
Read in order:
1. `mod.rs` - the `DistanceMetric` enum and `match`
2. `cosine.rs` - the most important metric, with math explanation
3. Skim `euclidean.rs` and `dot.rs`

### 4. Metadata & filtering
1. `metadata.rs` - how enums with data work, `From` trait
2. `query/filter.rs` - builder pattern, closure-based filtering

### 5. Error handling → `error.rs`
Short file showing Rust's `Result<T, E>` pattern and `thiserror`

### 6. Supporting files (skim)
- `search.rs` - just a struct for search results
- `config.rs` - trivial config struct

### 7. See it in action → `examples/basic.rs`
Run `cargo run --example basic` while reading to see how it all fits together

---

## Mental Model

```
User calls storage.search(query_vector, k, metric)
         │
         ▼
┌─────────────────────────────────────────┐
│  VectorStorage                          │
│  ┌─────────────────────────────────┐    │
│  │ HashMap<Uuid, VectorEntry>      │    │
│  │   - vector: [0.1, 0.2, ...]     │    │
│  │   - text: "original text"       │    │
│  │   - metadata: {key: value}      │    │
│  └─────────────────────────────────┘    │
│         │                               │
│         ▼  for each entry               │
│  ┌─────────────────────────────────┐    │
│  │ DistanceMetric::Cosine          │    │
│  │   cosine_similarity(query, vec) │    │
│  └─────────────────────────────────┘    │
│         │                               │
│         ▼  sort by score, take top k    │
│  Vec<SearchResult>                      │
└─────────────────────────────────────────┘
```

---

## Key Rust Concepts by File

| File | What you'll learn |
|------|-------------------|
| `storage.rs` | `&self` vs `&mut self`, `?` operator, iterators, closures |
| `metadata.rs` | Enums with data, `From` trait, const generics |
| `distance/mod.rs` | Modules, `pub use`, exhaustive `match` |
| `cosine.rs` | Tests, `#[cfg(test)]`, assertions |
| `error.rs` | `Result`, `thiserror`, type aliases |
| `filter.rs` | Builder pattern, `Option::map_or` |

---

## File Structure

```
src/
├── lib.rs           # Public API exports
├── storage.rs       # Core storage engine (start here after lib.rs)
├── config.rs        # Simple config struct
├── error.rs         # Error types
├── metadata.rs      # Key-value metadata for vectors
├── search.rs        # Search result struct
├── distance/        # Similarity calculations
│   ├── mod.rs       # DistanceMetric enum
│   ├── cosine.rs    # Cosine similarity (most common)
│   ├── euclidean.rs # L2 distance
│   └── dot.rs       # Dot product
└── query/           # Search filtering
    ├── mod.rs       
    └── filter.rs    # Metadata filters
```
