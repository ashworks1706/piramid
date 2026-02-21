import "server-only";
import fs from "fs";
import path from "path";
import GithubSlugger from "github-slugger";

// Blog posts are read directly from the repo root ../blogs
const BLOGS_DIR = path.join(process.cwd(), "..", "blogs");
const SIDEBAR_CONFIG = path.join(BLOGS_DIR, "_sidebar.json");

export type BlogMeta = {
  slug: string[];
  title: string;
  filePath: string;
};

export type BlogSearchEntry = {
  slug: string[];
  pageTitle: string;
  /** The heading text for this section (undefined = page-level entry) */
  section?: string;
  /** Anchor fragment id so we can link directly to this heading */
  headingId?: string;
  /** Plain-text content of just this section, capped for serialisation */
  text: string;
};

export type SidebarSection = {
  label: string;
  items: BlogMeta[];
};

export type Heading = {
  id: string;
  text: string;
  level: number;
};

export type BlogNav = {
  prev?: BlogMeta;
  next?: BlogMeta;
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
  const rel = path.relative(BLOGS_DIR, filePath);
  const parts = rel.split(path.sep);
  const last = parts.pop()!;
  const base = last.replace(/\.md$/, "");
  // Treat folder index.md as the folder slug (e.g., blogs/foo/index.md -> /blogs/foo)
  if (base === "index" && parts.length > 0) {
    return parts;
  }
  return [...parts, base];
}

let cachedBlogs: BlogMeta[] | null = null;
let cachedSearch: BlogSearchEntry[] | null = null;

export function listBlogs(): BlogMeta[] {
  if (cachedBlogs) return cachedBlogs;
  const results: BlogMeta[] = [];

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

  walk(BLOGS_DIR);
  results.forEach((meta) => {
    if (meta.slug.join("/") === "index") {
      meta.title = "Overview";
    }
  });
  cachedBlogs = results.sort((a, b) => a.slug.join("/").localeCompare(b.slug.join("/")));
  return cachedBlogs;
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
  } catch {
    return null;
  }
  return null;
}

export function buildSidebar(): SidebarSection[] {
  const blogs = listBlogs();
  const config = loadSidebarConfig();
  if (!config) {
    return [
      {
        label: "Blog",
        items: blogs.filter((d) => d.slug.join("/") !== "index"),
      },
    ];
  }

  const lookup = new Map(blogs.map((d) => [d.slug.join("/"), d]));
  const sections: SidebarSection[] = [];
  for (const section of config.sections) {
    const items: BlogMeta[] = [];
    for (const itemSlug of section.items) {
      const match = lookup.get(itemSlug);
      if (match) items.push(match);
    }
    sections.push({ label: section.label, items });
  }
  return sections;
}

export function findBlog(slug: string[]): BlogMeta | null {
  const target = slug.join("/");
  return listBlogs().find((d) => d.slug.join("/") === target) ?? null;
}

export function blogNeighbors(slug: string[]): BlogNav {
  const key = slug.join("/");
  const sections = buildSidebar();
  let ordered = sections.flatMap((s) => s.items);
  if (ordered.length === 0) {
    ordered = listBlogs();
  }
  const idx = ordered.findIndex((d) => d.slug.join("/") === key);
  if (idx === -1) return { prev: undefined, next: undefined };
  return {
    prev: ordered[idx - 1],
    next: ordered[idx + 1],
  };
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
  text = text.replace(/<[^>]+>/g, " "); // HTML / JSX tags
  text = text.replace(/[#>*_`~\-\+]/g, " "); // markdown tokens
  text = text.replace(/\s+/g, " ").trim();
  return text;
}

export function buildSearchIndex(): BlogSearchEntry[] {
  if (cachedSearch) return cachedSearch;
  const slugger = new GithubSlugger();
  const entries: BlogSearchEntry[] = [];

  for (const blog of listBlogs()) {
    const raw = fs.readFileSync(blog.filePath, "utf8");

    // Strip frontmatter
    let body = raw;
    if (body.startsWith("---")) {
      const end = body.indexOf("---", 3);
      if (end !== -1) body = body.slice(end + 3);
    }

    // Split body into sections by headings
    // Each section = { headingText, headingId, rawContent }
    type Section = { headingText: string; headingId: string; raw: string };
    const headingRe = /^(#{1,6})[ \t]+(.+)$/m;
    const sections: Section[] = [];

    let remaining = body;
    slugger.reset();

    while (remaining.length > 0) {
      const match = headingRe.exec(remaining);
      if (!match) {
        // No more headings — rest is content of whatever came before
        if (sections.length > 0) {
          sections[sections.length - 1].raw += remaining;
        }
        break;
      }
      // Content before first heading goes into a preamble (no headingId)
      const before = remaining.slice(0, match.index);
      if (sections.length > 0) {
        sections[sections.length - 1].raw += before;
      }
      // Strip markdown from heading text for the display label
      const rawHeadingText = match[2]
        .replace(/\[([^\]]*?)\]\([^)]*\)/g, "$1")
        .replace(/`([^`]*)`/g, "$1")
        .replace(/\*\*([^*]*)\*\*/g, "$1")
        .replace(/\*([^*]*)\*/g, "$1")
        .trim();
      const id = slugger.slug(rawHeadingText);
      sections.push({ headingText: rawHeadingText, headingId: id, raw: "" });
      remaining = remaining.slice(match.index + match[0].length);
    }

    // Convert each section to a search entry
    for (const sec of sections) {
      const text = stripFrontmatterAndMarkdown(sec.raw).slice(0, 500);
      if (!text && !sec.headingText) continue; // skip empty
      entries.push({
        slug: blog.slug,
        pageTitle: blog.title,
        section: sec.headingText,
        headingId: sec.headingId,
        text,
      });
    }

    // Also add a page-level entry (no headingId) for title-based matching
    entries.push({
      slug: blog.slug,
      pageTitle: blog.title,
      text: stripFrontmatterAndMarkdown(body).slice(0, 300),
    });
  }

  cachedSearch = entries;
  return cachedSearch;
}

export { BLOGS_DIR };

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
      // Strip markdown link syntax: [text](url) → text, also strip backticks and bold/italic markers
      const rawText = match[2].trim();
      const text = rawText
        .replace(/\[([^\]]*?)\]\([^)]*\)/g, "$1") // [text](url) → text
        .replace(/`([^`]*)`/g, "$1")               // `code` → code
        .replace(/\*\*([^*]*)\*\*/g, "$1")         // **bold** → bold
        .replace(/\*([^*]*)\*/g, "$1")             // *italic* → italic
        .trim();
      // Slug from the plain text so it matches what rehype-slug generates
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

export function blogSeo(slug: string[]): { title: string; description: string } | null {
  const blog = findBlog(slug);
  if (!blog) return null;
  const raw = fs.readFileSync(blog.filePath, "utf8");
  const frontmatter = parseFrontmatter(raw);
  const title =
    frontmatter.title ??
    (blog.slug.join("/") === "index" ? "Overview" : blog.title);
  const description = frontmatter.description ?? summarize(raw);
  return { title, description };
}
