import Link from "next/link";

/** Where the binaries and the client source are published. */
export const PUBLIC_REPO = "https://github.com/ashworks1706/piramid";

/** The bar across the top of the landing page. */
export function SiteNav() {
  return (
    <header className="site-nav">
      <Link href="/" className="site-nav-brand">
        PIRAMID
      </Link>
      <nav className="site-nav-links" aria-label="Main">
        <Link href="/docs">docs</Link>
        <a href={`${PUBLIC_REPO}/releases`}>releases</a>
        <a href={PUBLIC_REPO}>github</a>
      </nav>
    </header>
  );
}
