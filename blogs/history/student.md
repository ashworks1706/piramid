# A Student mindset

This page is just about the history and stuff how i came up with this idea and why i did this project.

### Where It Started

I always like to work on hard things and challenging things. I see challenges as a way to become better version of my self and i absolutely love to do that, How can one not? One of my coworkers at work, told me about this book and im so glad he emphasized on how important this book was to read, because honestly if he didn't i don't think i would've went on the rabbit hole.

After doing lot of web dev and ai stuff, I finally got myself to reading the book [Data Intensive Applications](https://www.oreilly.com/library/view/designing-data-intensive-applications/9781491903063/), i honestly never thought this book was gonna be so philosophical because i had this weird previous bias that CS books were all about syntaxes and programming paradigms, but how deep and philosophical this book was really trasnforemd by entire perspective about software engineering, how beautiful it is than what media says. i realized how non deterministic are computers and how deep the actual SWE field is than what just media portrays, I really fell in love with distributed systems and the low level dirty work. I honestly did not expect this project to be such an exciting, one in a million experience kind of thing.

### Rust

But i was not aware of any low level langauges or had experiecne with one, after some research i just picked up rust. and that has to be one of my best decisions ever. learning rust has taught me sooo much about how really low level tiny details are controlled and manipulated in order to suck out most of the performance, how unsolved are these kinds of problems, the entire steep learning curve of rust was very difficult to get past to, I'm still daily learning about new concepts about rust and it honestly seems like it's that langauge that can be studied forever. There's ton of ways to do some X thing in rust, and to me that was fascinating yet frustating at times, because sometimes i was just confused which method to pick to do this in the best way possible. 

I build some basic rust projects with ecommerce backend, CLI, memory map storages, bascially here i got familiar with syntax and how on variety of usecases, rust runs differently than others. these were core projects.

> One key thing i noticed here that not using AI was the best decision in the learning process. Thouhg i did use some AI in part of piramid to get the documentation and syntax fixes, but overall, AI is definitely not recommended for learning purposes, writing code by hand is so important that something cannot go unnoticed no matter whatever the media says. I just used stack overflow or google during my time of these core projects. AI makes you weak if u use it too early.

Learning rust was very challenging, it was almost like i'm learning a different variety of programming world, no OOP principles and a different way of managing objects, instances and allocations. I'm so glad i found about this platform called [Rustfinity](), this was absolutely what i was looking for, I always had been a fan of hands on learning something like [boot.dev]() but was trying so hard to find with Rust. Since there's minority of rust platforms in terms of resources, it's mainlky the rust book from the official documentation, i was completley hooked to rustfinity, such a clean UI, beautiful done software and it never lagged, the way the concepts were explained there really solidifed my learning process. One of my biggest doubts about rust was that it lifetime and ownerships, and boy i spent a lot of time just going through tutorials nad just geting familair with it. It really opened by eyes and just such a clever way to writing programming languages.

One other thing i noticed about rust and the different from c++ was that, rust was delivering the same independence of manipulation and toying around but with a safety a clever independent safety mechanism that really put me in awe and love with rust. such lot of effort and time has been put into this language, its a complete lagnauge perhaps.

### Databases

I knew vector databases existed a long time ago, i've used them several times (*fun fact: i met chroma db's ceo one time at an event*) with my python applications especially [RAG projects](), for example, some agentic AI application for getting semantic data from a text query, but not just basic use cases, i've went into great depth of managing rankings, searches, hybrid routers, data pipeline integrations and several SoTA retreival strategies. But they were all from a different perspective.

one of the cool things i came to know about databases was like, how differently can anyone use it, there can be a user side using a python wrapper client to call that maine core engine, and that actually does the business logic, or there's some npm guy using it, there's just different ways a database is required to work which is so beautiful about engineering. there could even be tons of business using it at teh same time, so the expectations are always very random, it's supposed to scale as needed and fully shaped inorder to be customized.

I read a lot of blogs from [qdrant]() --- how standardized and stable their approach it to vector db, [chroma]() --- how ... , [milvus]() --- , [weaviate](), [pgvector]() and [helix]() -- helix seemed to focus mainly on llms and mcps, which were honestly totally understandable in the current boom of AI, it's super attractive and useful, i noticed how they were tackling vector dbs in their own unique ways, how each of them focuses on specific problem and delivers an elegant solution.

> Even though a lot of them were solving problems, there will still lot of solutions that were left to be unwrapped, I did not think a lot of about how i could be different because i barely cared about the startup side of this, i just wanted to build a distributed system just cause i loved the engineering side of it.


### GPU kernels

Though one day i just thought to myself about RAG solutions out there, since i have closest experience with rag when it comes to vector databases, i noticed how calling each vector db api call, waiting for it, then using llm api call, and then finally responding to an average user is so slow, no matter how fast each could be because theres always some business solution in between that the application wants to do.

which is when i thought, what if we could keep LLMs and RAG on teh same vector db? because i noticed vector dbs worked kind of like similarly to llms, llms have [sinsonidal embeddings]() (though they're replaced with [RoPE]() now), they seem to have the same dot product cosine similarity, so i thought if we could fit them on same gpu, and compute it like in one forward pass, we could definitely get rid of two round trip api calls as latency is expensive in big applications, if it's 100M vectors and especially with HNSW searches where pruning happens, it could be a game changer. 

I dove down more into how actually on low level dbs managed their memory, how they stored data, how they managed their indexes, how they did their searches, and i realized that there was a lot of work to be done in order to get to that point, but it was such an exciting rabbit hole to go down. I had no idea about how databases were actually built, i just knew how to use them, but now i was learning about the internals of databases, the storage management, the indexing strategies, the search algorithms, and it was such a fascinating journey. GPU operations happened with Kernels. I was so confused about the term, kernels were used in so many different terminologies, at my DAT 494 advanced deep learning class --- he meant convolution neural network kernels, at quantum club --- he meant quantum kernels, and in OS, there were next level kernels, super confusing but i got through, GPU kernels were basically the way to run code on the GPU, they were like functions that could be executed on the GPU, and they were written in a specific language called CUDA. I had to learn about CUDA and how to write GPU kernels, but i did not wanted to learn CUDA so fast given i was learning rust, so i just thought im gonna do a rust based gpu kernel with wgpu, to still get the gist of it.

So far im still thinking about gpu kernls and its a future thing. but the vision is kinda clear.



