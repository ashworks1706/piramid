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
