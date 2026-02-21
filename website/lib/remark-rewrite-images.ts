import path from "path";

/**
 * A remark plugin factory that rewrites relative image paths in markdown files
 * to absolute public URLs served from /assets/...
 *
 * Markdown files live at e.g.  blogs/history/piramid.md and reference images
 * as  ../../assets/blogs/foo.png  (relative to the .md file).  That works fine
 * in a normal markdown preview, but Next.js looks for static files under
 * public/, not under arbitrary filesystem paths.
 *
 * The companion symlink  website/public/assets -> ../../assets  makes the
 * assets directory available to Next.js at the /assets/ URL prefix.  This
 * plugin rewrites the relative paths to that absolute URL so that Next.js
 * renders the images correctly without touching the source markdown files.
 *
 * Usage:
 *   remarkPlugins: [remarkGfm, remarkRewriteImages(blog.filePath)]
 */
export function remarkRewriteImages(markdownFilePath: string) {
  // ASSETS_DIR is the filesystem root that maps to the /assets/ URL prefix.
  const assetsDir = path.resolve(process.cwd(), "..", "assets");

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  return () => (tree: any) => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    function walk(node: any) {
      if (
        node.type === "image" &&
        typeof node.url === "string" &&
        !node.url.startsWith("http") &&
        !node.url.startsWith("/")
      ) {
        const abs = path.resolve(path.dirname(markdownFilePath), node.url);
        if (abs.startsWith(assetsDir)) {
          const rel = path.relative(assetsDir, abs);
          // Use forward slashes even on Windows (URL path)
          node.url = "/assets/" + rel.split(path.sep).join("/");
        }
      }
      if (Array.isArray(node.children)) {
        for (const child of node.children) walk(child);
      }
    }
    walk(tree);
  };
}
