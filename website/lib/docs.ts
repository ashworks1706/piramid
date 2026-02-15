import "server-only";
import fs from "fs";
import path from "path";
import GithubSlugger from "github-slugger";

// Docs are read directly from the repo root ../docs
const DOCS_DIR = path.join(process.cwd(), "..", "docs");
const SIDEBAR_CONFIG = path.join(DOCS_DIR, "_sidebar.json");

export type DocMeta = {
  slug: string[];
  title: string;
  filePath: string;
};

export type DocSearchEntry = {
  slug: string[];
  title: string;
  text: string;
};

export type SidebarSection = {
  label: string;
  items: DocMeta[];
};

export type Heading = {
  id: string;
  text: string;
  level: number;
};

function isMarkdown(file: string) {
  return file.toLowerCase().endsWith(".md");
}

function readTitle(filePath: string): string {
  const raw = fs.readFileSync(filePath, "utf8");
  if (raw.startsWith("---")) {
    const end = raw.indexOf("---", 3);
    if (end !== -1) {
      const frontmatter = raw.slice(3, end).split("\n");
      const titleLine = frontmatter.find((line) => line.trim().startsWith("title:"));
      if (titleLine) {
        return titleLine.replace("title:", "").trim();
      }
    }
  }
  const heading = raw.split("\n").find((line) => line.startsWith("# "));
  if (heading) return heading.replace(/^#\s+/, "").trim();
  return path.basename(filePath).replace(/\.md$/, "");
}

function parseFrontmatter(raw: string): Record<string, string> {
  if (!raw.startsWith("---")) return {};
  const end = raw.indexOf("---", 3);
  if (end === -1) return {};
  const block = raw.slice(3, end).trim();
  const out: Record<string, string> = {};
  for (const line of block.split("\n")) {
    const [k, ...rest] = line.split(":");
    if (!k || rest.length === 0) continue;
    out[k.trim()] = rest.join(":").trim();
  }
  return out;
}

function slugFromPath(filePath: string): string[] {
  const rel = path.relative(DOCS_DIR, filePath);
  const parts = rel.split(path.sep);
  const last = parts.pop()!;
  const base = last.replace(/\.md$/, "");
  // Treat folder index.md as the folder slug (e.g., docs/foo/index.md -> /docs/foo)
  if (base === "index" && parts.length > 0) {
    return parts;
  }
  return [...parts, base];
}

let cachedDocs: DocMeta[] | null = null;
let cachedSearch: DocSearchEntry[] | null = null;

export function listDocs(): DocMeta[] {
  if (cachedDocs) return cachedDocs;
  const results: DocMeta[] = [];

  function walk(current: string) {
    const entries = fs.readdirSync(current, { withFileTypes: true });
    for (const entry of entries) {
      if (entry.name.startsWith(".")) continue;
      if (entry.isDirectory()) {
        walk(path.join(current, entry.name));
        continue;
      }
      if (!isMarkdown(entry.name)) continue;
      if (entry.name.startsWith("_")) continue; // skip meta files like _sidebar
      const filePath = path.join(current, entry.name);
      results.push({
        slug: slugFromPath(filePath),
        title: readTitle(filePath),
        filePath,
      });
    }
  }

  walk(DOCS_DIR);
  results.forEach((meta) => {
    if (meta.slug.join("/") === "index") {
      meta.title = "Overview";
    }
  });
  cachedDocs = results.sort((a, b) => a.slug.join("/").localeCompare(b.slug.join("/")));
  return cachedDocs;
}

type SidebarConfig = {
  sections: { label: string; items: string[] }[];
};

function loadSidebarConfig(): SidebarConfig | null {
  if (!fs.existsSync(SIDEBAR_CONFIG)) return null;
  try {
    const raw = fs.readFileSync(SIDEBAR_CONFIG, "utf8");
    const parsed = JSON.parse(raw) as SidebarConfig;
    if (Array.isArray(parsed.sections)) return parsed;
  } catch (_) {
    return null;
  }
  return null;
}

export function buildSidebar(): SidebarSection[] {
  const docs = listDocs();
  const config = loadSidebarConfig();
  if (!config) {
    return [
      {
        label: "Docs",
        items: docs.filter((d) => d.slug.join("/") !== "index"),
      },
    ];
  }

  const lookup = new Map(docs.map((d) => [d.slug.join("/"), d]));
  const sections: SidebarSection[] = [];
  for (const section of config.sections) {
    const items: DocMeta[] = [];
    for (const itemSlug of section.items) {
      const match = lookup.get(itemSlug);
      if (match) items.push(match);
    }
    sections.push({ label: section.label, items });
  }
  return sections;
}

export function findDoc(slug: string[]): DocMeta | null {
  const target = slug.join("/");
  return listDocs().find((d) => d.slug.join("/") === target) ?? null;
}

function stripFrontmatterAndMarkdown(raw: string): string {
  let text = raw;
  if (text.startsWith("---")) {
    const end = text.indexOf("---", 3);
    if (end !== -1) {
      text = text.slice(end + 3);
    }
  }
  text = text.replace(/```[\s\S]*?```/g, " "); // fenced code
  text = text.replace(/`[^`]*`/g, " "); // inline code
  text = text.replace(/\[([^\]]+)\]\([^)]+\)/g, "$1"); // links
  text = text.replace(/[#>*_`~\-\+]/g, " "); // markdown tokens
  text = text.replace(/\s+/g, " ").trim();
  return text;
}

export function buildSearchIndex(): DocSearchEntry[] {
  if (cachedSearch) return cachedSearch;
  cachedSearch = listDocs().map((doc) => {
    const raw = fs.readFileSync(doc.filePath, "utf8");
    const text = stripFrontmatterAndMarkdown(raw);
    return {
      slug: doc.slug,
      title: doc.title,
      text,
    };
  });
  return cachedSearch;
}

export { DOCS_DIR };

export function extractHeadings(filePath: string): Heading[] {
  const raw = fs.readFileSync(filePath, "utf8");
  const lines = raw.split("\n");
  const slugger = new GithubSlugger();
  const headings: Heading[] = [];
  let inFrontmatter = false;
  for (const line of lines) {
    if (line.trim() === "---") {
      inFrontmatter = !inFrontmatter;
      continue;
    }
    if (inFrontmatter) continue;
    const match = /^(#{1,6})\s+(.*)$/.exec(line.trim());
    if (match) {
      const level = match[1].length;
      const text = match[2].trim();
      const id = slugger.slug(text);
      headings.push({ id, text, level });
    }
  }
  return headings;
}

function summarize(raw: string): string {
  const text = stripFrontmatterAndMarkdown(raw);
  if (!text) return "";
  const snippet = text.slice(0, 220).trim();
  return snippet.length < text.length ? `${snippet}…` : snippet;
}

export function docSeo(slug: string[]): { title: string; description: string } | null {
  const doc = findDoc(slug);
  if (!doc) return null;
  const raw = fs.readFileSync(doc.filePath, "utf8");
  const frontmatter = parseFrontmatter(raw);
  const title =
    frontmatter.title ??
    (doc.slug.join("/") === "index" ? "Overview" : doc.title);
  const description = frontmatter.description ?? summarize(raw);
  return { title, description };
}
