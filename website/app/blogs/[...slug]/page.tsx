import fs from "fs";
import { notFound } from "next/navigation";
import { compileMDX } from "next-mdx-remote/rsc";
import rehypeKatex from "rehype-katex";
import rehypeSlug from "rehype-slug";
import remarkGfm from "remark-gfm";
import remarkMath from "remark-math";
import { mdxComponents } from "../../../mdx-components";
import { findBlog, listBlogs, extractHeadings, blogSeo, blogNeighbors } from "../../../lib/blogs";
import { remarkRewriteImages } from "../../../lib/remark-rewrite-images";
import { DocsToc } from "../../../components/DocsToc";
import { DocsPager } from "../../../components/DocsPager";
import type { Metadata } from "next";

export async function generateStaticParams() {
  const blogs = listBlogs().filter((b) => b.slug.join("/") !== "index");
  return blogs.map((b) => ({ slug: b.slug }));
}

export const runtime = "nodejs";

export async function generateMetadata({ params }: { params: Promise<{ slug: string[] }> }): Promise<Metadata> {
  const { slug } = await params;
  const slugArray = Array.isArray(slug) ? slug : [slug];
  const seo = blogSeo(slugArray);
  const pageTitle = seo?.title ?? slugArray[slugArray.length - 1]
    .replace(/-/g, " ")
    .replace(/\b\w/g, (c) => c.toUpperCase());
  const fullTitle = `${pageTitle} | Piramid`;
  const description = seo?.description ?? "Piramid blog.";
  const url = `/blogs/${slugArray.join("/")}`;
  return {
    title: pageTitle,
    description,
    openGraph: { title: fullTitle, description, url },
    twitter: { title: fullTitle, description, card: "summary" },
  };
}

export default async function DocPage({ params }: { params: Promise<{ slug: string[] }> }) {
  const { slug } = await params;
  const slugArray = Array.isArray(slug) ? slug : [slug];
  const blog = findBlog(slugArray);
  if (!blog) return notFound();

  const source = await fs.promises.readFile(blog.filePath, "utf8");
  const headings = extractHeadings(blog.filePath);
  const nav = blogNeighbors(blog.slug);
  const { content } = await compileMDX<{ title?: string }>({
    source,
    components: mdxComponents,
    options: {
      parseFrontmatter: true,
      mdxOptions: {
        remarkPlugins: [remarkGfm, remarkMath, remarkRewriteImages(blog.filePath)],
        rehypePlugins: [rehypeSlug, rehypeKatex],
      },
    },
  });

  return (
    <div className="space-y-6 animate-fade-in">
      <DocsPager prev={nav.prev} next={nav.next} wide />
      <div className="grid gap-8 lg:grid-cols-[minmax(0,1fr)_240px]">
        <article className="space-y-4 rounded-3xl border border-white/10 bg-linear-to-br from-white/5 to-indigo-500/5 p-6 shadow-2xl shadow-slate-900/30 backdrop-blur">
          {content}
        </article>
        <DocsToc headings={headings} />
      </div>
      <DocsPager prev={nav.prev} next={nav.next} wide />
    </div>
  );
}
