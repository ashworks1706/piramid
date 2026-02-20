# A Student mindset

This page is about how I got this idea and why I decided to build this project in the first place.

### Where It Started

I’ve always liked working on hard things. Challenges are the fastest way I know to become a better engineer. One of my coworkers told me to read this book and really emphasized it. I’m glad he did, because without that push I probably wouldn’t have gone down this rabbit hole.

After a lot of web dev and AI work, I finally picked up [Designing Data-Intensive Applications](https://www.oreilly.com/library/view/designing-data-intensive-applications/9781491903063/). I expected a technical handbook, but it was much more than that. It changed how I think about software engineering.

Before reading it, I had this bias that CS books were mostly syntax and patterns. Instead, this one made me see how non-deterministic systems are in practice, and how deep the field gets once you care about reliability, consistency, and scale. That’s where I really fell in love with distributed systems and lower-level engineering work.

### Rust

I didn’t have low-level language experience, so after some research I picked Rust. That ended up being one of my best decisions.

Rust taught me how much tiny implementation details matter when performance and safety both matter. The learning curve was steep, and honestly still is. I’m still learning new Rust concepts almost every day. There are often multiple valid ways to solve the same problem, which is exciting but also confusing when you’re trying to choose the best approach.

I started with smaller projects: an ecommerce backend, CLI tools, and some memory-mapped storage experiments. Those helped me get comfortable with the syntax and with Rust’s way of thinking across different use cases.

> One thing I noticed: avoiding AI early in the learning phase helped me a lot. I did use AI later in parts of Piramid for docs and some syntax fixes, but while learning fundamentals, writing code by hand and debugging it yourself made a huge difference for me.

Learning Rust felt like learning a different programming world. Ownership, borrowing, lifetimes, and memory models forced me to think differently. I also found [Rustfinity](), which helped a lot with hands-on learning. I’ve always liked practical platforms like [boot.dev](), and Rustfinity gave me that same “learn by doing” feeling.

Ownership and lifetimes were the hardest part for me. I spent a lot of time repeating tutorials and exercises until it finally clicked. Once it did, I started appreciating how clever Rust’s safety model is.

Compared to C++, Rust gave me similar low-level control but with much stronger safety guarantees. That combination was what made me stick with it.

### Databases

I knew vector databases long before this project. I had used them a lot in Python projects (*fun fact: I once met ChromaDB’s CEO at an event*), especially for [RAG projects]().

Over time I worked on more than just basic retrieval. I dealt with ranking quality, hybrid search routing, pipeline integrations, and different retrieval strategies. But all of that was from the user side, not from the “build the engine” side.

One thing I found beautiful about databases is how differently people use them. One person calls them through a Python client, another through npm, another through direct APIs in production systems. The same core engine has to serve very different needs, workloads, and expectations.

I read a lot from [qdrant](), [chroma](), [milvus](), [weaviate](), [pgvector](), and [helix](). Each one has a different design focus and tradeoff profile. Helix in particular seemed very focused on LLM and MCP-heavy workflows, which makes total sense in the current AI wave.

Seeing how differently these systems approached similar problems was super useful. It made me realize there’s no single “perfect” design, only designs that are good for specific constraints.

> Even though many of these products solve real problems really well, there are still open spaces to explore. I wasn’t thinking about differentiation in startup terms. I just wanted to build systems because I genuinely enjoy the engineering side.


### GPU kernels

At some point I kept thinking about RAG latency in real products. Even if each component is fast individually, the end-to-end flow can still feel slow: call vector DB, wait, call LLM, wait, run app logic, then return a response.

That’s when the core idea clicked for me: what if the LLM and vector retrieval could run closer together, ideally on the same device? If we can reduce cross-service round trips, latency drops a lot, especially at scale.

I started thinking about whether parts of retrieval and generation could be colocated on GPU workflows. If that works well, it could be a big win in high-volume systems where every extra hop is expensive.

Then I went deep into internals: memory management, storage layouts, index behavior, and search mechanics. That rabbit hole was intense and honestly addictive.

I also got confused by the word “kernel” because it means different things in different contexts. In deep learning classes it can mean convolution kernels, in OS it means the operating system kernel, and in GPU programming it means code executed on the GPU. Once I separated those meanings, things got clearer.

I didn’t want to jump fully into CUDA while still getting comfortable with Rust, so I started exploring Rust-friendly GPU paths (like `wgpu`) just to build intuition first.

Right now, GPU kernels are still a future step for this project. But the direction is clear, and that clarity is enough to keep building.



