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
            Memory Native Inference
          </p>
          <p className="landing-copy">
            Piramid is an inference systems project investigating how language models can access and use external information directly during computation.
          </p>
          <p className="landing-copy">
            The long term goal is to move beyond prompt-based retrieval by investigating how external information can be represented, retained, and integrated into transformer computation.
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
