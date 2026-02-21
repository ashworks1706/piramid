"use client";

import Link from "next/link";
import { useEffect, useMemo, useRef, useState } from "react";
import type { BlogSearchEntry } from "../lib/blogs";

type Props = {
  entries: BlogSearchEntry[];
  className?: string;
};

/** Return up to ~160 chars of text centred around the first match of `q`. */
function buildSnippet(text: string, q: string): { before: string; match: string; after: string } | null {
  if (!q || !text) return null;
  const idx = text.toLowerCase().indexOf(q.toLowerCase());
  if (idx === -1) return null;
  const radius = 70;
  const start = Math.max(0, idx - radius);
  const end = Math.min(text.length, idx + q.length + radius);
  const before = (start > 0 ? "…" : "") + text.slice(start, idx);
  const match = text.slice(idx, idx + q.length);
  const after = text.slice(idx + q.length, end) + (end < text.length ? "…" : "");
  return { before, match, after };
}

function Highlight({ text, query }: { text: string; query: string }) {
  const idx = text.toLowerCase().indexOf(query.toLowerCase());
  if (idx === -1) return <>{text}</>;
  return (
    <>
      {text.slice(0, idx)}
      <mark className="bg-indigo-400/25 text-indigo-200 rounded px-0.5">
        {text.slice(idx, idx + query.length)}
      </mark>
      {text.slice(idx + query.length)}
    </>
  );
}

export function DocsSearchLauncher({ entries, className }: Props) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [cursor, setCursor] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setOpen(true);
        requestAnimationFrame(() => inputRef.current?.focus());
      }
      if (e.key === "Escape") {
        setOpen(false);
        setQuery("");
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const results = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return [];

    const seen = new Set<string>();
    const scored: { entry: BlogSearchEntry; score: number }[] = [];

    for (const e of entries) {
      const key = `${e.slug.join("/")}#${e.headingId ?? ""}`;
      if (seen.has(key)) continue;

      const titleHit = e.pageTitle.toLowerCase().includes(q);
      const sectionHit = e.section?.toLowerCase().includes(q);
      const textHit = e.text.toLowerCase().includes(q);

      if (!titleHit && !sectionHit && !textHit) continue;
      seen.add(key);

      const score = sectionHit ? 3 : titleHit ? 2 : 1;
      scored.push({ entry: e, score });
    }

    return scored
      .sort((a, b) => b.score - a.score)
      .slice(0, 30)
      .map((s) => s.entry);
  }, [entries, query]);

  const close = () => {
    setOpen(false);
    setQuery("");
    setCursor(0);
  };

  function onKeyDown(e: React.KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setCursor((c) => Math.min(c + 1, results.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setCursor((c) => Math.max(c - 1, 0));
    } else if (e.key === "Enter" && results[cursor]) {
      const r = results[cursor];
      const href = "/blogs/" + r.slug.join("/") + (r.headingId ? "#" + r.headingId : "");
      window.location.href = href;
      close();
    }
  }

  useEffect(() => setCursor(0), [results]);

  useEffect(() => {
    const el = listRef.current?.querySelector(`[data-idx="${cursor}"]`) as HTMLElement | null;
    el?.scrollIntoView({ block: "nearest" });
  }, [cursor]);

  const q = query.trim();

  return (
    <>
      <button
        type="button"
        onClick={() => {
          setOpen(true);
          requestAnimationFrame(() => inputRef.current?.focus());
        }}
        className={`rounded-full border border-white/10 bg-white/5 px-3 py-1.5 text-sm text-slate-300 hover:border-indigo-300/40 hover:text-white transition-colors ${className ?? ""}`}
      >
        Search
        <span className="ml-2 rounded bg-white/10 px-1.5 py-0.5 text-[11px] text-slate-400">⌘K</span>
      </button>

      {open && (
        <div className="fixed inset-0 z-50 bg-black/60 backdrop-blur-sm" onClick={close}>
          <div
            className="mx-auto mt-20 w-full max-w-2xl rounded-2xl border border-white/10 bg-[#0b1020] shadow-2xl shadow-black/60 overflow-hidden"
            onClick={(e) => e.stopPropagation()}
          >
            {/* Input */}
            <div className="flex items-center gap-2 border-b border-white/10 px-4 py-3">
              <svg className="w-4 h-4 text-slate-500 flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M21 21l-4.35-4.35M17 11A6 6 0 1 1 5 11a6 6 0 0 1 12 0z" />
              </svg>
              <input
                ref={inputRef}
                type="search"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                onKeyDown={onKeyDown}
                placeholder="Search sections and content…"
                className="w-full bg-transparent text-sm text-slate-100 placeholder:text-slate-500 focus:outline-none"
                autoComplete="off"
                spellCheck={false}
              />
              <button onClick={close} className="rounded-md px-2 py-0.5 text-xs text-slate-500 hover:text-white transition-colors">
                Esc
              </button>
            </div>

            {/* Results */}
            <div ref={listRef} className="max-h-[60vh] overflow-y-auto">
              {!q ? (
                <div className="px-4 py-10 text-center text-sm text-slate-500">
                  Type to search sections and content…
                </div>
              ) : results.length === 0 ? (
                <div className="px-4 py-10 text-center text-sm text-slate-500">
                  No results for <span className="text-slate-300">"{q}"</span>
                </div>
              ) : (
                results.map((res, i) => {
                  const href =
                    "/blogs/" + res.slug.join("/") + (res.headingId ? "#" + res.headingId : "");
                  const snippet = buildSnippet(res.text, q);
                  const isActive = i === cursor;

                  return (
                    <Link
                      key={`${href}-${i}`}
                      href={href}
                      data-idx={i}
                      onClick={close}
                      onMouseEnter={() => setCursor(i)}
                      className={`block px-4 py-3 border-b border-white/5 transition-colors ${
                        isActive ? "bg-indigo-500/15" : "hover:bg-white/[0.04]"
                      }`}
                    >
                      {/* Breadcrumb */}
                      <div className="flex items-center gap-1 text-[11px] text-slate-500 mb-1">
                        <span>{res.pageTitle}</span>
                        {res.section && <><span>/</span><span className="text-slate-400">{res.section}</span></>}
                      </div>

                      {/* Section heading or page title */}
                      {res.section ? (
                        <div className="text-sm font-medium text-slate-100 leading-snug">
                          {res.section.toLowerCase().includes(q.toLowerCase())
                            ? <Highlight text={res.section} query={q} />
                            : res.section}
                        </div>
                      ) : (
                        <div className="text-sm font-semibold text-indigo-300">{res.pageTitle}</div>
                      )}

                      {/* Text snippet with highlight */}
                      {snippet && (
                        <p className="mt-1 text-xs text-slate-500 leading-relaxed">
                          {snippet.before}
                          <mark className="bg-indigo-400/25 text-indigo-200 rounded px-0.5 not-italic">
                            {snippet.match}
                          </mark>
                          {snippet.after}
                        </p>
                      )}
                    </Link>
                  );
                })
              )}
            </div>

            {results.length > 0 && (
              <div className="border-t border-white/5 px-4 py-2 flex gap-4 text-[11px] text-slate-500">
                <span><kbd className="font-sans">↑↓</kbd> navigate</span>
                <span><kbd className="font-sans">↵</kbd> open</span>
                <span><kbd className="font-sans">Esc</kbd> close</span>
              </div>
            )}
          </div>
        </div>
      )}
    </>
  );
}
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
