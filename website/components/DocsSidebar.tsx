"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import type { SidebarSection } from "../lib/blogs";

type Props = {
  sections: SidebarSection[];
  sticky?: boolean;
  className?: string;
};

export function DocsSidebar({ sections, sticky = true, className = "" }: Props) {
  const pathname = usePathname();
  const hrefForSlug = (slugParts: string[]) => {
    const slugPath = slugParts.join("/");
    return slugPath === "index" ? "/blogs" : "/blogs/" + slugPath;
  };

  return (
    <div
      className={`${sticky ? "sticky top-24" : ""} space-y-4 rounded-2xl border border-white/10 bg-white/5 p-4 shadow-lg shadow-slate-900/30 backdrop-blur ${className}`}
    >
      <div className="space-y-6">
        {sections.map((section) => (
          <div key={section.label} className="space-y-2">
            {section.items.length > 0 ? (
              <Link
                href={hrefForSlug(section.items[0].slug)}
                className="text-xs font-semibold uppercase tracking-[0.18em] text-slate-400 hover:text-white"
              >
                {section.label}
              </Link>
            ) : (
              <div className="text-xs font-semibold uppercase tracking-[0.18em] text-slate-400">
                {section.label}
              </div>
            )}
            <div className="space-y-1">
              {section.items.map((item) => {
                const href = hrefForSlug(item.slug);
                const label = item.slug.join("/") === "index" ? "Overview" : item.title;
                const isActive = pathname === href;
                return (
                  <Link
                    key={href}
                    href={href}
                    className={`block rounded-lg px-3 py-2 text-sm transition ${
                      isActive
                        ? "bg-indigo-500/20 text-white font-semibold ring-1 ring-inset ring-indigo-400/30"
                        : "text-slate-200 hover:bg-indigo-500/10 hover:text-white"
                    }`}
                  >
                    {label}
                  </Link>
                );
              })}
            </div>
          </div>
        ))}
        {sections.length === 0 ? (
          <div className="text-xs text-slate-400">No posts.</div>
        ) : null}
      </div>
    </div>
  );
}
