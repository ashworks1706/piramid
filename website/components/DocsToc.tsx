"use client";

import { useEffect, useState } from "react";

type Heading = {
  id: string;
  text: string;
  level: number;
};

export function DocsToc({ headings }: { headings: Heading[] }) {
  const [activeId, setActiveId] = useState<string | null>(null);

  useEffect(() => {
    if (!headings || headings.length === 0) return;
    const observer = new IntersectionObserver(
      (entries) => {
        const visible = entries
          .filter((e) => e.isIntersecting)
          .sort((a, b) => (b.intersectionRatio || 0) - (a.intersectionRatio || 0));
        if (visible.length > 0) {
          setActiveId(visible[0].target.id);
        }
      },
      {
        rootMargin: "0px 0px -60% 0px",
        threshold: [0, 0.2, 0.4, 0.6, 0.8, 1],
      }
    );

    headings.forEach((h) => {
      const el = document.getElementById(h.id);
      if (el) observer.observe(el);
    });

    return () => observer.disconnect();
  }, [headings]);

  if (!headings || headings.length === 0) return null;

  return (
    <aside className="hidden xl:block w-64">
      <div className="sticky top-24 rounded-2xl border border-white/10 bg-white/5 p-4 shadow-lg shadow-slate-900/30 backdrop-blur space-y-3">
        <div className="text-xs font-semibold uppercase tracking-[0.18em] text-slate-400">On this page</div>
        <div className="relative pl-3 border-l border-white/10 space-y-0.5 text-sm">
          {headings.map((h) => (
            <a
              key={h.id}
              href={`#${h.id}`}
              className={`block rounded-r-lg py-1 pr-2 transition-colors leading-snug ${
                h.level >= 4 ? "pl-6 text-xs" : h.level === 3 ? "pl-4 text-xs" : "pl-2"
              } ${
                activeId === h.id
                  ? "text-indigo-300 font-medium"
                  : "text-slate-400 hover:text-slate-100"
              }`}
            >
              {activeId === h.id && (
                <span className="absolute -left-px top-auto w-0.5 rounded-full bg-indigo-400" style={{ height: "1.25rem" }} />
              )}
              {h.text}
            </a>
          ))}
        </div>
      </div>
    </aside>
  );
}
