# Embeddings

In the [previous section](/blogs/architecture/database) we established that a vector database doesn't search for exact matches — it searches for geometric proximity in embedding space. That raises the obvious follow-up question: where do those vectors come from, and what does it actually mean for two vectors to be "close"? That's what this section is about.

![embeddings](https://xomnia.com/wp-content/uploads/2025/05/vector-database.png)


### From words to numbers — why representation matters

Before neural embeddings, the standard way to represent text for information retrieval was bag-of-words or [TF-IDF](https://en.wikipedia.org/wiki/Tf%E2%80%93idf). A document becomes a sparse vector of length $|\mathcal{V}|$ (the vocabulary size, typically 50,000–200,000), where component $i$ is some weight for word $i$.

TF-IDF weights are:

$$\text{tf-idf}(t, d, D) = \underbrace{\frac{f_{t,d}}{\sum_{t'} f_{t',d}}}_{\text{term frequency}} \times \underbrace{\log\frac{|D|}{|\{d' \in D : t \in d'\}|}}_{\text{inverse document frequency}}$$

This is fast and interpretable. The problem is that it is purely lexical. The words "car" and "automobile" are orthogonal vectors even though they mean the same thing. "Not good" and "bad" are distant even though they express the same sentiment. Any query about "machine learning" misses documents about "deep learning" or "neural networks" unless those exact words appear. Synonymy, polysemy, and compositionality all break TF-IDF.

Distributed representations (embeddings) solve this by learning a *dense* $d$-dimensional vector for each concept where the coordinates encode meaning — similar meanings land near each other. The key insight is the **distributional hypothesis**: words that appear in similar contexts tend to have similar meanings. If you train a model to predict context from word (or word from context), the internal representations it must learn to do this successfully will encode semantic similarity as geometric proximity.

> **The manifold hypothesis:** real-world data like text, images, and audio doesn't fill $\mathbb{R}^d$ uniformly. It lies near a much lower-dimensional curved surface (a manifold) embedded in the high-dimensional space. A model that learns to embed sentences into $\mathbb{R}^{1536}$ is essentially learning a coordinate system on this manifold. Two sentences that are close on the manifold — semantically similar — map to nearby coordinates. This is why a $d = 1536$ vector can meaningfully represent the semantics of a sentence that might require thousands of words to fully describe: the manifold intrinsically has far fewer than 1536 degrees of freedom.

### What an embedding actually is

An embedding is a function $f: \mathcal{X} \to \mathbb{R}^d$ that maps some input domain $\mathcal{X}$ — text strings, images, audio clips — to a point in $d$-dimensional real-valued space. The function is learned, not designed. A neural network is trained on large amounts of data with an objective that forces semantically related inputs to map to geometrically nearby points. Once trained, the network is frozen and its internal activations at some layer become the embedding vector.

#### [Word2Vec](https://arxiv.org/abs/1301.3781) and the skip-gram objective

The simplest historical example that makes this concrete is Word2Vec. Its skip-gram variant trains a shallow neural network to predict context words $c$ from a center word $w$ within a window of size $k$. The training objective maximises:

$$\mathcal{L} = \sum_{t=1}^{T} \sum_{-k \le j \le k,\, j \ne 0} \log P(w_{t+j} \mid w_t)$$

where $P(c \mid w)$ is modelled via a softmax over all vocabulary words:

$$P(c \mid w) = \frac{\exp(\mathbf{u}_c^\top \mathbf{v}_w)}{\sum_{c' \in \mathcal{V}} \exp(\mathbf{u}_{c'}^\top \mathbf{v}_w)}$$

Here $\mathbf{v}_w$ is the "input" embedding for word $w$ and $\mathbf{u}_c$ is the "output" embedding for context word $c$. The problem is that evaluating this softmax requires summing over all $|\mathcal{V}|$ vocabulary members per training step — $O(|\mathcal{V}|)$ which is prohibitely expensive at vocabulary sizes of 100K+.

The practical solution is **negative sampling**: instead of computing the full softmax, train a binary classifier that distinguishes the true context word from $K$ randomly sampled "noise" words. The objective becomes:

$$\mathcal{L}_{\text{NS}} = \log \sigma(\mathbf{u}_c^\top \mathbf{v}_w) + \sum_{j=1}^{K} \mathbb{E}_{c_j \sim P_n}\left[\log \sigma(-\mathbf{u}_{c_j}^\top \mathbf{v}_w)\right]$$

where $\sigma$ is the sigmoid function and $P_n$ is a noise distribution (typically unigram frequency raised to the 3/4 power). This replaces $O(|\mathcal{V}|)$ with $O(K)$ per update, with $K = 5$–$20$ in practice.

The gradients from this objective shape a 300-dimensional embedding space where words that appear in similar contexts end up near each other. The famous arithmetic — $\vec{\text{king}} - \vec{\text{man}} + \vec{\text{woman}} \approx \vec{\text{queen}}$ — falls out as an emergent property, not something explicitly built in: it reflects that the "royalty" direction and the "gender" direction are approximately linear in the learned space.

Word2Vec is a useful intuition builder but it has hard limits. Each word gets exactly one vector regardless of context, so "bank" (financial) and "bank" (river) share a single representation. And it operates at the word level — there's no way to represent a whole sentence.

#### Transformers and contextual embeddings

Modern embedding models are transformer-based and address both limitations. The transformer architecture was introduced in ["Attention is All You Need" (Vaswani et al. 2017)](https://arxiv.org/abs/1706.03762). Its central mechanism is **multi-head self-attention**, which lets every token's representation be influenced by every other token in the sequence.

For a single attention head with input matrix $X \in \mathbb{R}^{n \times d_\text{model}}$ (where $n$ is sequence length), the computation is:

$$\text{Attention}(Q, K, V) = \text{softmax}\!\left(\frac{QK^\top}{\sqrt{d_k}}\right) V$$

where $Q = XW^Q$, $K = XW^K$, $V = XW^V$ are linear projections, and $d_k$ is the key dimension. The $1/\sqrt{d_k}$ scaling factor prevents the dot products from growing large enough to push softmax into regions of vanishing gradient — without it, the softmax saturates and learning stalls.

Multi-head attention runs $H$ such operations in parallel with separate projections, then concatenates and projects the outputs:

$$\text{MultiHead}(Q, K, V) = \text{Concat}(\text{head}_1, \ldots, \text{head}_H) W^O, \quad \text{head}_i = \text{Attention}(XW_i^Q, XW_i^K, XW_i^V)$$

With $H = 12$ heads and $d_\text{model} = 768$ (BERT-base), each head operates in $d_k = 64$ dimensions. Different heads learn to attend to different types of relationships — syntax, coreference, semantic roles — simultaneously.

The result is that each token's output representation is a weighted mixture of all tokens' value vectors, with weights determined by how "relevant" each token is to the current one. The word "bank" in "river bank" ends up near "water" rather than "finance" because the attention weights pull the representation in the direction of the context.

> **Positional encoding:** transformers have no inherent notion of word order (unlike RNNs). Position is injected by adding a positional encoding $\mathbf{pe}_{pos}$ to each token embedding before the first attention layer. The original formulation uses sinusoidal functions: $\mathbf{pe}_{pos,2i} = \sin(pos / 10000^{2i/d})$, $\mathbf{pe}_{pos,2i+1} = \cos(pos / 10000^{2i/d})$. Modern models use [RoPE (Rotary Position Embedding)](https://arxiv.org/abs/2104.09864) which encodes relative rather than absolute positions and generalises better to sequences longer than those seen during training.

To get a single embedding vector for an entire input text, most models either use the final-layer hidden state of a special `[CLS]` token inserted at the start of the sequence, or compute the **mean pool** of all token representations across the final layer:

$$\mathbf{e}_\text{mean-pool} = \frac{1}{n} \sum_{i=1}^{n} \mathbf{h}_i^{(L)}$$

Mean pooling treats all tokens equally; the CLS token approach trains the model to distil the full sequence meaning into that single position. Empirically, mean pooling tends to outperform CLS pooling on retrieval benchmarks when the model wasn't specifically trained with a CLS objective.

#### Contrastive learning — teaching similarity

A pre-trained language model knows syntax and semantics, but its internal similarity geometry isn't necessarily aligned with what you want a retrieval system to do. A model trained on next-token prediction (like GPT) will have token representations useful for generation, not necessarily for semantic retrieval. Contrastive fine-tuning reshapes the embedding space specifically for similarity search.

Contrastive training operates on pairs: a query $q$ and a positive $p^+$ (semantically related) and a set of negatives $\{n_j\}$ (unrelated). The standard loss is a variant of [InfoNCE (Noise Contrastive Estimation)](https://arxiv.org/abs/1807.03748):

$$\mathcal{L} = -\frac{1}{N} \sum_{i=1}^{N} \log \frac{e^{\text{sim}(\mathbf{z}_i, \mathbf{z}_i^+) / \tau}}{\sum_{j=1}^{N} e^{\text{sim}(\mathbf{z}_i, \mathbf{z}_j^-) / \tau}}$$

where $\mathbf{z}_i$ and $\mathbf{z}_i^+$ are the normalized embeddings of a positive pair, and $\mathbf{z}_j^-$ are all negatives in the batch. The temperature $\tau$ controls how sharply peaked the distribution is: small $\tau$ makes the model very sensitive to small distance differences (high contrast), while large $\tau$ allows more spread.

The denominator sums over all non-matching samples in the batch — this is **in-batch negative sampling**, and it's why larger training batch sizes produce better embedding models. With $N = 4096$ samples per batch, each training example is contrasted against 4095 negatives simultaneously. The signal from 4095 negatives is much richer than from a handful, forcing the model to learn a much stricter notion of "similar."

> **Hard negatives:** the most effective training uses **hard negatives** — samples that are superficially similar to the query but subtly different (e.g., the same question paraphrased slightly, or a document from the same domain but a different topic). In-batch random negatives are mostly easy (a query about cooking vs a passage about finance is trivially distinguishable). Hard negatives force the model to develop fine-grained discriminative representations. Models like [E5](https://arxiv.org/abs/2212.03533), [GTE](https://arxiv.org/abs/2308.03281), and the OpenAI text-embedding-3 series are trained with hard negative mining pipelines — the measurable quality difference between them and earlier models is largely attributable to this.

**Temperature calibration** has a mathematically interesting role. The optimal $\tau$ isn't a fixed constant — it depends on the expected magnitude of similarity scores in your data. If your embeddings are $\ell_2$-normalized (as is standard), cosine similarity ranges from -1 to 1. The InfoNCE loss with $\tau = 0.07$ (a common value) turns this into an effective "temperature" for the distribution: $\text{sim}/0.07$ values range from about -14 to +14, giving a reasonably peaked softmax. Too-small $\tau$ produces a distribution so sharp that small numerical errors dominate; too-large $\tau$ makes the loss insensitive to the exact relative ordering of candidates. OpenAI's models use a learned `logit_scale` parameter that plays the role of $1/\tau$, optimising it jointly with the embedding weights during training.

#### The geometry of learned embedding space

A few geometric properties of embedding spaces are worth understanding because they dictate how you should configure distance metrics and quantisation.

**$\ell_2$ normalisation.** Most production embedding models output $\ell_2$-normalised vectors — each vector satisfies $\|\mathbf{z}\|_2 = 1$. On the unit hypersphere, cosine similarity and Euclidean distance are monotonically related:

$$\|\mathbf{z}_i - \mathbf{z}_j\|_2^2 = 2 - 2\cos\theta_{ij}$$

So minimising L2 distance and maximising cosine similarity are equivalent for normalised vectors. The practical reason to normalise is that it makes similarity scores comparable across different inputs — unnormalised dot products are biased by vector magnitude, which correlates with input length and vocabulary frequency.

**Anisotropy and dimensional collapse.** Language models trained only with next-token prediction tend to produce anisotropic embedding spaces — all vectors cluster in a narrow cone of the "occupied" subspace rather than spreading uniformly across $\mathbb{R}^d$. This was documented in the "BERT sentence embeddings" literature ([Ethayarajh 2019](https://arxiv.org/abs/1908.10084), [Li et al. 2020](https://arxiv.org/abs/2011.05864)) and is measurable as a high average cosine similarity across random sentence pairs ($\bar{\cos} \approx 0.9$ for vanilla BERT vs $\bar{\cos} \approx 0.02$ for a well-trained retrieval model). Contrastive fine-tuning pushes the embeddings toward **isotropy** — spreading them across the full surface of the unit sphere — which dramatically improves nearest-neighbour search quality.

**Matryoshka Representation Learning (MRL).** OpenAI's `text-embedding-3` models use a training technique called [MRL (Kusupati et al. 2022)](https://arxiv.org/abs/2205.13147) that makes the embedding dimensions hierarchically meaningful. The full $d$-dimensional vector is trained, but the loss is summed over a set of nested truncation sizes $m_1 < m_2 < \ldots < m_L = d$:

$$\mathcal{L}_\text{MRL} = \sum_{\ell=1}^{L} \lambda_\ell \cdot \mathcal{L}\bigl(\mathbf{z}_{[1:m_\ell]}\bigr)$$

where $\mathbf{z}_{[1:m_\ell]}$ is the first $m_\ell$ dimensions of the full embedding. This forces the model to pack the most discriminative information into the first few dimensions, with later dimensions adding progressively finer-grained signal. The practical benefit: you can truncate `text-embedding-3-small`'s 1536-dimensional output to 512, 256, or even 64 dimensions and still get strong retrieval performance — with Recall@10 at 256 dimensions being only a few points below the full 1536d. This is a significant memory and compute saving when you control the tradeoff consciously.

> **Choosing dimensions with MRL:** a useful heuristic for selecting the truncation size $m$ is to plot Recall@10 vs $m$ on a sample of your actual query/document pairs and find the elbow point. For most English text retrieval tasks, the elbow is around 256–512 dimensions. Going below 128 usually degrades recall noticeably, while going from 1024 to 1536 often gives less than 1 point of improvement. Only you can decide what tradeoff is right given your latency budget and recall requirements.


### Providers in Piramid

Piramid treats embedding generation as a runtime concern — you configure a provider and model at startup, and the server handles the rest. There are two supported provider types today, both implementing the same `Embedder` trait:

```rust
#[async_trait]
pub trait Embedder: Send + Sync {
    async fn embed(&self, text: &str) -> EmbeddingResult<EmbeddingResponse>;
    fn provider_name(&self) -> &str;
    fn model_name(&self) -> &str;
    fn dimensions(&self) -> Option<usize>;
}
```

The `dimensions()` method lets downstream code validate that a stored collection's dimension matches the currently configured model before wasting compute on a mismatched embed+search cycle.

#### OpenAI

The OpenAI provider supports three models:

| Model | Dimensions | Notes |
|-------|-----------|-------|
| `text-embedding-3-small` | 1536 | MRL-trained; best cost/quality ratio |
| `text-embedding-3-large` | 3072 | MRL-trained; 2× cost, ~5% better recall |
| `text-embedding-ada-002` | 1536 | Legacy; no MRL; kept for backward compat |

Requests hit `https://api.openai.com/v1/embeddings` with `Authorization: Bearer {api_key}` and `encoding_format: "float"` so you get raw float32 values directly. The API key is resolved from `config.api_key` first, then the `OPENAI_API_KEY` environment variable — the fallback to env var is intentional for containerised deployments where you don't want secrets in config files:

```rust
let api_key = config.api_key.clone()
    .or_else(|| std::env::var("OPENAI_API_KEY").ok())
    .ok_or_else(|| EmbeddingError::ConfigError("...".into()))?;
```

The `base_url` field can override `DEFAULT_OPENAI_API_URL`. This is primarily useful for pointing Piramid at an OpenAI-compatible proxy (Azure OpenAI, LiteLLM, etc.) without changing any other config.

One thing worth knowing about `text-embedding-3-small` vs `text-embedding-3-large`: the larger model produces higher-dimensional vectors with meaningfully better semantic resolution, but at 2× the API cost and 2× the in-memory storage per vector. For most RAG use cases, `text-embedding-3-small` at full 1536 dimensions or even at 512 (via MRL truncation) hits a good cost/quality balance. The legacy `ada-002` exists for collections built before the v3 models.

#### Local HTTP and TEI

The local provider speaks to any OpenAI-compatible or TEI-style HTTP endpoint. [TEI (Text Embeddings Inference)](https://github.com/huggingface/text-embeddings-inference) is Hugging Face's high-throughput embedding server — it exposes the same `/embeddings` JSON contract as the OpenAI API, making it a drop-in replacement. [Ollama](https://ollama.com/), TEI, and other locally-hosted embedding runtimes all work as long as they implement that protocol.

A typical local setup for TEI running a 768-dimensional model:

```yaml
embeddings:
  provider: local
  model: BAAI/bge-base-en-v1.5
  base_url: http://localhost:8080/v1/embeddings
  timeout: 10
```

This matters a lot for Piramid's core thesis: if you're already running a local LLM for generation, you almost certainly want embedding to be local too. Having both on the same machine eliminates the network round-trip and keeps your documents away from remote APIs entirely. A local TEI instance on a modern CPU can embed short texts in under 5ms — comparable to OpenAI API latency but without the variance of an external network call.

> **Model alignment:** be careful about mixing embedding models across insert and search operations. If you insert documents with one model and then reconfigure the server to use a different model (different architecture, even same dimensionality), searches will produce nonsense results — the query embedding and the stored embeddings come from different spaces. Piramid doesn't currently enforce model-version locking per collection. Worth noting in production ops.


### Configuration and resolution order

```yaml
embeddings:
  provider: openai          # openai | local
  model: text-embedding-3-small
  base_url: ~               # required for local; overrides default for openai
  api_key: ~                # read from OPENAI_API_KEY env var if absent
  timeout: 30               # seconds; requests hang forever if absent
```

The resolution order for `api_key` is `config file → OPENAI_API_KEY env var → error`. Everything else comes only from the config file — there are no other environment variable overrides for `model`, `base_url`, or `timeout`. There's no per-request override for provider or model — the server uses whatever it was configured with at startup, and all collections on that server share the same embedding configuration.

The `timeout` field is important to set in production. If the embedding endpoint becomes unresponsive (OOM on the remote server, network partition, cold-start delay on a local model), Piramid's embed and search/text endpoints will block indefinitely without it. For OpenAI, 30 seconds is generous — normal latency is well under 500ms. For a local model running on CPU with a large context, you may need 60 seconds or more.


### Request flow: /embed and /search/text

When you call `POST /api/collections/{collection}/embed`, Piramid takes your text, routes it through the configured embedder stack, and stores the resulting vector. `POST /api/collections/{collection}/search/text` does the same for the query text — it's embedded on the fly and then passed into the ANN search path.

The embedder stack is layered:

```
Request
  └── RetryEmbedder          (exponential backoff on transient failures)
        └── CachedEmbedder   (LRU cache, default 10K entries)
              └── Provider   (OpenAIEmbedder or LocalEmbedder)
```

The ordering is deliberate. The cache sits *inside* the retry wrapper: if the underlying provider fails on the first attempt, the retry wrapper fires, but a subsequent cache hit will short-circuit before hitting the provider again. A cache miss falls through to the provider, and the result gets stored in the LRU on the way back up. Every layer is transparent to the caller — all three implement the same `Embedder` trait.

The cache is keyed on the exact raw text string. Identical inputs across different collections on the same server share the same cache entry. This is intentional — if the same document appears in multiple collections, the embed call is only paid once per server lifetime (until eviction). The tradeoff is that the cache is semantically unaware: "computer science" and "CS" are different keys even though they'd produce nearby vectors. A semantic-deduplication cache would be more memory-efficient for collections with near-duplicate content, but adds significant complexity.

#### Retry and error classification

The `RetryEmbedder` uses exponential backoff with: `initial_delay = 1000ms`, doubling each attempt, capped at `max_delay = 30,000ms`, `max_retries = 3`.

Not all errors are retried. The `is_retryable_error` function classifies errors before deciding:

```rust
fn is_retryable_error(error: &EmbeddingError) -> bool {
    matches!(error,
        EmbeddingError::RateLimitExceeded | EmbeddingError::RequestFailed(_))
    // AuthenticationFailed is NOT retried — retrying with a bad key is pointless
}
```

A 401 authentication failure propagates immediately — no delay, no retry. A 429 rate limit or a connection error retries with backoff. After exhausting all retries, the last error propagates up as a 5xx from the Piramid API.

> **The embedding provider is also a health endpoint.** `GET /api/health/embeddings` checks whether the configured provider is reachable and responding. A failing embedding provider will surface there before it causes query failures — worth monitoring in production alongside standard CPU/memory metrics.


### The caching tradeoff

The LRU cache holds up to 10,000 embeddings by default. Each `text-embedding-3-small` vector is $1536 \times 4 = 6{,}144$ bytes. A full 10K-entry cache occupies roughly 60MB — a manageable overhead relative to the collection's vector storage.

Cache effectiveness is entirely workload-dependent:

**High hit rate:** a RAG system where a fixed corpus is loaded at startup and then queried repeatedly. After the first pass over the corpus, every document re-embed on restart hits the cache, eliminating API calls entirely during warm-up. Effective cache size is $\min(\text{corpus size}, 10\text{K})$.

**Near-zero hit rate:** a system where every input is a unique real-time user query. Each query is distinct, the cache never hits, and the overhead is a lock acquisition per request — the `CachedEmbedder` wraps access in a `Mutex<LruCache>`. That's a few hundred nanoseconds per call, not worth worrying about.

**The eviction boundary matters:** if your hot corpus has 12,000 documents and the cache holds 10,000, the oldest 2,000 entries get continuously evicted and re-fetched. This is a surprising cliff effect — the 10K default works well for corpora that fit, and degrades suddenly for slightly larger ones. If you have a fixed corpus, set the cache size to `corpus_size + 20%` to avoid thrashing.

One thing the cache doesn't do is persist across restarts. It is in-memory only. A server restart means re-embedding your entire corpus on the first pass. For large corpora on paid embedding APIs, this can be a non-trivial cost. A persistent embedding store (write the vectors to a separate key-value store keyed by content hash) is a common production pattern to address this, but it's outside Piramid's current scope.


### Dimensions, memory, and the cost of scale

A collection with $n$ vectors of dimension $d$ stored as float32 requires exactly $4nd$ bytes.

For $n = 10^6$, $d = 1536$ (OpenAI `text-embedding-3-small`):

$$4 \times 10^6 \times 1536 = 6.144 \times 10^9 \text{ bytes} = 6.14\text{ GB}$$

For `text-embedding-3-large` at $d = 3072$ the raw storage doubles to 12.3 GB. These numbers don't include the index structure — HNSW adds roughly another 550 MB of edge storage for $10^6$ vectors at $M = 16$ (see the indexing section for the derivation). Total working memory is:

| Model | $d$ | Vectors | Raw storage | + HNSW index | Total |
|-------|-----|---------|-------------|--------------|-------|
| small | 1536 | 1M | 6.1 GB | 0.55 GB | ~6.7 GB |
| large | 3072 | 1M | 12.3 GB | 0.55 GB | ~12.8 GB |
| small | 1536 | 10M | 61 GB | 5.5 GB | ~67 GB |

These are the numbers that motivate quantisation and dimensionality reduction.

#### int8 scalar quantisation

Piramid supports int8 scalar quantisation, which compresses each float32 component to a signed int8 by linearly mapping the observed range:

$$\hat{x}_i = \text{round}\!\left(\frac{x_i - x_{\min}}{x_{\max} - x_{\min}} \times 255\right) - 128$$

Storage drops from $4d$ to $d$ bytes per vector — a 4× reduction. Quantisation error introduces a small amount of distance measurement error. The distance error for a quantised component is bounded by the quantisation step size:

$$\epsilon_q = \frac{x_{\max} - x_{\min}}{255}$$

For $\ell_2$-normalised embeddings, each component has range roughly $[-0.1, 0.1]$ (most of the variance is absorbed by the length-1 constraint spreading across 1536 dimensions), so $\epsilon_q \approx 0.0008$. The total L2 distance error accumulated over $d = 1536$ components is approximately $\sqrt{d} \cdot \epsilon_q \approx 0.031$ — small relative to typical inter-cluster distances.

> **When to use quantisation:** int8 is appropriate when recall degradation is acceptable (< 1–2 pp Recall@10 drop for most datasets) and memory is the binding constraint. It is not appropriate when precision is critical — compliance retrieval, exact deduplication, or narrow similarity thresholding. For those workloads, keep full float32.

#### MRL truncation — a cleaner memory/quality tradeoff

For `text-embedding-3` models, MRL truncation is often a better choice than quantisation for reducing memory. Truncating from 1536 to 512 dimensions is a 3× reduction with typically less recall degradation than int8 quantisation, because you're discarding the least-informative dimensions rather than uniformly degrading all of them.

The approximate recall retention curve for `text-embedding-3-small`:

| Dimensions | MTEB Recall@10 (approx) | Memory vs full |
|------------|-------------------------|----------------|
| 1536 | 100% (baseline) | 1× |
| 512 | ~97% | 3× savings |
| 256 | ~94% | 6× savings |
| 64 | ~85% | 24× savings |

Combining MRL truncation (e.g. to 512) with int8 quantisation gives a 12× memory reduction with recall typically staying above 95% — a practical configuration for memory-constrained deployments.

Piramid currently applies quantisation at the storage layer rather than at embedding time. The full float32 vector is returned by the provider and stored, and quantisation is applied when writing to disk. This means the HNSW index always operates on full-precision vectors in memory while only the persistent storage benefits from compression. Full PQ integration (operating the ANN search on compressed codes) is on the roadmap and would reduce in-memory vector storage by 32× on top of the index savings — see the indexing section for the full memory math.

