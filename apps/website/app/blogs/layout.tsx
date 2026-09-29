import "../globals.css";
import type { ReactNode } from "react";
import Link from "next/link";
import { DocsSearchLauncher } from "../../components/DocsSearchLauncher";
import { buildSearchIndex } from "../../lib/blogs";

export default function BlogsLayout({ children }: { children: ReactNode }) {
  const searchEntries = buildSearchIndex();

  return (
    <div className="page">
      <section className="window">
        <header className="readme-bar">
          <span className="readme-dot" />
          <span className="readme-dot" />
          <span className="readme-dot" />
          <Link href="/" className="readme-name window-home">
            /piramid
          </Link>
          <span className="readme-name window-path">/blogs</span>
          <span className="console-bar-links">
            {searchEntries.length > 0 && (
              <DocsSearchLauncher entries={searchEntries} />
            )}
            <a href="https://github.com/ashworks1706/piramid">github</a>
          </span>
        </header>
        <div className="window-body readme-prose">{children}</div>
      </section>
    </div>
  );
}
