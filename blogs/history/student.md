# Where It Started

I’ve always liked working on hard things under constraitns, it's the fastest way I know to become a better engineer. One of my coworkers told me to read this book and really emphasized it. I’m glad he did, because without that push I probably wouldn’t have gone down this rabbit hole.

After a lot of web dev and AI work, I finally picked up [Designing Data-Intensive Applications](https://www.oreilly.com/library/view/designing-data-intensive-applications/9781491903063/).

![Designing Data-Intensive Applications by Martin Kleppmann -- the book that changed how I think about systems](https://m.media-amazon.com/images/I/91YfNb49PLL._AC_UF1000,1000_QL80_.jpg)

I loved understanding the distributed side of this book, it geniunely made me realize how beautiful engineering gets the harder the problem goes, this one made me see how non-deterministic systems are in practice, and how deep the field gets once you care about reliability, consistency, and scale. That’s where I really fell in love with distributed systems and lower-level engineering work.

### Rust

I didn’t have low-level language experience, so after some research I picked Rust. That ended up being one of my best decisions. Rust taught me how much tiny implementation details matter when performance and safety both matter. The learning curve was steep, and honestly still is. I’m still learning new Rust concepts almost every day. There are often multiple valid ways to solve the same problem, which is exciting but also confusing when you’re trying to choose the best approach.

I started with smaller projects: an ecommerce backend, CLI tools, and some memory-mapped storage experiments. Those helped me get comfortable with the syntax and with Rust’s way of thinking across different use cases.

> One thing I noticed: avoiding AI early in the learning phase helped me a lot. I did use AI later in parts of Piramid for docs and some syntax fixes, but while learning fundamentals, writing code by hand and debugging it yourself made a huge difference for me.

Learning Rust felt like learning a different programming world. Ownership, borrowing, lifetimes, and memory models forced me to think differently.

![Rust ownership -- every value has exactly one owner; when the owner leaves scope, the memory is freed without a GC](https://doc.rust-lang.org/book/img/trpl04-01.svg)
*Rust's ownership model: each value has exactly one owner, and when that owner goes out of scope the memory is freed automatically, without a garbage collector.*

I also found [Rustfinity](https://www.rustfinity.com/), which helped a lot with hands-on learning. I’ve always liked practical platforms like [boot.dev](https://boot.dev/), and Rustfinity gave me that same “learn by doing” feeling.

Ownership and lifetimes were the hardest part for me. I spent a lot of time repeating tutorials and exercises until it finally clicked. Once it did, I started appreciating how clever Rust’s safety model is.

Compared to C++, Rust gave me similar low-level control but with much stronger safety guarantees. That combination was what made me stick with it.

### Databases

I knew vector databases long before this project. I had used them a lot in Python projects, especially for [RAG projects](https://en.wikipedia.org/wiki/Retrieval-augmented_generation).

Over time I worked on more than just basic retrieval. I dealt with ranking quality, hybrid search routing, pipeline integrations, and different retrieval strategies. But all of that was from the user side, not from the “build the engine” side.

One thing I found beautiful about databases is how differently people use them. One person calls them through a Python client, another through npm, another through direct APIs in production systems. The same core engine has to serve very different needs, workloads, and expectations.

I read a lot from [qdrant](https://qdrant.tech/), [chroma](https://www.trychroma.com/), [milvus](https://milvus.io/), [weaviate](https://weaviate.io/), [pgvector](https://github.com/pgvector/pgvector), and [helix](https://helix-db.com/). Each one has a different design focus and tradeoff profile. Helix in particular seemed very focused on agentic and MCP-heavy workflows, which makes total sense in the current AI wave.

Seeing how differently these systems approached similar problems was super useful. It made me realize there’s no single “perfect” design, only designs that are good for specific constraints.

> Even though many of these products solve real problems, I wasn’t thinking about differentiation in startup terms. I just wanted to build systems because I genuinely enjoy the engineering side.


### Parallelism & performance

At some point I kept thinking about retrieval latency in real products. Even if each component is fast individually, the end-to-end flow can still feel slow: database lookup, network round-trips, and application logic all add up.

Then I went deep into internals: memory management, storage layouts, index behavior, and search mechanics. That rabbit hole was intense and honestly addictive.

I also got confused by the word “kernel” because it means different things in different contexts. In deep learning classes it can mean convolution kernels, in OS it means the operating system kernel, and in systems programming it means code executed on a compute unit. Once I separated those meanings, things got clearer.

I didn't want to jump fully into [CUDA](https://developer.nvidia.com/cuda-toolkit) while still getting comfortable with Rust, so I started exploring Rust-friendly paths (like [`wgpu`](https://wgpu.rs/)) just to build intuition first.

Right now, i think i'm still figuring it out.



