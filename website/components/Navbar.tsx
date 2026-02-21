import Image from "next/image";
import Link from "next/link";
import type { BlogSearchEntry } from "../lib/blogs";
import { DocsSearchLauncher } from "./DocsSearchLauncher";

type Props = {
  searchEntries?: BlogSearchEntry[];
};

export function Navbar({ searchEntries }: Props) {
  return (
    <header className="sticky top-0 z-20 backdrop-blur border-b border-white/5 bg-black/30">
      <div className="mx-auto flex max-w-6xl items-center justify-between px-4 sm:px-6 py-4">
        <Link href="/" className="flex items-center gap-2.5">
          <Image src="/logo_light.png" alt="Piramid" width={32} height={32} />
          <span className="text-base font-semibold tracking-wide text-white">piramid</span>
        </Link>
        <div className="flex items-center gap-4 text-sm text-slate-400">
          <Link href="/blogs" className="hover:text-white transition-colors">blog</Link>
          <a
            href="https://github.com/ashworks1706/piramid"
            className="hover:text-white transition-colors"
          >
            github
          </a>
          {searchEntries && searchEntries.length > 0 && (
            <DocsSearchLauncher entries={searchEntries} />
          )}
        </div>
      </div>
    </header>
  );
}
