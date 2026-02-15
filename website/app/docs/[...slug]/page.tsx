import fs from "fs";
import path from "path";
import { notFound } from "next/navigation";
import { compileMDX } from "next-mdx-remote/rsc";
import rehypeAutolinkHeadings from "rehype-autolink-headings";
import rehypeSlug from "rehype-slug";
import remarkGfm from "remark-gfm";
import { mdxComponents } from "../../../mdx-components";
import { findDoc, listDocs, extractHeadings } from "../../../lib/docs";
import { DocsToc } from "../../../components/DocsToc";

export async function generateStaticParams() {
  const docs = listDocs().filter((d) => d.slug.join("/") !== "index");
  return docs.map((d) => ({ slug: d.slug }));
}

export const runtime = "nodejs";

export default async function DocPage({ params }: { params: Promise<{ slug: string[] }> }) {
  const { slug } = await params;
  const slugArray = Array.isArray(slug) ? slug : [slug];
  const doc = findDoc(slugArray);
  if (!doc) return notFound();

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
    <div className="space-y-6 animate-fade-in">
      <div className="grid gap-8 lg:grid-cols-[minmax(0,1fr)_240px]">
        <article className="space-y-4 rounded-3xl border border-white/10 bg-gradient-to-br from-white/5 to-indigo-500/5 p-6 shadow-2xl shadow-slate-900/30 backdrop-blur">
          {frontmatter?.title ? <h1>{frontmatter.title}</h1> : null}
          {content}
        </article>
        <DocsToc headings={headings} />
      </div>
    </div>
  );
}
