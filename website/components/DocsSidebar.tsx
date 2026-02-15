"use client";

import { useMemo, useState } from "react";
import Link from "next/link";
import type { SidebarSection } from "../lib/docs";

type Props = {
  sections: SidebarSection[];
};

export function DocsSidebar({ sections }: Props) {
  const [query, setQuery] = useState("");

  const filtered = useMemo(() => {
    if (!query.trim()) return sections;
    const q = query.toLowerCase();
    return sections
      .map((section) => {
        const items = section.items.filter(
          (item) =>
            item.title.toLowerCase().includes(q) ||
            item.slug.join("/").toLowerCase().includes(q),
        );
        return { ...section, items };
      })
      .filter((section) => section.items.length > 0);
  }, [query, sections]);

  return (
    <div className="sticky top-24 space-y-4 rounded-2xl border border-white/10 bg-white/5 p-4 shadow-lg shadow-slate-900/30 backdrop-blur">
      <div className="relative">
        <input
          type="search"
          placeholder="Search docs..."
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          className="w-full rounded-lg border border-white/10 bg-black/30 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-400 focus:border-indigo-400 focus:outline-none"
        />
        <div className="pointer-events-none absolute inset-y-0 right-3 flex items-center text-xs text-slate-500">
          ⌘K
        </div>
      </div>
      <div className="space-y-6">
        {filtered.map((section) => (
          <div key={section.label} className="space-y-2">
            <div className="text-xs font-semibold uppercase tracking-[0.18em] text-slate-400">
              {section.label}
            </div>
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
        {filtered.length === 0 ? (
          <div className="text-xs text-slate-400">No matches.</div>
        ) : null}
      </div>
    </div>
  );
}
