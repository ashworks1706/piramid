export default function NotFound() {
  return (
    <div className="min-h-screen flex items-center justify-center bg-[#05070d] text-slate-100">
      <div className="rounded-3xl border border-white/10 bg-white/5 px-8 py-10 shadow-2xl shadow-slate-900/40 text-center">
        <div className="text-2xl font-semibold">Not found</div>
        <p className="mt-3 text-sm text-slate-300">That page does not exist.</p>
        <a href="/" className="mt-6 inline-flex items-center rounded-full bg-indigo-400 px-4 py-2 text-sm font-semibold text-black shadow-lg shadow-indigo-500/30 hover:bg-indigo-300 transition">Back home</a>
      </div>
    </div>
  );
}
