# Embeddings

TODO cover:
- Providers: OpenAI and local HTTP (Ollama/TEI style); how provider/model/base_url/api_key/timeout are resolved.
- Request flow for /embed and /search/text; retry/backoff and caching layers.
- Token/count/cost metrics (planned) and timeout behavior.
- How embedding configs are stored vs. per-request overrides (if any).
- Future GPU co-location story with Zipy kernel and LLM on the same device.
