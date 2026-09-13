# piramid-model

Model execution, the seam retrieval enters it through, and the providers that feed it.

`inference` runs Qwen2 and Qwen3 checkpoints on candle, on the CPU or a CUDA device: a driver that
runs one decoder layer at a time and calls the retrieval hook between them, a paged KV cache with
prefix sharing, a scheduler with continuous batching, chunked prefill and preemption by recompute,
sampling, the checkpoint's chat template, and streamed detokenization. `InferenceManager` loads a
model from configuration and hands out streamed generations.

`fusion` is the `RetrievalHook` seam. `HiddenState` is either a host slice or a
`DeviceBuffer`, so fusing into device memory needs no host round trip, and `launch` is separate
from `join` so retrieval can run on its own stream while the model computes.

`embeddings` turns text into vectors. Three providers: the OpenAI wire format, which covers OpenAI
itself and any server implementing it, Ollama, and `piramid`, a Qwen3 embedding checkpoint run in
this process. LRU-cached and retried.

This crate depends on nothing in the retrieval stack, which is what keeps a collection queryable
with no model loaded. A hook implementation that queries an index is a separate crate depending on
both this one and `piramid-database`.

Part of [Piramid](https://github.com/ashworks1706/piramid). See
[`docs/ARCHITECTURE.md`](../../../docs/ARCHITECTURE.md) for how the crates fit together.
