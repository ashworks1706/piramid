import type { Metadata } from "next";
import Link from "next/link";
import { docs } from "../../lib/docs";

export const metadata: Metadata = {
  title: "Docs",
  description:
    "Running piramid serve: builds, configuration, embedding providers and the HTTP API.",
  alternates: { canonical: "/docs" },
};

export default function Docs() {
  const { html, headings } = docs();
  return (
    <main className="docs">
      <nav className="docs-nav" aria-label="Contents">
        <Link href="/" className="docs-home">
          piramid
        </Link>
        {headings.map(({ id, text }) => (
          <a key={id} href={`#${id}`} className="docs-link">
            {text}
          </a>
        ))}
      </nav>
      <article
        className="docs-body"
        dangerouslySetInnerHTML={{ __html: html }}
      />
    </main>
  );
}
