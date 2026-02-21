# Architecture

In this section of blogs, I will explain core components of vector databases, how they work, and also cover Piramid's architecture, the different components involved, how they interact with each other, the design decisions and tradeoffs made in the architecture, and the future possibilities and directions for the architecture.

## What's covered

<PostCards>
  <PostCard href="/blogs/architecture/database" title="Databases">
    What a database is, how vector databases differ from relational, document, and graph stores, and why similarity search requires a fundamentally different model.
  </PostCard>
  <PostCard href="/blogs/architecture/embeddings" title="Embeddings">
    Where vectors come from, how neural encoders work, what it means for two vectors to be geometrically close, and how embedding quality affects retrieval.
  </PostCard>
  <PostCard href="/blogs/architecture/indexing" title="Indexing">
    The three index types (Flat, IVF, HNSW), the tradeoffs between exact recall and query latency, and how auto-selection works at different collection sizes.
  </PostCard>
  <PostCard href="/blogs/architecture/query" title="Query">
    How a search request moves through the engine: ANN traversal, metadata filtering, overfetch, and the recall/latency tradeoff in practice.
  </PostCard>
  <PostCard href="/blogs/architecture/storage" title="Storage">
    How Piramid keeps data alive across restarts: mmap, the write-ahead log, checkpoints, compaction, and what the durability guarantees actually mean.
  </PostCard>
</PostCards>

