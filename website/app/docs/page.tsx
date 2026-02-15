import fs from "fs";
import path from "path";
import { compileMDX } from "next-mdx-remote/rsc";
import rehypeAutolinkHeadings from "rehype-autolink-headings";
import rehypeSlug from "rehype-slug";
import remarkGfm from "remark-gfm";
import { mdxComponents } from "../../mdx-components";
import { findDoc, extractHeadings, docSeo } from "../../lib/docs";
import { DocsToc } from "../../components/DocsToc";
import type { Metadata } from "next";

export const runtime = "nodejs";

export async function generateMetadata(): Promise<Metadata> {
  const seo = docSeo(["index"]);
  const title = seo?.title ?? "Piramid docs";
  const description = seo?.description ?? "Piramid documentation.";
  const url = "/docs";
  return {
    title,
    description,
    openGraph: {
      title,
      description,
      url,
    },
    twitter: {
      title,
      description,
      card: "summary",
    },
  };
}

export default async function DocsIndex() {
  const doc = findDoc(["index"]);
  if (!doc) return null;

  const source = await fs.promises.readFile(doc.filePath, "utf8");
  const headings = extractHeadings(doc.filePath);
  const { content, frontmatter } = await compileMDX<{ title?: string }>({
    source,
    components: mdxComponents,
    options: {
      parseFrontmatter: true,
      mdxOptions: {
        remarkPlugins: [remarkGfm],
        rehypePlugins: [rehypeSlug, rehypeAutolinkHeadings],
      },
    },
  });

  return (
    <div className="space-y-6">
      <div className="grid gap-8 lg:grid-cols-[minmax(0,1fr)_240px]">
        <article className="space-y-4 animate-fade-in rounded-3xl border border-white/10 bg-gradient-to-br from-white/5 to-indigo-500/5 p-6 shadow-2xl shadow-slate-900/30 backdrop-blur">
          {frontmatter?.title ? <h1>{frontmatter.title}</h1> : <h1>Documentation</h1>}
          {content}
        </article>
        <DocsToc headings={headings} />
      </div>
    </div>
  );
}
