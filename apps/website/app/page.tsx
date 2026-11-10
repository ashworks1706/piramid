import { CliLogo } from "../components/CliLogo";
import { InstallTabs } from "../components/InstallTabs";
import { SiteNav } from "../components/SiteNav";
import { DESCRIPTION, TAGLINE } from "../lib/site";

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
            {TAGLINE}
          </p>
          <p className="landing-copy">
            {DESCRIPTION}
          </p>
          <p className="landing-copy">
            Written in Rust, Piramid combines model execution, GPU memory and KV-cache management with durable vector storage and search.
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

