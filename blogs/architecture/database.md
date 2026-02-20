# Databases



### But, What is a database?
A database is a structured collection of data that allows for efficient storage, retrieval, and management. Traditional databases are designed to handle structured data (like tables with rows and columns) and support operations like querying, updating, and deleting data based on exact matches.

There are many types of databases, and each one exists because of a specific set of tradeoffs its designers decided to prioritize. Understanding them is worth doing properly, because vector databases don't exist in isolation — they slot into a broader ecosystem, and knowing where other systems stop is how you understand where vector databases begin.


#### In-memory databases

The premise is simple: instead of reading and writing data to disk, you keep everything in RAM. That sounds obvious until you remember that a RAM access takes roughly 100 nanoseconds, while a random disk read — even on a fast NVMe SSD — is somewhere between 100 and 1000 microseconds. That's a 1,000x difference, and for certain workloads it changes everything.

Under the hood, most in-memory stores are built on hash tables. A key is hashed (using something like MurmurHash or xxHash) to produce a bucket index, and the value lives at that address. With a good hash function and a reasonable load factor, lookups are effectively O(1). Redis takes this further by supporting richer data structures — sorted sets backed by a skip list plus a hash table, which gives you O(log n) range queries while still maintaining O(1) point lookups. Streams use a radix tree. Lists use a doubly linked list that compresses to a listpack below a configurable threshold. Each structure was chosen because of what the access pattern actually demands, not uniformity.

The obvious limitation is volatility. If the process dies, the data is gone — unless you configure persistence. Redis solves this in two ways: RDB (a periodic full snapshot to disk, cheap to read, lossy) and AOF (an append-only log of every write command, recoverable to the last sync). You can run both simultaneously. The tradeoff is that the further you push toward durability, the more the latency characteristics start resembling a disk-backed system. Nothing is free.

Use cases where this matters: caching layers sitting in front of a slower database, session state for web applications, real-time leaderboards, rate limiting counters, and pub/sub messaging. Basically anything where you need sub-millisecond response times and the dataset fits in RAM. The cost of RAM is also a real constraint — holding a terabyte of data in memory is expensive in a way that holding it on disk simply isn't.


#### Relational databases

Relational databases are probably what most people think of when they hear "database." Postgres, MySQL, SQLite — the idea is that data is organized into tables with defined schemas (columns with types), and rows represent individual records. The relational part comes from the ability to model *relationships* between tables and then query across them using SQL joins.

The math that makes them fast is primarily B-trees. When you create an index on a column, the database builds a balanced tree structure where each node holds keys and pointers to children. A lookup traverses from the root to a leaf in O(log n) time, regardless of table size. For a table with a billion rows, a B-tree index on a unique column takes about 30 comparisons to find any record. Without the index, you'd scan every row.

ACID is the other property relational databases are known for. Atomicity means a transaction either fully commits or fully rolls back — no partial state. Consistency means every transaction brings the database from one valid state to another. Isolation means concurrent transactions don't interfere with each other in unexpected ways (enforced via locking or MVCC, multi-version concurrency control). Durability means once a commit is acknowledged, the data survives a crash. Postgres implements this using a write-ahead log — every change is logged to disk before it's applied to the data files, so if the system crashes mid-write, the log can be replayed on startup.

Relational databases are excellent when your data has clear structure, changes to that structure are infrequent, and you need transactional integrity. Think financial ledgers, user accounts, inventory systems — anything where the correctness of relationships between pieces of data actually matters. They're also worth understanding because almost every other database type is in some way a reaction to their limitations: schema rigidity, difficulty scaling horizontally across machines, and poor performance when each row is a semi-structured blob.


#### Non-relational databases

Non-relational (or NoSQL) databases are a wide umbrella that covers several very different systems, unified mainly by the fact that they don't organize data as fixed-schema tables. The reason they exist is that the relational model, for all its strengths, doesn't map naturally to every problem.

Document databases like MongoDB store data as JSON (or BSON) documents. The schema is flexible — one document in a collection can have fields that another doesn't. Internally, documents are typically stored in a B-tree-adjacent structure and queried by field values. This works well when your data is already naturally document-shaped (think a product catalog, a CMS, event logs) and when you'd rather evolve your schema over time without migrations.

Key-value stores (DynamoDB, the core of Redis) take the relational model to its minimum: a key maps to a value. The value is opaque to the system. DynamoDB partitions data across nodes using consistent hashing — a technique where the hash space is a ring and keys are distributed across nodes such that adding or removing a node only moves a fraction of the keys, rather than rehashing everything. Very fast at what it does, terrible at anything that requires scanning across keys or expressing relationships.

Wide-column stores like Cassandra and HBase are built for write-heavy, distributed workloads. Data is still organized by rows and columns, but the column schema is dynamic and can vary per row. The on-disk layout is column-oriented within each partition, which makes range scans on sorted data fast and compaction predictable. Cassandra in particular is designed around eventual consistency and the Dynamo lineage — it uses a gossip protocol for cluster coordination and lets you tune the consistency/availability tradeoff per query. The price you pay is that there's no join support and query patterns have to be designed upfront around how data is partitioned.

Graph databases like Neo4j model data as nodes and edges. The storage is an adjacency list internally, but the key insight is that relationship traversal is O(1) per hop — you don't do a join across a table, you follow a pointer. For problems that are fundamentally about relationships (social networks, fraud detection, knowledge graphs), this is dramatically faster than trying to express the same structure in SQL.

The common thread across all of these is that they traded the generality and correctness guarantees of the relational model for something specific: flexibility, scale, write throughput, or relationship traversal speed. They typically operate under BASE semantics (Basically Available, Soft state, Eventually consistent) rather than ACID, and the CAP theorem is usually lurking in the design — you can have at most two of consistency, availability, and partition tolerance at the same time.

![Databases](https://media.geeksforgeeks.org/wp-content/uploads/20250703161012389874/Types-of-Databases.webp)



### What is a vector database?

Vectors are mathematical representations of data points in a multi-dimensional space. Each vector consists of a list of numbers (called dimensions) that capture the characteristics of the data point. For example, in natural language processing, a vector might represent a word or a sentence, where each dimension captures some aspect of its meaning or context.

![Vectors illustration](https://www.nvidia.com/content/nvidiaGDC/us/en_US/glossary/vector-database/_jcr_content/root/responsivegrid/nv_container_1795650/nv_image_copy.coreimg.100.1070.jpeg/1710829331227/vector-database-embedding-1920x1080.jpeg)

A vector database is a type of database that is designed to store and manage high-dimensional data, such as vectors. While traditional databases search for exact matches (like finding the exact word "apple" in a text column), vector databases search for semantic similarity. They allow you to find data that *means* the same thing, even if it doesn't look exactly the same. This is the underlying technology powering modern AI, including large language models, recommendation engines, and reverse image searches.


### 2. Key Components of a Vector Database System

A complete vector database ecosystem involves several moving parts that work together to ingest data, organize it, and retrieve it quickly.

#### A. The Embedding Model (The Translator)

While not technically part of the database storage itself, the embedding model is the required first step. Before data goes into the database, an AI model (like OpenAI's text-embedding models or open-source models like BERT) processes your documents, images, or audio and outputs the vector arrays.

An embedding is a translation of raw data (text, images, audio, or video) into a high-dimensional vector. Imagine a list of thousands of numbers, like `[0.12, -0.45, 0.89, ... ]`. AI models (like neural networks) are trained to convert data into these arrays so that the numbers capture the *meaning* or *context* of the original data.

In this high-dimensional mathematical space:

* Concepts that are similar are placed close to each other. For example, the vector for "dog" will be geometrically very close to the vector for "puppy," but far away from the vector for "car."
* When you ask a vector database a question, your question is converted into a vector. The database then searches its space to find the stored vectors that are physically closest to your question's vector.


#### B. The Vector Index (The Organizer)

If a database has millions of vectors, comparing your query to every single one (an exhaustive search) would be far too slow. The index organizes vectors to enable Approximate Nearest Neighbor (ANN) search. It trades a tiny bit of accuracy for a massive boost in speed. Common indexing algorithms include:

* HNSW (Hierarchical Navigable Small World): A graph-based approach that creates a multi-layered map of the vectors, allowing the search to quickly zoom in on the right neighborhood.
* IVF (Inverted File Index): Groups vectors into clusters. When you search, it only looks ins
todo -- explain in memory storages, their use cases, the math behind them, how they exactly work in depth, give examples and the variations of them


todo -- explain relational databases, their use cases, the math behind them, how they exactly work in depth, give examples and the variations of them


todo -- explain non relational databases, their use cases, the math behind them, how they exactly work in depth, give examples and the variations of themide the clusters that are closest to your query.
* PQ (Product Quantization): Compresses the vectors so they take up less memory, speeding up the calculation process.


#### C. Distance Metrics (The Rulers)

Once the database finds a neighborhood of vectors, it needs a mathematical way to measure exactly how "close" they are to your query vector. The database uses distance metrics, such as:

* Cosine Similarity: Measures the angle between two vectors. It's excellent for text because it focuses on the direction (meaning) rather than the magnitude (length of the text).
* Euclidean Distance (L2): Measures the straight-line distance between two points in space.
* Dot Product: Multiplies the vectors together; often used when vectors are normalized.


#### D. Metadata Storage (The Context)

A vector by itself is just a string of numbers. To be useful, the database must also store the original data or metadata attached to that vector. For example, alongside the vector, it stores the original text snippet, the URL of the document, an image ID, or an author's name. This allows you to perform hybrid searches (e.g., "Find vectors similar to 'financial report' BUT only where the metadata year is 2025").

#### E. The Query Engine / API

This is the interface you interact with. It handles your incoming requests, coordinates the vector search with metadata filtering, scores the results based on the distance metric, and returns the top *k* most relevant results (e.g., "return the top 5 closest matches").


> More on these things in [later sections](/blogs/architecture)

---

### What does the flow look like in practice?

1. You feed raw text into an embedding model.
2. The model spits out a vector. You save that vector and its metadata (the original text) into the vector database.
3. The database updates its index (like HNSW) to place this new vector in the correct "neighborhood."
4. A user asks a question. The question is turned into a vector. The database rapidly searches the index using a distance metric and returns the metadata of the closest matching vectors.
