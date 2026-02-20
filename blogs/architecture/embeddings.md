# Embeddings

In the [previous section](/blogs/architecture/database) we established that a vector database doesn't search for exact matches — it searches for geometric proximity in embedding space. That raises the obvious follow-up question: where do those vectors come from, and what does it actually mean for two vectors to be "close"? That's what this section is about.


### What an embedding actually is

An embedding is a function $f: \mathcal{X} \to \mathbb{R}^d$ that maps some input domain $\mathcal{X}$ — text strings, images, audio clips — to a point in $d$-dimensional real-valued space. The function is learned, not designed. A neural network is trained on large amounts of data with an objective that forces semantically related inputs to map to geometrically nearby points. Once trained, the network is frozen and its internal activations at some layer become the embedding vector.

The simplest historical example is Word2Vec, trained with a skip-gram objective: given a word $w$, predict its context words $c$ within a window of size $k$. The training objective maximizes:

$$\mathcal{L} = \sum_{t=1}^{T} \sum_{-k \le j \le k, j \ne 0} \log P(w_{t+j} \mid w_t)$$

where $P(c \mid w)$ is modeled as a softmax over all vocabulary words. The gradients from this objective shape a 300-dimensional embedding space where words that appear in similar contexts end up near each other. The famous arithmetic — $\vec{king} - \vec{man} + \vec{woman} \approx \vec{queen}$ — falls out as an emergent property of this training, not something explicitly built in.

Modern text embedding models are transformer-based. The input text goes through multiple self-attention layers, and the embedding is typically the mean-pooled output across all token positions in the final layer (or a learned `[CLS]` token). The training objective is more sophisticated — contrastive learning on pairs of text, where positive pairs (semantically related sentences) are pushed together and negative pairs are pushed apart. The loss function used by most current models is a variant of the InfoNCE loss:

$$\mathcal{L} = -\frac{1}{N} \sum_{i=1}^{N} \log \frac{e^{\text{sim}(\mathbf{z}_i, \mathbf{z}_i^+) / \tau}}{\sum_{j=1}^{N} e^{\text{sim}(\mathbf{z}_i, \mathbf{z}_j^-) / \tau}}$$

where $\mathbf{z}_i$ and $\mathbf{z}_i^+$ are embeddings of a matched pair, $\mathbf{z}_j^-$ are embeddings of non-matching samples in the same batch, and $\tau$ is a temperature hyperparameter. The denominator sums across all negatives in the batch, which is why larger batch sizes during training tend to produce better embeddings — more negatives means a harder contrastive task and cleaner separation in the learned space.

The practical upshot of all this is that the quality of everything downstream — retrieval recall, hybrid search relevance, duplicate detection — is ultimately bounded by how good your embedding model is and how well its training distribution matches your data. No index structure or query optimization can recover recall that the embedding model never produced.


### Providers in Piramid

Piramid treats embedding generation as a runtime concern — you configure a provider and model, and the server handles the rest. There are two supported provider types today.

#### OpenAI

The OpenAI provider supports three models: `text-embedding-3-small` (1536 dimensions), `text-embedding-3-large` (3072 dimensions), and `text-embedding-ada-002` (1536 dimensions, legacy). The request goes to `https://api.openai.com/v1/embeddings` by default, with the API key read from config or the `OPENAI_API_KEY` environment variable — whichever is present, in that order. The response comes back as a float32 array, which is stored directly as `Vec<f32>` internally.

One thing worth knowing about `text-embedding-3-small` vs `text-embedding-3-large`: the larger model produces higher-dimensional vectors with meaningfully better semantic resolution, but at roughly 2× the API cost and 2× the memory per stored vector. For most RAG use cases `text-embedding-3-small` hits a good cost/quality balance. The legacy `ada-002` is there mostly for collections that were built before the v3 models existed.

#### Local HTTP

The local provider speaks to any OpenAI-compatible or TEI-style HTTP endpoint. TEI (Text Embeddings Inference) is Hugging Face's high-throughput embedding server, and it exposes the same `/embeddings` JSON contract as the OpenAI API. That means Ollama, TEI, or any other locally-hosted embedding server all work without modification as long as they speak that protocol — you just point `base_url` at the right address.

This matters a lot for Piramid's core use case. If you're running a local LLM for inference, you almost certainly don't want to send your documents to OpenAI's API for embedding either. Having both on the same machine keeps data local and eliminates the round-trip latency of a remote embedding call entirely.


### Configuration and resolution order

The embedding config is set at server startup in `piramid.yaml` or via environment variables. The fields that matter:

```yaml
embeddings:
  provider: openai          # openai | local
  model: text-embedding-3-small
  base_url: ~               # required for local; overrides default for openai
  api_key: ~                # read from OPENAI_API_KEY env var if absent
  timeout: 30               # seconds; no timeout if absent
```

Environment variables take precedence over the config file for the API key specifically (`OPENAI_API_KEY`), which is the standard expectation for secrets in containerized deployments. Everything else comes from the config file. There's no per-request override for provider or model today — the collection uses whatever the server was configured with at startup.

The `timeout` field controls how long the HTTP client waits for a response from the embedding endpoint before giving up. For OpenAI's API, 30 seconds is generous — normal latency is under 500ms for typical inputs. For local models running on CPU, you might need more. If absent, there is no timeout, which can cause requests to hang indefinitely if the embedding server becomes unresponsive.


### Request flow: /embed and /search/text

When you call `POST /api/collections/{collection}/embed`, Piramid takes your text, routes it through the configured embedder, and stores the resulting vector. When you call `POST /api/collections/{collection}/search/text`, the same thing happens for the query text — it's embedded on the fly and then passed into the ANN search path just like a raw vector search would be.

The embedder stack that handles these requests is layered:

```
Request
  └── RetryEmbedder          (exponential backoff on transient failures)
        └── CachedEmbedder   (LRU cache, default 10K entries)
              └── Provider   (OpenAI or LocalEmbedder)
```

The cache sits between retry and the actual provider. A cache hit returns immediately without touching the network. A cache miss falls through to the provider, and the result gets stored in the LRU on the way back up. The cache is keyed on the raw input text string, so identical inputs across different requests will hit the cache regardless of which endpoint or collection they came from.

The retry layer uses exponential backoff: first retry after 1 second, doubling each time up to a 30-second cap, with a maximum of 3 attempts. Errors are classified before deciding whether to retry — a 401 authentication failure isn't retryable (retrying with the same bad key just makes things worse), but a 429 rate limit or a network-level failure is. Hitting the retry limit propagates the last error back through the stack, which surfaces as a 5xx from the API.

> The embedding provider is also exposed as a health check at `GET /api/health/embeddings`. A failing embedding provider will show up there before it causes query failures — worth watching in production.


### The caching tradeoff

The LRU cache holds up to 10,000 embeddings by default. Each `text-embedding-3-small` vector is 1536 float32 values, which is $1536 \times 4 = 6{,}144$ bytes per entry — call it 6KB. A full cache of 10K entries is about 60MB in memory. That's a manageable overhead in exchange for avoiding API calls on repeated inputs.

The hit rate depends entirely on your workload. For a RAG system where the same documents get re-embedded repeatedly (e.g., every time the server restarts), the cache basically eliminates embedding API cost for the warm-up phase. For workloads where every input is unique — like real-time user queries — the hit rate will be near zero and the cache adds only a lock acquisition overhead per request, which is sub-millisecond and not worth worrying about.

One thing the cache doesn't do is persist across restarts. It's in-memory only. If your workload has a large hot set that benefits from caching, a server restart means re-warming the cache from scratch. This is a known limitation — persistent embedding caches are a possible future direction.


### Dimensions, memory, and the cost of scale

It's worth doing the math on what embedding storage actually costs at scale, because it grows fast and the numbers aren't intuitive.

A collection with $n$ vectors of dimension $d$ stored as float32 requires $4nd$ bytes on disk and in memory (when loaded). For $n = 10^6$ and $d = 1536$ (OpenAI's small model):

$$4 \times 10^6 \times 1536 = 6.144 \times 10^9 \text{ bytes} = 6.1 \text{ GB}$$

For `text-embedding-3-large` at $d = 3072$ that doubles to 12.3GB. These are just the raw vectors — not counting the index structure (HNSW adds roughly 1.1× overhead from graph edges) or metadata. At 10 million vectors you're looking at tens of gigabytes just for the embedding data.

This is exactly why quantization matters. Piramid supports int8 scalar quantization, which compresses each float32 component to an int8:

$$\hat{x}_i = \text{round}\!\left(\frac{x_i - x_{\min}}{x_{\max} - x_{\min}} \times 255\right)$$

That reduces per-vector storage from $4d$ bytes to $d$ bytes — a 4× reduction. For the million-vector example above, 6.1GB becomes 1.5GB, with a small, measurable degradation in recall due to quantization error. Whether that's acceptable depends on your precision requirements. More on this in the [storage section](/blogs/architecture/storage).
