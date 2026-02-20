import fs from "fs";
import { compileMDX } from "next-mdx-remote/rsc";
import rehypeAutolinkHeadings from "rehype-autolink-headings";
import rehypeSlug from "rehype-slug";
import remarkGfm from "remark-gfm";
import { mdxComponents } from "../../mdx-components";
import { findBlog, extractHeadings, blogSeo, blogNeighbors } from "../../lib/blogs";
import { DocsToc } from "../../components/DocsToc";
import { DocsPager } from "../../components/DocsPager";
import type { Metadata } from "next";

export const runtime = "nodejs";

export async function generateMetadata(): Promise<Metadata> {
  const seo = blogSeo(["index"]);
  const title = seo?.title ?? "Piramid blog";
  const description = seo?.description ?? "Piramid updates, notes, and deep dives.";
  const url = "/blogs";
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

export default async function BlogsIndex() {
  const blog = findBlog(["index"]);
  if (!blog) return null;

  const source = await fs.promises.readFile(blog.filePath, "utf8");
  const headings = extractHeadings(blog.filePath);
  const nav = blogNeighbors(blog.slug);
  const docTitle = blog.title || "Overview";
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
      <DocsPager prev={nav.prev} next={nav.next} wide />
      <div className="grid gap-8 lg:grid-cols-[minmax(0,1fr)_240px]">
        <article className="space-y-4 animate-fade-in rounded-3xl border border-white/10 bg-gradient-to-br from-white/5 to-indigo-500/5 p-6 shadow-2xl shadow-slate-900/30 backdrop-blur">
          <h1>{frontmatter?.title ?? docTitle}</h1>
          {content}
        </article>
        <DocsToc headings={headings} />
      </div>
      <DocsPager prev={nav.prev} next={nav.next} wide />
    </div>
  );
}
