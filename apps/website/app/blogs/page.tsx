import fs from "fs";
import Link from "next/link";
import { compileMDX } from "next-mdx-remote/rsc";
import remarkGfm from "remark-gfm";
import type { Metadata } from "next";
import { mdxComponents } from "../../mdx-components";
import { blogSeo, buildSidebar, findBlog } from "../../lib/blogs";
import { remarkRewriteImages } from "../../lib/remark-rewrite-images";

export const runtime = "nodejs";

export async function generateMetadata(): Promise<Metadata> {
  const seo = blogSeo(["index"]);
  const pageTitle = seo?.title ?? "Blogs";
  const description = seo?.description ?? "Piramid updates, notes, and deep dives.";
  return {
    title: pageTitle,
    description,
    openGraph: { title: `${pageTitle} | Piramid`, description, url: "/blogs" },
    twitter: { title: `${pageTitle} | Piramid`, description, card: "summary" },
  };
}

export default async function BlogsIndex() {
  const blog = findBlog(["index"]);
  if (!blog) return null;

  const sections = buildSidebar();

  const source = await fs.promises.readFile(blog.filePath, "utf8");
  const { content } = await compileMDX({
    source,
    components: mdxComponents,
    options: {
      parseFrontmatter: true,
      mdxOptions: { remarkPlugins: [remarkGfm, remarkRewriteImages()] },
    },
  });

  return (
    <div className="blog-index animate-fade-in">
      <div className="blog-index-intro">{content}</div>

      <nav className="blog-index-sections">
        {sections.map((section) => (
          <section key={section.label} className="blog-index-section">
            <h2 className="blog-index-label">{section.label}</h2>
            <ul className="blog-index-list">
              {section.items.map((item) => (
                <li key={item.slug.join("/")}>
                  <Link href={`/blogs/${item.slug.join("/")}`} className="blog-index-item">
                    <span className="blog-index-arrow" aria-hidden="true">
                      &rsaquo;
                    </span>
                    <span className="blog-index-title">{item.title}</span>
                    <span className="blog-index-path">
                      /{item.slug.join("/")}
                    </span>
                  </Link>
                </li>
              ))}
            </ul>
          </section>
        ))}
      </nav>
    </div>
  );
}
