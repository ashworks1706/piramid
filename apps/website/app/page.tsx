import Link from "next/link";
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
            INFERENCE RUNTIME FOR RETRIEVAL SYSTEMS
          </p>
          <p className="landing-copy">
            Piramid is an inference engine for retrieval systems on one GPU. It
            holds the documents, the model weights and the KV cache in a single
            process, so retrieval can run during generation instead of once
            before it.
          </p>
          <InstallTabs />
          <section className="example" aria-label="Example">
            <pre className="code-block">
              <code>{EXAMPLE}</code>
            </pre>
          </section>
          <p className="landing-status">
            In active research. Results and write-ups will be published here.
          </p>
        </div>
      </main>
    </>
  );
}
