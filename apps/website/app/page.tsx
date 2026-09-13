import Link from "next/link";
import { CliAnimation } from "../components/CliAnimation";

export default function Home() {
  return (
    <main className="landing">
      <div className="landing-inner">
        <CliAnimation />

        <p className="landing-tagline">Inference engine for RAG</p>

        <div className="landing-actions">
          <code className="landing-install select-all">
            cargo install --git https://github.com/ashworks1706/piramid piramid --locked --features
            inference-candle,gpu-cuda
          </code>
          <a href="https://github.com/ashworks1706/piramid" className="landing-link">
            github
          </a>
          <Link href="/blogs" className="landing-link">
            blog
          </Link>
        </div>
      </div>
    </main>
  );
}
