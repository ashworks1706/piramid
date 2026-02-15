import Image from "next/image";

const features = [
  {
    title: "Latency-first",
    detail: "Mmap storage, cached vectors/metadata, WAL + checkpoints. Built for low tail latency in agentic pipelines.",
  },
  {
    title: "Index choice",
    detail: "Flat, HNSW, IVF with per-request overrides (ef/nprobe/filter_overfetch) and filter-aware search.",
  },
  {
    title: "Embeddings",
    detail: "OpenAI and local HTTP (Ollama/TEI-style) with caching + retries. Single embed endpoint for single/batch.",
  },
  {
    title: "Guardrails",
    detail: "Limits on vectors/bytes, disk low-space read-only mode, cache caps, tracing + metrics/health endpoints.",
  },
  {
    title: "One binary",
    detail: "Install via cargo, run `piramid serve`, ship. CLI also generates configs and prints resolved settings.",
  },
  {
    title: "Roadmap: GPU co-location",
    detail: "Future Zipy kernel to co-locate vector search with the LLM on the same GPU—no CPU round-trips.",
  },
];

const quickstart = `cargo install piramid
piramid init --path piramid.yaml
piramid serve --data-dir ./data

# Insert
curl -X POST http://localhost:6333/api/collections/docs/vectors \\
  -H "Content-Type: application/json" \\
  -d '{"vector":[0.1,0.2,0.3,0.4],"text":"hello","metadata":{"cat":"demo"}}'

# Search
curl -X POST http://localhost:6333/api/collections/docs/search \\
  -H "Content-Type: application/json" \\
  -d '{"vector":[0.1,0.2,0.3,0.4],"k":5}'`;

export default function Home() {
  const jsonLd = {
    "@context": "https://schema.org",
    "@type": "SoftwareApplication",
    name: "Piramid",
    applicationCategory: "DatabaseApplication",
    description: "Rust vector database for agentic workloads, built for low latency and GPU co-location.",
    operatingSystem: "Cross-platform",
    offers: {
      "@type": "Offer",
      price: "0",
      priceCurrency: "USD",
    },
    author: {
      "@type": "Person",
      name: "ashworks1706",
    },
    programmingLanguage: "Rust",
    codeRepository: "https://github.com/ashworks1706/piramid",
  };

  return (
    <>
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLd) }}
      />
      <div className="min-h-screen bg-gradient-to-b from-[#05070d] via-[#0b1020] to-[#05070d] text-slate-100">
        <div className="absolute inset-0 bg-[radial-gradient(circle_at_20%_20%,rgba(99,102,241,0.14),transparent_30%),radial-gradient(circle_at_80%_10%,rgba(14,165,233,0.12),transparent_25%),radial-gradient(circle_at_50%_80%,rgba(56,189,248,0.08),transparent_25%)] pointer-events-none" />
        <div className="relative">
          {/* Nav */}
          <header className="sticky top-0 z-20 backdrop-blur border-b border-white/5 bg-black/30">
            <div className="mx-auto flex max-w-6xl items-center justify-between px-6 py-4">
              <div className="flex items-center gap-3">
                <Image src="/logo_light.png" alt="Piramid" width={40} height={40} />
                <div className="flex flex-col leading-tight">
                  <span className="text-lg font-semibold tracking-wide">piramid</span>
                  <span className="text-xs text-slate-400">Vector Database for agentic systems</span>
                </div>
              </div>
              <div className="flex items-center gap-3">
                <a
                  href="/docs"
                  className="text-sm text-slate-300 hover:text-white transition"
                >
                  docs
                </a>
                <a
                  href="https://crates.io/crates/piramid"
                  className="text-sm text-slate-300 hover:text-white transition"
                >
                  crates.io
                </a>
                <a
                  href="https://github.com/ashworks1706/piramid"
                  className="rounded-full bg-white text-black px-4 py-2 text-sm font-semibold shadow-lg shadow-indigo-500/20 hover:bg-slate-100 transition"
                >
                  GitHub
                </a>
              </div>
            </div>
          </header>

          {/* Hero */}
          <main className="relative mx-auto flex max-w-6xl flex-col gap-16 px-6 py-14">
            <div className="absolute inset-x-0 -top-10 md:-top-16 lg:-top-20 h-[420px] md:h-[500px] lg:h-[540px] -z-10">
              <div className="rounded-[32px] w-full h-full bg-[radial-gradient(circle_at_50%_20%,rgba(99,102,241,0.18),rgba(14,165,233,0.08)),linear-gradient(160deg,rgba(24,27,42,0.8),rgba(10,12,24,0.85))] border border-white/10 shadow-2xl shadow-indigo-900/30 blur-[0px]" />
            </div>
            <section className="grid gap-10 md:grid-cols-[1.1fr_0.9fr] items-center">
              <div className="space-y-6 fade-in">
                <p className="inline-flex items-center gap-2 rounded-full border border-white/10 bg-white/5 px-3 py-1 text-xs font-semibold uppercase tracking-[0.2em] text-slate-300">
                  GPU Powered
                </p>
                <h1 className="text-4xl md:text-5xl font-semibold leading-tight text-white">
                  Vectors, Tokens. One Device.
                </h1>
             
                <div className="flex flex-wrap gap-3">
                  <a
                    href="https://crates.io/crates/piramid"
                    className="rounded-full bg-indigo-400 text-black px-5 py-2 text-sm font-semibold shadow-lg shadow-indigo-500/30 hover:bg-indigo-300 transition"
                  >
                    Install via Cargo
                  </a>
                  <a
                    href="https://github.com/ashworks1706/piramid"
                    className="rounded-full border border-white/15 px-5 py-2 text-sm font-semibold text-white hover:border-white/40 transition"
                  >
                    View on GitHub
                  </a>
                </div>
                <div className="flex flex-wrap gap-4 text-xs text-slate-400">
                  <span className="rounded-full bg-white/5 px-3 py-1 border border-white/10">HNSW • IVF • Flat</span>
                  <span className="rounded-full bg-white/5 px-3 py-1 border border-white/10">Filters + metadata cache</span>
                  <span className="rounded-full bg-white/5 px-3 py-1 border border-white/10">Embeddings: OpenAI + local HTTP</span>
                  <span className="rounded-full bg-white/5 px-3 py-1 border border-white/10">WAL + checkpoints</span>
                </div>
              </div>

              <div className="relative space-y-4 fade-in delay-1">
                <div className="rounded-2xl border border-white/10 bg-black/50 shadow-2xl shadow-indigo-600/20 p-6 backdrop-blur">
                  <div className="flex items-center justify-between text-xs text-slate-400 mb-3">
                    <span>Run in 30 seconds</span>
                    <span className="text-emerald-300 font-semibold">live</span>
                  </div>
                  <pre className="code-block">
                    <code>{quickstart}</code>
                  </pre>

                </div>
              </div>
            </section>

            {/* Features grid */}
            <section className="space-y-6 fade-in delay-2">
              <div>
                <h2 className="text-3xl font-semibold text-white">Built for infra teams and latency-sensitive agents</h2>
              </div>
              <div className="grid gap-4 md:grid-cols-2">
                {features.map((f) => (
                  <div
                    key={f.title}
                    className="rounded-2xl border border-white/10 bg-white/5 p-5 backdrop-blur-sm shadow-lg shadow-slate-900/40 hover:border-indigo-400/40 transition"
                  >
                    <h3 className="text-lg font-semibold text-white">{f.title}</h3>
                    <p className="mt-2 text-sm text-slate-300 leading-relaxed">{f.detail}</p>
                  </div>
                ))}
              </div>
            </section>

            {/* Architecture / vision */}
            <section className="grid gap-8 lg:grid-cols-2 items-start fade-in delay-3">
              <div className="rounded-2xl border border-white/10 bg-gradient-to-br from-white/5 to-indigo-500/10 p-6 shadow-xl shadow-indigo-900/30">
                <p className="text-sm uppercase tracking-[0.24em] text-slate-400">Architecture</p>
                <h3 className="text-2xl font-semibold text-white mt-2">Current path</h3>
                <ul className="mt-4 space-y-3 text-sm text-slate-200 leading-relaxed">
                  <li>Axum server + Rust core; single binary CLI (`piramid`).</li>
                  <li>Storage: mmap-backed data, WAL + checkpoints, sidecar indexes.</li>
                  <li>Indexes: Flat/HNSW/IVF with cached vectors/metadata; filter-aware search.</li>
                  <li>Embeddings: OpenAI/local HTTP with retry + cache; unified embed endpoint (single/batch).</li>
                  <li>Guardrails: limits, disk low-space read-only mode, cache caps, tracing + metrics/health.</li>
                </ul>
              </div>
              <div className="rounded-2xl border border-white/10 bg-gradient-to-br from-white/5 to-cyan-500/10 p-6 shadow-xl shadow-cyan-900/30">
                <p className="text-sm uppercase tracking-[0.24em] text-slate-400">Roadmap</p>
                <h3 className="text-2xl font-semibold text-white mt-2">GPU co-location (Zipy)</h3>
                <ul className="mt-4 space-y-3 text-sm text-slate-200 leading-relaxed">
                  <li>Co-locate vector search and the LLM on the same GPU kernel to remove CPU hops.</li>
                  <li>GPU-aware index strategies and memory layout tuned for RAG/agent loops.</li>
                  <li>Retain the same API/CLI surface; swap execution backend when GPU is available.</li>
                </ul>
              </div>
            </section>

            {/* Footer */}
            <footer className="flex flex-col gap-3 pb-10 text-sm text-slate-400">
              <div className="flex gap-4">
                <a href="/docs" className="hover:text-white transition">Docs</a>
                <a href="https://github.com/ashworks1706/piramid" className="hover:text-white transition">GitHub</a>
                <a href="https://crates.io/crates/piramid" className="hover:text-white transition">crates.io</a>
                <a href="https://github.com/ashworks1706/piramid/blob/main/docs/TODO.md" className="hover:text-white transition">Roadmap</a>
              </div>
              <p>piramid @ 2026</p>
            </footer>
          </main>
        </div>
      </div>
    </>
  );
}
