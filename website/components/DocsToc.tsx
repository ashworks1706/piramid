type Heading = {
  id: string;
  text: string;
  level: number;
};

export function DocsToc({ headings }: { headings: Heading[] }) {
  if (!headings || headings.length === 0) return null;

  return (
    <aside className="hidden xl:block w-64">
      <div className="sticky top-24 rounded-2xl border border-white/10 bg-white/5 p-4 shadow-lg shadow-slate-900/30 backdrop-blur space-y-3">
        <div className="text-xs font-semibold uppercase tracking-[0.18em] text-slate-400">On this page</div>
        <div className="space-y-1 text-sm">
          {headings.map((h) => (
            <a
              key={h.id}
              href={`#${h.id}`}
              className={`block rounded-lg px-2 py-1 text-slate-200 hover:bg-indigo-500/10 hover:text-white transition ${h.level > 2 ? "pl-4 text-xs" : ""}`}
            >
              {h.text}
            </a>
          ))}
        </div>
      </div>
    </aside>
  );
}
