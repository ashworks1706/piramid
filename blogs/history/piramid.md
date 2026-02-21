# The Evolution

I called it Piramid. I wanted to build it for [RAG](https://en.wikipedia.org/wiki/Retrieval-augmented_generation) applications where latency matters, especially in environments where demand changes fast.

> Piramid is a latency-first vector database written in Rust. The goal is to keep the database and your LLM on the same device, minimize round-trips, and expose a simple HTTP API.

![RAG pipeline — retrieve, augment, generate: the user query gets embedded, the vector DB returns relevant chunks, and the LLM generates a grounded response](https://docs.nvidia.com/nemo-framework/user-guide/24.07/_images/rag_pipeline.png)

It started as a simple idea, but I knew from day one it would be difficult. Still, I had that feeling that this was exactly the kind of project that would force me to level up. It was clearly outside my comfort zone, and that was the whole point.

I opened [Excalidraw](https://excalidraw.com) and started sketching everything I could think of: architecture, data flow, storage layout, indexing strategies, search paths, API shape, CLI flow, caching, and collection lifecycle. Then I went even broader into things like consistency, durability, backup/restore, observability, deployment, and scaling.

At every step I noticed the same thing: every decision connects to five others. Nothing in database systems is isolated. It’s deep, and everything needs to work together.

> Even though Piramid is still very early and single-node, I wanted to apply what I learned from [Designing Data-Intensive Applications](https://www.oreilly.com/library/view/designing-data-intensive-applications/9781491903063/).

#### The Journey

In the next posts I’ll go through these components in detail. I don’t want this to be just technical explanation after technical explanation. I want to include the why behind decisions, the dead ends, the tradeoffs, and the alternatives I considered before choosing a path.

Also, this is a learning project first. I’m building it to understand systems deeply and share that process. So I’m not focusing on startup topics like market analysis, competition, or funding. This section is about engineering decisions, curiosity, and the process of building.



