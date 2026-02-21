import Link from "next/link";
import { Navbar } from "../components/Navbar";

export default function Home() {
  return (
    <div className="min-h-screen bg-[#05070d] text-slate-100">
      <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_30%_10%,rgba(99,102,241,0.12),transparent_40%),radial-gradient(ellipse_at_75%_80%,rgba(14,165,233,0.07),transparent_40%)] pointer-events-none" />

      <Navbar />

      <main className="relative mx-auto max-w-3xl px-6 py-24 flex flex-col gap-20">
        {/* Hero */}
        <section className="space-y-7">
          <h1 className="text-4xl sm:text-5xl font-semibold leading-[1.15] tracking-tight text-white">
            A vector database,<br className="hidden sm:block" /> built from scratch in Rust.
          </h1>
          <p className="text-lg text-slate-400 leading-relaxed max-w-xl">
            Piramid is an open-source project I built to understand how vector databases actually work —
            the indexing algorithms, storage engine, embeddings layer, and everything in between.
          </p>
          <div className="flex flex-wrap gap-3 pt-1">
            <Link
              href="/blogs"
              className="rounded-full bg-indigo-500 text-white px-5 py-2 text-sm font-semibold shadow-lg shadow-indigo-500/25 hover:bg-indigo-400 transition-colors"
            >
              Read the blog
            </Link>
            <a
              href="https://github.com/ashworks1706/piramid"
              className="rounded-full border border-white/15 px-5 py-2 text-sm font-semibold text-slate-300 hover:border-white/40 hover:text-white transition-colors"
            >
              View on GitHub
            </a>
          </div>
        </section>

        {/* What's in the blog */}
        <section className="space-y-6">
          <h2 className="text-xs font-semibold uppercase tracking-[0.18em] text-slate-500">What's covered</h2>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            {[
              { title: "Vector databases", desc: "What they are, how they differ from relational, graph, and document stores." },
              { title: "Embeddings", desc: "Word2Vec, transformers, contrastive learning, and the geometry of embedding spaces." },
              { title: "Indexing", desc: "HNSW, IVF, flat search — the algorithms, the math, and when to use each." },
              { title: "Query engine", desc: "ANN search, metadata filtering, overfetch, and the recall/latency tradeoff." },
              { title: "Storage", desc: "mmap, WAL, checkpoints, compaction, and how durability actually works." },
              { title: "Operations", desc: "Config, health endpoints, logging, and running it in production." },
            ].map((item) => (
              <div key={item.title} className="rounded-2xl border border-white/8 bg-white/[0.03] p-5 space-y-1.5">
                <div className="text-sm font-semibold text-slate-100">{item.title}</div>
                <div className="text-sm text-slate-500 leading-relaxed">{item.desc}</div>
              </div>
            ))}
          </div>
          <Link href="/blogs" className="inline-flex text-sm text-indigo-400 hover:text-indigo-300 transition-colors">
            Browse all posts →
          </Link>
        </section>

        {/* Footer */}
        <footer className="flex flex-wrap gap-x-5 gap-y-1 pb-4 text-sm text-slate-500">
          <Link href="/blogs" className="hover:text-slate-300 transition-colors">Blog</Link>
          <a href="https://github.com/ashworks1706/piramid" className="hover:text-slate-300 transition-colors">GitHub</a>
          <a href="https://crates.io/crates/piramid" className="hover:text-slate-300 transition-colors">crates.io</a>
          <span className="ml-auto">piramid © 2026</span>
        </footer>
      </main>
    </div>
  );
}
