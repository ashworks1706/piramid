import Link from "next/link";
import { CliLogo } from "../components/CliLogo";
import { InstallTabs } from "../components/InstallTabs";
import { SiteNav } from "../components/SiteNav";

const FEATURES = [
  {
    title: "one process",
    body: "Documents, model weights and the KV cache live on one device. No vector database beside the model server.",
  },
  {
    title: "retrieval in generation",
    body: "One request embeds the question, runs an exact search with metadata filters, adds the passages and generates.",
  },
  {
    title: "plain HTTP",
    body: "A JSON API for collections, search and generation with streaming, plus OpenAI-compatible /v1 routes.",
  },
];

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
          <section className="features" aria-label="What it does">
            {FEATURES.map((feature) => (
              <div key={feature.title} className="feature">
                <h2>{feature.title}</h2>
                <p>{feature.body}</p>
              </div>
            ))}
          </section>
          <section className="example" aria-label="Example">
            <pre className="code-block">
              <code>{EXAMPLE}</code>
            </pre>
            <div className="landing-actions">
              <Link href="/docs" className="landing-link">
                read the docs
              </Link>
              <Link href="/docs/http-api" className="landing-link">
                http api
              </Link>
            </div>
          </section>
          <p className="landing-status">
            In active research. Results and write-ups will be published here.
          </p>
        </div>
      </main>
    </>
  );
}
