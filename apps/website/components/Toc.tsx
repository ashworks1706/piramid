"use client";

import type { Heading } from "./Readme";

/**
 * The README's own second-level headings, as a strip above it.
 *
 * Scrolling is done here rather than with an anchor because the README scrolls inside its panel,
 * not with the page: an href would move the page to the panel instead of the panel to the
 * heading.
 */
export function Toc({ headings }: { headings: Heading[] }) {
  if (!headings.length) {
    return null;
  }
  return (
    <nav className="toc" aria-label="README contents">
      <span className="toc-label">contents</span>
      {headings.map((heading) => (
        <button
          key={heading.id}
          type="button"
          className="toc-link"
          onClick={() => {
            document
              .getElementById(heading.id)
              ?.scrollIntoView({ block: "start", behavior: "smooth" });
          }}
        >
          {heading.text.toLowerCase()}
        </button>
      ))}
    </nav>
  );
}
