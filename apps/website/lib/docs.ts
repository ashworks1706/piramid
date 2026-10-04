import { readFileSync } from "node:fs";
import { join } from "node:path";
import { marked } from "marked";

/** One heading of the docs, for the contents list. */
export type Heading = { id: string; text: string };

/** The user docs, rendered, with the second-level headings they link to. */
export type Docs = { html: string; headings: Heading[] };

/** An id for a heading, the way GitHub spells one. */
function slug(text: string) {
  return text
    .toLowerCase()
    .replace(/[^a-z0-9 -]/g, "")
    .trim()
    .replace(/ +/g, "-");
}

/** The user docs from content/docs.md, rendered at build time. */
export function docs(): Docs {
  const raw = readFileSync(join(process.cwd(), "content", "docs.md"), "utf8");
  const headings: Heading[] = [];
  const renderer = new marked.Renderer();
  renderer.heading = ({ tokens, depth }) => {
    const text = tokens.map((token) => token.raw).join("").replace(/`/g, "");
    const id = slug(text);
    if (depth === 2) {
      headings.push({ id, text });
    }
    const inner = marked.Parser.parseInline(tokens);
    return `<h${depth} id="${id}">${inner}</h${depth}>`;
  };
  const html = marked.parse(raw, { async: false, gfm: true, renderer });
  return { html, headings };
}
