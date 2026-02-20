# Storage

Coming from [embeddings](/blogs/architecture/embeddings), you have a vector. Now the question is how Piramid actually keeps it alive — on disk, across restarts, through crashes — while still being fast enough to read back at query time. This section covers the on-disk layout for a collection, how memory-mapped I/O works and why it's the default, how the write-ahead log (WAL) makes writes durable, and what compaction does over time.


### On-disk layout

Every collection lives under a path you configure at startup (`DATA_DIR`). For a collection named `my_docs`, Piramid creates a set of files that together represent the complete state of that collection:

```
my_docs.db          — the main data file (mmap'd raw byte store)
my_docs.index.db    — the offset index: UUID → (offset, length) in the data file
my_docs.vidx.db     — the serialized vector index (HNSW graph, IVF clusters, etc.)
my_docs.meta.db     — collection-level metadata (vector count, timestamps, config)
my_docs.wal.db      — the write-ahead log
```

Each file has a distinct job. The data file is a flat binary store of serialized document entries — raw bytes, no self-describing structure beyond what the index tells you. The index file maps each document's UUID to an `(offset, length)` pair that says where in the data file that document lives. A lookup for a specific UUID goes: read the index to find the byte range, then read those bytes from the data file and deserialize. The vector index file holds the ANN index structure (HNSW graph adjacency lists, IVF centroids, etc.) serialized separately — it's a distinct artifact because its rebuild cost is high and you never want to touch it on every insert if you can avoid it. The metadata file tracks collection-level facts like vector count and the last checkpoint sequence. The WAL is discussed in its own section below.

There's intentional separation between the data file and the vector index because they have different access patterns and different levels of reconstruction cost. The data file is append-heavy and rarely rewritten. The vector index is rebuilt from scratch during index rebuild operations or compaction. Treating them as separate files lets each evolve independently.


### Memory-mapped I/O

The data file is accessed via memory mapping (`mmap`) rather than explicit file reads and writes, and this is the default. It's worth understanding what that actually means and why it matters.

When you call `mmap` on a file, the operating system maps the file's byte range into the process's virtual address space. No data is immediately copied into RAM. Instead, the OS sets up page table entries that point to the file on disk. When your code dereads an address in that range, the CPU generates a page fault, the OS copies the referenced 4KB page from disk into RAM, updates the page table, and resumes execution. Subsequent accesses to the same page hit RAM directly. For a data file that's larger than available RAM, the OS transparently evicts cold pages under memory pressure using its page replacement policy — LRU-approximate in the Linux kernel. The application code sees a flat contiguous byte slice; the OS handles all the physical I/O.

The advantage is that read paths through the data file don't require explicit syscalls. Once a page is resident, reads are just memory dereferences. Random access over a large file is efficient because only the referenced pages need to be in RAM at any time — you're not forced to load the whole file. The kernel's readahead heuristics also help with sequential scans.

The downside is that mmap growth requires unmapping and remapping. When a collection grows beyond its current file size, Piramid unmaps the file, calls `ftruncate` to extend it to twice the required size, and remaps it. Doubling is deliberate — it amortizes the remap cost the same way a dynamic array amortizes reallocation. For a collection whose size you can roughly bound ahead of time, tuning `initial_mmap_size` reduces how often this growth cycle happens.

For environments where mmap is undesirable (certain containerized deployments, or when you specifically want to limit OS page cache influence), mmap can be disabled via `use_mmap: false` in the memory config, which falls back to regular heap allocation backed by file I/O.


### Warming

When Piramid loads a collection on startup, the data file and vector index file are "warmed" by walking every page before the server starts accepting traffic. For mmap, warming means touching each 4KB-aligned offset with a read that forces the OS to fault every page into RAM:

```rust
const PAGE: usize = 4096;
let mut offset: usize = 0;
while offset < len {
    let byte = mmap[offset];
    std::hint::black_box(byte);
    offset = offset.saturating_add(PAGE);
}
```

Without warming, the first requests after startup would pay page fault latency on every uncached access — up to milliseconds per fault if the OS has to read from a cold NVMe. With warming, those faults are paid at startup time rather than during live traffic. For large collections this can mean a few seconds of startup delay in exchange for predictable sub-millisecond read latency once the server is live.


### The write-ahead log

Every mutation — insert, update, delete — is written to the WAL before it's applied to the data file. This ordering is the core durability guarantee: if the process crashes between logging the entry and actually modifying the data file, the WAL can replay that entry on the next startup and bring the collection to a consistent state. Without a WAL, a crash mid-write could leave the data file in a partially-updated state with no way to know what happened.

Each WAL entry is a JSON-serialized object appended to `my_docs.wal.db`. The entry types are:

```
WalEntry::Insert  { id, vector, text, metadata, seq }
WalEntry::Update  { id, vector, text, metadata, seq }
WalEntry::Delete  { id, seq }
WalEntry::Checkpoint { timestamp, seq }
```

The `seq` field is a monotonically increasing sequence number assigned at log time. Replay reads the WAL from disk and applies all entries with `seq > last_checkpoint_seq`, in order. This means recovery is bounded: you only need to replay entries since the last checkpoint, not the entire history of the collection.

There's a real tradeoff here between durability and write throughput. The WAL calls `flush()` after every entry by default, but `fsync` is a separate knob (`sync_on_write`). `flush()` drains the userspace `BufWriter` to the kernel buffer — fast, but a kernel crash can still lose the data. `fsync` pushes it all the way to the storage device — slow (a few milliseconds per call on most hardware, up to 10ms+ on HDDs), but truly durable. With `sync_on_write: false` (the default), you get good performance at the cost of a small window of potential data loss if the machine loses power. With `sync_on_write: true`, every write is bounded by disk fsync latency. Which setting is right depends on how much data loss you can tolerate.


### Checkpoints and WAL rotation

A checkpoint entry is written to the WAL periodically, controlled by two independent triggers: every `checkpoint_frequency` operations (default: 1000), or every `checkpoint_interval_secs` seconds (disabled by default). Whichever fires first causes a checkpoint.

When a checkpoint happens, Piramid serializes the current state of the index, vector index, and collection metadata to their respective `.db` files on disk, then writes a `Checkpoint` entry to the WAL. After that point, any WAL entry before the checkpoint sequence is no longer needed for recovery — the on-disk files already reflect that state. The WAL is then rotated: the current log file is closed and a new empty one is started. This prevents the WAL from growing indefinitely, which is especially important for long-running collections with high write volumes.

The tradeoff with checkpoint frequency is similar to the `fsync` tradeoff. More frequent checkpoints mean shorter recovery time after a crash (less to replay) but more I/O overhead during normal operation, since serializing the vector index in particular can be expensive for large HNSW graphs. Less frequent checkpoints mean lighter write overhead but longer recovery time if you crash between checkpoints.

> The default of 1000 operations per checkpoint was chosen as a reasonable middle ground for general workloads. For write-heavy pipelines, raising it to 10000 (the "fast" preset) cuts checkpoint overhead significantly. For high-durability requirements, lowering it along with `sync_on_write: true` gives near-transactional guarantees.


### Compaction

Over time, a collection that has seen lots of updates and deletes ends up with dead space in the data file. Deleted documents are tombstoned in the in-memory index (their UUID is removed), but their bytes remain on disk until a compaction runs. The same is true for updated documents — the old byte range is orphaned when a new version is written at a new offset.

Compaction rewrites the entire data file from scratch. It reads all live documents from the current mmap, truncates the file back to the initial size, remaps it, clears all indexes and caches, then reinserts every document through the normal insert path. After all documents are reinserted, it saves the index, vector index, and metadata, then rotates the WAL. The result is a file that contains only live data, no fragmentation, and a fresh WAL.

The cost is proportional to the number of live documents — if you have a million vectors, compaction reads and rewrites a million vectors. It's a blocking operation at the collection level, so it's something you schedule during low-traffic windows or trigger manually via the API. For collections that are mostly append-only with rare deletes, compaction frequency can be very low. For collections with high churn — frequent updates or deletions — compaction matters more.


### In-memory caches

Alongside the mmap, the collection maintains two in-memory caches: a vector cache (`HashMap<Uuid, Vec<f32>>`) and a metadata cache (`HashMap<Uuid, Metadata>`). These are simple hash maps keyed on document UUID. When a document is inserted or looked up, its vector and metadata are stored in the respective cache. Subsequent lookups for the same UUID return from cache without touching the mmap or deserializing from disk at all.

The caches are not LRU-evicting by default — they grow unboundedly unless a `max_memory_per_collection` limit is set. When the collection's total memory usage (mmap size + index size + cache sizes + vector index memory) exceeds the limit, the caches are cleared. This is a coarse-grained eviction strategy: you trade the cache warmup cost for staying within the configured memory budget. For collections that fit entirely in memory, the caches effectively make the collection a hot in-memory store with a durable on-disk backing.

The total memory usage for a collection is:

$$\text{mem} = |\text{mmap}| + |\text{index}| + \sum_{\text{cached vectors}} (16 + 4d) + |\text{metadata cache}| + |\text{vector index}|$$

where $d$ is the embedding dimension, 16 bytes is the UUID key, and $4d$ bytes is the float32 vector. For a fully-cached collection with $n = 10^6$ vectors of dimension $d = 1536$, the vector cache alone is roughly $10^6 \times (16 + 6144) \approx 6.16\text{ GB}$. That's why the memory limit config matters — without it, a large collection will happily consume all available RAM.
