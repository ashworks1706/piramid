import Link from "next/link";

export function PostCards({ children }: { children: React.ReactNode }) {
  return (
    <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 my-6">
      {children}
    </div>
  );
}

export function PostCard({
  href,
  title,
  children,
}: {
  href: string;
  title: string;
  children: React.ReactNode;
}) {
  return (
    <Link
      href={href}
      className="group block rounded-xl border border-white/8 bg-white/[0.03] px-4 py-3.5 hover:border-indigo-400/40 hover:bg-indigo-500/[0.07] transition-colors no-underline"
    >
      <div className="flex items-center justify-between gap-2">
        <span className="text-sm font-semibold text-slate-100 group-hover:text-white transition-colors">
          {title}
        </span>
        <span className="text-slate-600 group-hover:text-indigo-400 transition-colors text-sm leading-none">
          →
        </span>
      </div>
      <p className="mt-1.5 text-xs text-slate-500 leading-relaxed">{children}</p>
    </Link>
  );
}
