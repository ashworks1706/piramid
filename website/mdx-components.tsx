import type { MDXComponents } from "mdx/types";

const Callout = ({ title, children }: { title: string; children: React.ReactNode }) => (
  <div className="rounded-xl border border-indigo-400/30 bg-indigo-500/10 px-4 py-3 text-slate-100 shadow-lg shadow-indigo-900/30">
    <div className="text-sm font-semibold text-indigo-200">{title}</div>
    <div className="mt-1 text-sm text-slate-200">{children}</div>
  </div>
);

export const mdxComponents: MDXComponents = {
  Callout,
  h1: (props) => <h1 className="text-4xl font-semibold tracking-tight text-white" {...props} />,
  h2: (props) => <h2 className="mt-10 text-3xl font-semibold tracking-tight text-white" {...props} />,
  h3: (props) => <h3 className="mt-8 text-2xl font-semibold text-slate-100" {...props} />,
  h4: (props) => <h4 className="mt-6 text-xl font-semibold text-slate-100" {...props} />,
};
