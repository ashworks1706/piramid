import path from "path";

/**
 * Rewrites relative image paths in blog markdown to absolute URLs from the assets/ path segment
 * onward, at any directory depth.
 */
export function remarkRewriteImages() {
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
        // ../../assets/blogs/lsm.png becomes /assets/blogs/lsm.png.
        const normalised = node.url.split(path.sep).join("/");
        const marker = normalised.indexOf("assets/");
        if (marker !== -1) {
          node.url = "/" + normalised.slice(marker);
        }
      }
      if (Array.isArray(node.children)) {
        for (const child of node.children) walk(child);
      }
    }
    walk(tree);
  };
}
