import { readFileSync } from "node:fs";
import { join } from "node:path";
import { marked } from "marked";
import { listBlogs } from "./blogs";

/** The repository root, three levels above apps/website. */
const ROOT = join(process.cwd(), "..", "..");

const read = (path: string) => readFileSync(join(ROOT, path), "utf8");

/** One thing the console can be asked. */
export type Entry = { name: string; blurb: string; lines: string[] };

/** One page the console can show, already rendered. */
export type File = { name: string; path: string; title: string; html: string };

/** What `ls` lists and `cat` prints, in the order `ls` shows them. */
const FILES: { name: string; path: string }[] = [
  { name: "readme", path: "README.md" },
  { name: "architecture", path: "docs/ARCHITECTURE.md" },
  { name: "roadmap", path: "docs/ROADMAP.md" },
  { name: "setup", path: "docs/SETUP.md" },
  { name: "config", path: "config.example.yaml" },
  { name: "contributing", path: "CONTRIBUTING.md" },
  { name: "agents", path: "AGENTS.md" },
];

/** Where a link that points inside the repository goes once it is off GitHub. */
const REPO = "https://github.com/ashworks1706/piramid";
const BLOB = `${REPO}/blob/main/`;
const RAW = "https://raw.githubusercontent.com/ashworks1706/piramid/main/";

/**
 * Repository-relative links and images, pointed back where they resolve.
 *
 * A README is written to be read on GitHub, where `docs/ARCHITECTURE.md` resolves. Served from
 * this site it resolves to nothing. A path already absolute, a fragment, or a site-root path
 * from a post's own assets is left alone.
 */
function absolute(html: string) {
  return html.replace(
    /(href|src)="([^"]+)"/g,
    (whole, attribute: string, target: string) => {
      if (/^([a-z]+:|\/\/|\/|#)/i.test(target)) {
        return whole;
      }
      const base = attribute === "src" ? RAW : BLOB;
      return `${attribute}="${base}${target.replace(/^\.?\//, "")}"`;
    },
  );
}

/** Front matter, which is metadata for the page rather than something to print. */
function withoutFrontMatter(raw: string) {
  if (!raw.startsWith("---")) {
    return raw;
  }
  const end = raw.indexOf("\n---", 3);
  return end === -1 ? raw : raw.slice(raw.indexOf("\n", end + 1) + 1);
}

/**
 * The files, as the console prints them.
 *
 * Read whole rather than summarised, so `cat` means what it says. The one thing taken out is any
 * line carrying an `<img>` tag, commented out or not: a terminal cannot show a picture, and the
 * markup around one reads as noise.
 */
export function files(): File[] {
  const render = (raw: string, rewrite: boolean) => {
    const html = marked.parse(withoutFrontMatter(raw), {
      async: false,
      gfm: true,
    });
    return rewrite ? absolute(html) : html;
  };

  const repo = FILES.map(({ name, path }) => ({
    name,
    path,
    title: path,
    html: render(read(path), true),
  }));

  // The posts, under blogs/, so a slug cannot collide with a file at the repository root. Their
  // images are already site-root paths, so nothing is rewritten.
  const posts = listBlogs().map((blog) => ({
    name: `blogs/${blog.slug.join("/")}`,
    path: `/blogs/${blog.slug.join("/")}`,
    title: blog.title,
    html: render(readFileSync(blog.filePath, "utf8"), false),
  }));

  return [...repo, ...posts];
}

/**
 * The body of one `##` section of a markdown file, with its images and its own heading dropped.
 *
 * The console answers out of the repository rather than out of a copy of it, so a section that
 * is rewritten is answered differently next build and there is nothing here to keep in step.
 */
function section(markdown: string, heading: string) {
  const lines = markdown.split("\n");
  const start = lines.findIndex((line) => line.trim() === `## ${heading}`);
  if (start === -1) {
    return [];
  }
  const rest = lines.slice(start + 1);
  const end = rest.findIndex((line) => line.startsWith("## "));
  return (end === -1 ? rest : rest.slice(0, end)).filter(
    (line) => !/<img\b/.test(line),
  );
}

/** Text with the markdown that only makes sense rendered taken back out of it. */
function plain(lines: string[]) {
  return lines
    .join("\n")
    .replace(/\[([^\]]+)\]\([^)]*\)/g, "$1")
    .replace(/[*_`]/g, "")
    .split("\n");
}

/** A paragraph as one unwrapped line, so the console can rewrap it to its own width. */
function paragraphs(lines: string[], limit: number) {
  const out: string[] = [];
  let buffer: string[] = [];
  const flush = () => {
    if (buffer.length) {
      out.push(buffer.join(" "));
      buffer = [];
    }
  };
  for (const line of lines) {
    if (!line.trim()) {
      flush();
    } else {
      buffer.push(line.trim());
    }
    if (out.length >= limit) {
      break;
    }
  }
  flush();
  return out.slice(0, limit);
}

/** Every `feature = [...]` a crate declares, without the dependency plumbing behind it. */
function features() {
  const crates = [
    "apps/engine/core",
    "apps/engine/hardware",
    "apps/engine/model",
    "apps/engine/serving",
    "apps/cli",
  ];
  const found = new Set<string>();
  for (const crate of crates) {
    const manifest = read(`${crate}/Cargo.toml`);
    const start = manifest.indexOf("\n[features]");
    if (start === -1) {
      continue;
    }
    const rest = manifest.slice(start + "\n[features]".length);
    const next = rest.indexOf("\n[");
    const block = next === -1 ? rest : rest.slice(0, next);
    for (const [, name] of block.matchAll(/^([a-z][a-z0-9-]*)\s*=/gm)) {
      if (name !== "default" && name !== "test-support") {
        found.add(name);
      }
    }
  }
  return [...found].sort();
}

/** One `key = "value"` from a manifest table. */
function field(manifest: string, key: string) {
  return (
    manifest.match(new RegExp(`^${key}\\s*=\\s*"([^"]+)"`, "m"))?.[1] ?? ""
  );
}

/** What the console knows, assembled from the repository at build time. */
export function entries(): Entry[] {
  const readme = read("README.md");
  const cargo = read("Cargo.toml");
  const members = [...cargo.matchAll(/^\s+"(apps\/[^"]+)",/gm)].map(
    ([, path]) => path,
  );

  return [
    {
      name: "about",
      blurb: "what Piramid is",
      lines: paragraphs(plain(section(readme, "What this is")), 2),
    },
    {
      name: "install",
      blurb: "how to get it",
      lines: [
        "cargo install piramid",
        "",
        "That serves collections and search. Running a model is behind two",
        "cargo features, both off by default:",
        "",
        "  cargo install piramid --features inference-candle           # CPU",
        "  cargo install piramid --features inference-candle,gpu-cuda  # CUDA",
      ],
    },
    {
      name: "console",
      blurb: "the terminal UI the binary opens",
      lines: paragraphs(plain(section(readme, "The console")), 2),
    },
    {
      name: "features",
      blurb: "the cargo features",
      lines: features().map((name) => `  ${name}`),
    },
    {
      name: "crates",
      blurb: "the workspace",
      lines: members.map((path) => `  ${path}`),
    },
    {
      name: "spec",
      blurb: "version, licence, toolchain",
      lines: [
        `  version        ${field(cargo, "version")}`,
        `  edition        ${field(cargo, "edition")}`,
        `  rust           ${field(cargo, "rust-version")} or newer`,
        `  licence        ${field(cargo, "license")}`,
        `  crates         ${members.length}`,
      ],
    },
    {
      name: "roadmap",
      blurb: "where this is going",
      lines: paragraphs(plain(section(readme, "Where this is going")), 2),
    },
    {
      name: "docs",
      blurb: "the longer documents",
      lines: [
        "  architecture   docs/ARCHITECTURE.md",
        "  roadmap        docs/ROADMAP.md",
        "  setup          docs/SETUP.md",
        "  decisions      docs/decisions/",
        "",
        "Open any of them with: open <name>",
      ],
    },
  ];
}
