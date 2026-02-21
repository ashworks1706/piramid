import "../globals.css";
import type { ReactNode } from "react";
import { DocsSidebar } from "../../components/DocsSidebar";
import { DocsSidebarMobile } from "../../components/DocsSidebarMobile";
import { Navbar } from "../../components/Navbar";
import { buildSidebar, buildSearchIndex } from "../../lib/blogs";

export default function BlogsLayout({ children }: { children: ReactNode }) {
  const sidebar = buildSidebar();
  const searchEntries = buildSearchIndex();

  return (
    <div className="min-h-screen bg-[#05070d] text-slate-100">
      <Navbar searchEntries={searchEntries} />
      <main className="mx-auto flex max-w-6xl flex-col gap-6 px-4 sm:px-6 py-8 lg:flex-row lg:gap-8 lg:py-10">
        <DocsSidebarMobile sections={sidebar} />
        <aside className="hidden lg:block w-64 flex-shrink-0">
          <DocsSidebar sections={sidebar} />
        </aside>
        <article className="flex-1">{children}</article>
      </main>
    </div>
  );
}
