import { CliLogo } from "../components/CliLogo";
import { InstallTabs } from "../components/InstallTabs";
import { SiteNav } from "../components/SiteNav";

const EXAMPLE = `from piramid import Piramid

db = Piramid("http://localhost:6333")
db.embed("notes", ["Compaction rewrites the store without deleted documents."])

answer = db.generate(
    messages=[{"role": "user", "content": "What happens to deleted documents?"}],
    collection="notes",
    k=2,
)
print(answer["text"])`;

export default function Home() {
  return (
    <>
      <SiteNav />
      <main className="landing">
        <div className="landing-inner">
          <CliLogo />
          <p className="landing-tagline">
            Inference runtime + vector database
          </p>
          <p className="landing-copy">
            An experimental LLM inference runtime and vector database written in Rust. Execute supported models, store documents and embeddings, and search durable collections.
          </p>
          <p className="landing-copy">
            Systems work focuses on model execution, GPU memory, KV-cache management, and scheduling. Retrieval during generation is an experimental direction.
          </p>
          <InstallTabs />
          <section className="example" aria-label="Example">
            <pre className="code-block">
              <code>{EXAMPLE}</code>
            </pre>
          </section>
          <p className="landing-status">
            In development. Generation currently retrieves before prefill; integration during generation is planned.
          </p>
        </div>
      </main>
    </>
  );
}

