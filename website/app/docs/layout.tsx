import "../globals.css";
import Link from "next/link";
import type { ReactNode } from "react";
import { buildSidebar } from "../../lib/docs";

export default function DocsLayout({ children }: { children: ReactNode }) {
  const sidebar = buildSidebar();

  return (
    <div className="min-h-screen bg-[#05070d] text-slate-100">
      <header className="sticky top-0 z-20 backdrop-blur border-b border-white/5 bg-black/30">
        <div className="mx-auto flex max-w-6xl items-center justify-between px-6 py-4">
          <div className="flex items-center gap-3">
            <Link href="/" className="flex items-center gap-3">
              <img src="/logo_light.png" alt="Piramid" className="h-9 w-9" />
              <div className="flex flex-col leading-tight">
                <span className="text-lg font-semibold tracking-wide">piramid</span>
                <span className="text-xs text-slate-400">Docs</span>
              </div>
            </Link>
          </div>
          <div className="flex items-center gap-3 text-sm text-slate-300">
            <Link href="/docs" className="hover:text-white transition">Docs home</Link>
            <a href="https://github.com/ashworks1706/piramid" className="hover:text-white transition">GitHub</a>
            <a href="https://crates.io/crates/piramid" className="hover:text-white transition">crates.io</a>
          </div>
        </div>
      </header>
      <main className="mx-auto flex max-w-6xl gap-8 px-6 py-10">
        <aside className="hidden lg:block w-64">
          <div className="sticky top-24 space-y-6 rounded-2xl border border-white/10 bg-white/5 p-4 shadow-lg shadow-slate-900/30 backdrop-blur">
            {sidebar.map((section) => (
              <div key={section.label} className="space-y-2">
                <div className="text-xs font-semibold uppercase tracking-[0.18em] text-slate-400">{section.label}</div>
                <div className="space-y-1">
                  {section.items.map((item) => {
                    const href = "/docs/" + item.slug.join("/");
                    return (
                      <Link
                        key={href}
                        href={href}
                        className="block rounded-lg px-3 py-2 text-sm text-slate-200 hover:bg-indigo-500/10 hover:text-white transition"
                      >
                        {item.title}
                      </Link>
                    );
                  })}
                </div>
              </div>
            ))}
          </div>
        </aside>
        <article className="flex-1">
          <div className="rounded-3xl border border-white/10 bg-gradient-to-br from-white/5 to-indigo-500/5 p-6 shadow-2xl shadow-slate-900/30 backdrop-blur">
            <div className="prose prose-invert prose-slate prose-headings:text-white prose-a:text-indigo-200 max-w-none">
              {children}
            </div>
          </div>
        </article>
      </main>
    </div>
  );
}
