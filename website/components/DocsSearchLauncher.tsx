"use client";

import Link from "next/link";
import { useEffect, useMemo, useRef, useState } from "react";
import type { BlogSearchEntry } from "../lib/blogs";

type Props = {
  entries: BlogSearchEntry[];
  className?: string;
};

export function DocsSearchLauncher({ entries, className }: Props) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setOpen(true);
        requestAnimationFrame(() => {
          inputRef.current?.focus();
        });
      }
      if (e.key === "Escape") setOpen(false);
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const results = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return entries.slice(0, 20);
    return entries
      .filter(
        (e) =>
          e.title.toLowerCase().includes(q) ||
          e.slug.join("/").toLowerCase().includes(q) ||
          e.text.toLowerCase().includes(q),
      )
      .slice(0, 30);
  }, [entries, query]);

  const close = () => setOpen(false);

  return (
    <>
      <button
        type="button"
        onClick={() => setOpen(true)}
        className={`rounded-full border border-white/10 bg-white/5 px-3 py-1.5 text-sm font-semibold text-slate-100 hover:border-indigo-300/60 hover:text-white transition ${className ?? ""}`}
      >
        Search blog <span className="ml-2 rounded bg-white/10 px-1.5 py-0.5 text-[11px] text-slate-300">⌘K</span>
      </button>

      {open ? (
        <div className="fixed inset-0 z-50 bg-black/60 backdrop-blur-sm" onClick={close}>
          <div
            className="mx-auto mt-24 w-full max-w-2xl rounded-2xl border border-white/10 bg-[#0b1020] p-4 shadow-2xl shadow-black/50"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center gap-2 rounded-lg border border-white/10 bg-black/30 px-3 py-2">
              <input
                ref={inputRef}
                type="search"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                placeholder="Search posts, titles, and content…"
                className="w-full bg-transparent text-sm text-slate-100 placeholder:text-slate-500 focus:outline-none"
              />
              <button
                onClick={close}
                className="rounded-md px-2 py-1 text-xs text-slate-400 hover:text-white transition"
              >
                Esc
              </button>
            </div>
            <div className="mt-3 max-h-80 overflow-y-auto space-y-1">
              {results.length === 0 ? (
                <div className="rounded-lg border border-white/5 bg-white/5 px-3 py-2 text-sm text-slate-400">
                  No results.
                </div>
              ) : (
                results.map((res) => {
                  const href = "/blogs/" + res.slug.join("/");
                  return (
                    <Link
                      key={href}
                      href={href}
                      onClick={close}
                      className="block rounded-lg border border-transparent px-3 py-2 text-sm text-slate-200 hover:border-indigo-400/40 hover:bg-indigo-500/10 hover:text-white transition"
                    >
                      <div className="font-semibold text-white">{res.title}</div>
                      <div className="mt-1 line-clamp-2 text-xs text-slate-400">{res.text}</div>
                      <div className="mt-1 text-[11px] text-indigo-200">{href.replace(/^\/blogs/, "") || "/"}</div>
                    </Link>
                  );
                })
              )}
            </div>
          </div>
        </div>
      ) : null}
    </>
  );
}
