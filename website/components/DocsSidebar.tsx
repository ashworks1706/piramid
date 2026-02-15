"use client";

import Link from "next/link";
import type { SidebarSection } from "../lib/docs";

type Props = {
  sections: SidebarSection[];
};

export function DocsSidebar({ sections }: Props) {
  return (
    <div className="sticky top-24 space-y-4 rounded-2xl border border-white/10 bg-white/5 p-4 shadow-lg shadow-slate-900/30 backdrop-blur">
      <div className="space-y-6">
        {sections.map((section) => (
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
        {sections.length === 0 ? (
          <div className="text-xs text-slate-400">No docs.</div>
        ) : null}
      </div>
    </div>
  );
}
